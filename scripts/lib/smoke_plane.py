"""An isolated Core plane for end-to-end smokes.

One throwaway PostgreSQL container, one `restlessd` with its own state root,
namespace and port block, and helpers for the three ways a smoke talks to it:
the local control socket, the owner HTTP API, and capabilities minted with the
installation key exactly as the daemon mints them. Everything it creates carries
the `_test` suffix and is removed by `close()`, including after a failure.
"""
import base64, datetime, hashlib, hmac, json, os, pathlib, secrets, signal, socket, subprocess, time
import urllib.error, urllib.request


def docker(*args, check=True, **kwargs):
    kwargs.setdefault('capture_output', True)
    kwargs.setdefault('text', True)
    return subprocess.run(['docker', *args], check=check, **kwargs)


def b64(value):
    return base64.urlsafe_b64encode(value).rstrip(b'=').decode()


def reserve_offset(start=20000, stop=24900):
    """A block of ports the daemon derives from its offset, all free now.

    The test profile requires an offset of at least 20000, and the block stays
    below 32768: Linux hands out ephemeral ports from there up, so an outgoing
    connection could take a daemon port between this check and the bind."""
    for offset in range(start, stop, 100):
        held = []
        try:
            for base in range(7788, 7800):
                probe = socket.socket()
                probe.bind(('127.0.0.1', offset + base))
                held.append(probe)
            return offset
        except OSError:
            pass
        finally:
            for probe in held:
                probe.close()
    raise RuntimeError('no isolated test port range')


def free_port():
    with socket.socket() as probe:
        probe.bind(('127.0.0.1', 0))
        return probe.getsockname()[1]


class Plane:
    def __init__(self, root, namespace, daemon_bin, image, extra_env=None):
        if not namespace.endswith('_test'):
            raise ValueError('a smoke namespace must end in _test')
        self.root = pathlib.Path(root)
        self.namespace = namespace
        self.daemon_bin = daemon_bin
        self.image = image
        self.extra_env = dict(extra_env or {})
        self.offset = reserve_offset()
        self.owner_origin = f'http://127.0.0.1:{self.offset + 7788}'
        self.relay_port = self.offset + 7790
        self.coordinator = f'127.0.0.1:{self.offset + 7791}'
        self.pg_container = f'restless-{namespace}-pg'
        self.pg_password = secrets.token_hex(16)
        self.process = None
        self.companies = []
        self.log_path = self.root / 'daemon.log'

    # -- PostgreSQL ---------------------------------------------------------
    def start_database(self):
        docker('run', '-d', '--name', self.pg_container, '--label', f'io.restless.namespace={self.namespace}',
               '-e', 'POSTGRES_USER=restless', '-e', f'POSTGRES_PASSWORD={self.pg_password}',
               '-e', 'POSTGRES_DB=restless', '-p', '127.0.0.1::5432', 'postgres:17-alpine')
        port = docker('port', self.pg_container, '5432/tcp').stdout.strip().splitlines()[0].rsplit(':', 1)[1]
        # Over TCP: the image's first-boot init server listens on the unix
        # socket only, so a socket check reports ready before it restarts.
        for _ in range(120):
            if docker('exec', self.pg_container, 'pg_isready', '-h', '127.0.0.1', '-U', 'restless', check=False).returncode == 0:
                break
            time.sleep(0.5)
        else:
            raise RuntimeError('test PostgreSQL did not become ready')
        self.admin_url = f'postgres://restless:{self.pg_password}@127.0.0.1:{port}/restless'
        (self.root / 'orgintel.toml').write_text('database_url = ' + json.dumps(self.admin_url) + '\n')
        (self.root / 'orgintel.toml').chmod(0o600)

    def sql(self, command, database='restless'):
        return docker('exec', self.pg_container, 'psql', '-v', 'ON_ERROR_STOP=1', '-U', 'restless',
                      '-d', database, '-Atc', command).stdout.strip()

    # -- the daemon ---------------------------------------------------------
    def env(self):
        env = {key: os.environ[key] for key in ['PATH', 'HOME', 'USER', 'LANG'] if key in os.environ}
        # The owner gateway refuses to start without cockpit assets; API smokes
        # need none, so they get a stub page rather than a web build.
        cockpit = self.root / 'cockpit'
        if not cockpit.exists():
            cockpit.mkdir()
            (cockpit / 'index.html').write_text('<!doctype html><title>smoke</title>\n')
        env.update(RESTLESS_COCKPIT_DIR=str(cockpit))
        env.update(RESTLESS_HOME=str(self.root), RESTLESS_PROFILE='test', RESTLESS_RESOURCE_NAMESPACE=self.namespace,
                   RESTLESS_PORT_OFFSET=str(self.offset), RESTLESS_COMPANY_IMAGE=self.image,
                   RESTLESS_TEST_DISABLE_SCHEDULER='1', RESTLESS_OWNER_ADDR=f'127.0.0.1:{self.offset + 7788}')
        env.update(self.extra_env)
        return env

    def start(self):
        log = self.log_path.open('a')
        self.process = subprocess.Popen([self.daemon_bin], cwd=self.root, env=self.env(), stdout=log,
                                        stderr=subprocess.STDOUT, start_new_session=True)
        for _ in range(240):
            if self.process.poll() is not None:
                raise RuntimeError('test daemon exited:\n' + self.log_path.read_text()[-3000:])
            try:
                self.raw(None, {'cmd': 'company-list'})
                return
            except (OSError, RuntimeError):
                time.sleep(0.25)
        raise RuntimeError('test plane did not start')

    def stop(self):
        if self.process is not None and self.process.poll() is None:
            os.killpg(self.process.pid, signal.SIGTERM)
            try:
                self.process.wait(timeout=20)
            except subprocess.TimeoutExpired:
                os.killpg(self.process.pid, signal.SIGKILL)
                self.process.wait()
        self.process = None

    def wait_for_port(self, port, seconds=90):
        for _ in range(seconds * 4):
            with socket.socket() as probe:
                probe.settimeout(0.25)
                if probe.connect_ex(('127.0.0.1', port)) == 0:
                    return
            if self.process is not None and self.process.poll() is not None:
                raise RuntimeError('test daemon exited:\n' + self.log_path.read_text()[-3000:])
            time.sleep(0.25)
        raise RuntimeError(f'nothing listened on {port} within {seconds}s:\n' + self.log_path.read_text()[-3000:])

    # -- talking to it ------------------------------------------------------
    def raw(self, company, payload, timeout=120):
        with socket.socket(socket.AF_UNIX) as connection:
            connection.settimeout(timeout)
            connection.connect(str(self.root / 'restlessd.sock'))
            body = {'principal': 'owner', **payload}
            if company:
                body['company'] = company
            connection.sendall((json.dumps(body) + '\n').encode())
            response = json.loads(connection.makefile('rb').readline())
            if not response['ok']:
                raise RuntimeError(response['error']['message'])
            return response.get('data')

    def create_company(self, name, body):
        if not name.endswith('_test'):
            raise ValueError('a smoke company must end in _test')
        self.companies.append(name)  # first, so a half-made company is still destroyed
        self.raw(name, {'cmd': 'company-create', 'body': body})

    def container(self, company):
        return f'restless-{self.namespace}-co-{company}'

    def volume(self, company):
        return f'restless-{self.namespace}-vol-{company}'

    def owner(self, company, path, data=None, method=None, origin=True):
        headers = {'Origin': self.owner_origin} if origin else {}
        body = None
        if data is not None:
            body = json.dumps(data).encode()
            headers['Content-Type'] = 'application/json'
        url = f'{self.owner_origin}/api/companies/{company}{path}'
        request = urllib.request.Request(url, data=body, headers=headers, method=method or ('POST' if data is not None else 'GET'))
        try:
            with urllib.request.urlopen(request, timeout=120) as response:
                return response.status, json.load(response)
        except urllib.error.HTTPError as error:
            raw = error.read() or b'{}'
            try:
                return error.code, json.loads(raw)
            except ValueError:
                return error.code, {'raw': raw.decode(errors='replace')}

    def capability(self, claims, minutes=30):
        """A capability signed with this installation's key, in the daemon's
        own `r1.<claims>.<mac>` encoding."""
        claims = {'version': 1, **claims,
                  'expires_at': (datetime.datetime.now(datetime.timezone.utc) + datetime.timedelta(minutes=minutes)).isoformat()}
        signed = 'r1.' + b64(json.dumps(claims, separators=(',', ':')).encode())
        mac = hmac.new((self.root / 'runtime-capability.key').read_bytes(), signed.encode(), hashlib.sha256).digest()
        return signed + '.' + b64(mac)

    def tool_session(self, company, actor, session='smoke'):
        return self.capability({'kind': 'tool_session', 'company': company, 'actor': actor,
                                'session': f'tools-{session}-{secrets.token_hex(4)}'})

    # -- teardown -----------------------------------------------------------
    def close(self):
        """Remove everything this plane created. Returns the problems left."""
        problems = []
        for company in reversed(self.companies):
            if self.process is not None and self.process.poll() is None:
                try:
                    self.raw(company, {'cmd': 'down', 'destroy': True}, timeout=180)
                except Exception as error:  # noqa: BLE001 - every leak is reported below
                    problems.append(f'down {company}: {error}')
        self.stop()
        for company in self.companies:
            docker('rm', '-f', self.container(company), check=False)
            docker('volume', 'rm', self.volume(company), check=False)
            docker('volume', 'rm', f'restless-{self.namespace}-mcp-cache-{company}', check=False)
        docker('rm', '-f', self.pg_container, check=False)
        leftover = docker('ps', '-a', '--filter', f'name={self.namespace}', '--format', '{{.Names}}', check=False).stdout.split()
        leftover += [name for company in self.companies
                     for name in docker('ps', '-a', '--filter', f'name={company}', '--format', '{{.Names}}', check=False).stdout.split()]
        volumes = docker('volume', 'ls', '-q', '--filter', f'name={self.namespace}', check=False).stdout.split()
        volumes += [name for company in self.companies
                    for name in docker('volume', 'ls', '-q', '--filter', f'name={company}', check=False).stdout.split()]
        if leftover:
            problems.append(f'containers remain: {sorted(set(leftover))}')
        if volumes:
            problems.append(f'volumes remain: {sorted(set(volumes))}')
        return problems
