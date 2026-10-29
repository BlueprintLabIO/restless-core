#!/usr/bin/env python3
"""Verify owner CPU/RAM policy against actual disposable company containers.

No model connection is used. An optional empty temporary shared slice also exercises
admission behind a stricter pending limit. All company files, containers and databases
belong to this invocation and are removed, including on failure.
"""
from pathlib import Path
import argparse
import datetime, hashlib, json, os, secrets, signal, subprocess, sys, tempfile, time
repo = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane, docker
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument('--image', required=True)
parser.add_argument('--output', required=True, type=Path)
parser.add_argument('--hold-for-browser', action='store_true')
parser.add_argument('--with-browser', action='store_true', help='Also verify built cockpit Resources layouts')
parser.add_argument('--pool-parent')
parser.add_argument('--pool-root', type=Path)
args = parser.parse_args()
assert bool(args.pool_parent) == bool(args.pool_root), 'Supply both temporary pool arguments'
if args.pool_parent:
    assert args.pool_parent != 'restless-workloads.slice' and args.pool_parent.startswith('restless-')
    unit = subprocess.check_output(['systemctl', 'show', args.pool_parent, '--property=FragmentPath', '--value'], text=True).strip()
    assert unit == str(Path('/run/systemd/system') / args.pool_parent), 'Use an explicitly owned temporary test slice'
    assert args.pool_root.resolve() == Path('/sys/fs/cgroup/restless.slice') / args.pool_parent
    for path in args.pool_root.rglob('cgroup.procs'):
        assert not path.read_text().strip(), 'Another workload is using the test pool'
out = args.output
out.mkdir(parents=True, exist_ok=True)
assert not (out / 'finish').exists(), 'Choose a fresh evidence directory'

def wait(check, seconds=90):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        if check():
            return
        time.sleep(1)
    raise AssertionError('Timed out waiting for observed limits')

def stop(signum, frame):
    raise RuntimeError('Fixture interrupted')
signal.signal(signal.SIGTERM, stop)
signal.signal(signal.SIGALRM, stop)
signal.alarm(1800)
image_id = docker('image', 'inspect', '--format', '{{.Id}}', args.image).stdout.strip()
daemon = Path(os.environ.get('RESTLESSD_BIN', str(repo / 'target/debug/restlessd')))
with daemon.open('rb') as binary:
    daemon_hash = hashlib.file_digest(binary, 'sha256').hexdigest()
report = {'startedAt': datetime.datetime.now(datetime.timezone.utc).isoformat(), 'checks': [], 'sourceRevision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip(), 'workingTreeDirty': bool(subprocess.check_output(['git', 'status', '--porcelain'], cwd=repo, text=True).strip()), 'daemonSha256': daemon_hash, 'image': image_id}
plane = None
worker = None
with tempfile.TemporaryDirectory(prefix='s109-compute-controls-') as directory:
    root = Path(directory)
    (root / 'plane').mkdir()
    (root / 'home').mkdir()
    suffix = secrets.token_hex(3)
    company = f'caps_{suffix}_test'
    ns = f'caps_{suffix}_test'
    try:
        plane = Plane(root / 'plane', ns, os.environ.get('RESTLESSD_BIN', str(repo / 'target/debug/restlessd')), args.image, {'HOME': str(root / 'home'), 'RESTLESS_OMP_BIN': str(repo / 'infra/host-tools/omp-source'), 'RESTLESS_SOURCE_ROOT': str(repo), 'RESTLESS_COMPANY_CPUS': '1', 'RESTLESS_COMPANY_MEMORY': '2048m', 'RESTLESS_COMPUTER_MEMORY': '768m', 'RESTLESS_TEST_DISABLE_SCHEDULER': '1', 'RUST_LOG': 'info'})
        if args.pool_parent:
            plane.extra_env.update(RESTLESS_WORKLOAD_CGROUP_PARENT=args.pool_parent, RESTLESS_WORKLOAD_CGROUP_ROOT=str(args.pool_root))
        if args.with_browser or args.hold_for_browser:
            assert (repo / 'web/build/index.html').is_file(), 'Build the cockpit before browser verification'
            plane.extra_env['RESTLESS_COCKPIT_DIR'] = str(repo / 'web/build')
        plane.start_database()
        plane.start()
        plane.wait_for_port(plane.offset + 7788)
        plane.create_company(company, f'name = "{company}"\nauto_sleep_after_minutes = 0\n')
        plane.raw(company, {'cmd': 'up'}, timeout=600)

        def view():
            status, value = plane.owner(company, '/company/compute')
            assert status == 200
            return value['limits']

        def save(target, limits, expected, status=200):
            code, value = plane.owner(company, '/company/compute-limits/' + target, {'limits': limits, 'expected': expected})
            assert code == status, (code, value)
            return value

        def applied(target, cpu, memory):
            reading = view()[target]
            return reading['applied'] == {'cpu_millis': cpu, 'memory_mib': memory} and (not reading['pending'])
        assert view()['runtime']['saved'] is None
        save('runtime', {'cpu_millis': 2000, 'memory_mib': 768}, None, 400)
        save('runtime', {'cpu_millis': 500, 'memory_mib': 0}, None, 400)
        save('runtime', {'cpu_millis': 500, 'memory_mib': 512}, None, 400)
        save('unknown', None, None, 400)
        work = {'cpu_millis': 500, 'memory_mib': 1536}
        browser = {'cpu_millis': 200, 'memory_mib': 512}
        save('runtime', work, None)
        wait(lambda: applied('runtime', 500, 1536))
        save('runtime', {'cpu_millis': 600, 'memory_mib': 768}, None, 409)
        save('computer', browser, None)
        wait(lambda: applied('computer', 200, 512))
        assert view()['runtime']['saved'] == work
        report['checks'].append('live CPU/RAM edits; operator ceiling; zero and undersized memory rejection; stale edit; independent targets')
        container = plane.container(company)
        docker('exec', '-u', 'company', container, 'python3', '-c', "from pathlib import Path; Path('/company/compute-policy-proof.txt').write_text('preserved')")
        plane.stop()
        plane.start()
        plane.wait_for_port(plane.offset + 7788)
        assert view()['runtime']['saved'] == work and view()['computer']['saved'] == browser
        old_id = docker('inspect', '-f', '{{.Id}}', container).stdout.strip()
        plane.raw(company, {'cmd': 'down', 'destroy': False})
        docker('rm', f'restless-{ns}-pc-{company}', container)
        plane.raw(company, {'cmd': 'up'}, timeout=600)
        assert docker('inspect', '-f', '{{.Id}}', container).stdout.strip() != old_id
        wait(lambda: applied('runtime', 500, 1536) and applied('computer', 200, 512))
        assert docker('exec', container, 'cat', '/company/compute-policy-proof.txt').stdout == 'preserved'
        report['checks'].append('daemon restart and container recreation retain both limits and company files')
        pid = int(docker('inspect', '-f', '{{.State.Pid}}', container).stdout)
        cg = None
        proc = Path(f'/proc/{pid}/cgroup')
        if proc.is_file():
            relative = next((line.split('::', 1)[1] for line in proc.read_text().splitlines() if '::' in line), None)
            if relative:
                candidate = (Path('/sys/fs/cgroup') / relative.lstrip('/')).resolve()
                assert candidate.is_relative_to('/sys/fs/cgroup')
                if (candidate / 'memory.events').is_file():
                    cg = candidate

        def events():
            return {k: int(v) for k, v in (line.split() for line in (cg / 'memory.events').read_text().splitlines())} if cg else None
        before = events()
        code = "import os,time,pathlib; data=bytearray(680*1024*1024); data[::4096]=bytes([1])*len(data[::4096]); pathlib.Path('/tmp/compute-caps-held.pid').write_text(str(os.getpid())); time.sleep(180)"
        worker = subprocess.Popen(['docker', 'exec', '-u', 'company', container, 'timeout', '190s', 'python3', '-u', '-c', code], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        wait(lambda: docker('exec', container, 'test', '-f', '/tmp/compute-caps-held.pid', check=False).returncode == 0)
        small = {'cpu_millis': 500, 'memory_mib': 768}
        before_admission = plane.owner(company, '/company/compute')[1]
        if args.pool_parent:
            assert not before_admission['actor_capacity']['waiting_for_memory'], before_admission
        result = save('runtime', small, work)
        if args.pool_parent:
            assert plane.owner(company, '/company/compute')[1]['actor_capacity']['waiting_for_memory']
        assert result['runtime']['pending'] and result['runtime']['applied']['memory_mib'] == 1536
        assert worker.poll() is None
        docker('exec', '-u', 'company', container, 'python3', '-c', "import os,pathlib; os.kill(int(pathlib.Path('/tmp/compute-caps-held.pid').read_text()),15)")
        worker.wait(timeout=10)
        worker = None
        wait(lambda: applied('runtime', 500, 768), 80)
        after = events()
        if before is not None:
            assert after['oom_kill'] == before['oom_kill']
        if args.pool_parent:
            assert not plane.owner(company, '/company/compute')[1]['actor_capacity']['waiting_for_memory']
        report['pendingLimitAdmission'] = 'allows, waits, resumes' if args.pool_parent else 'not exercised without shared pool'
        report['checks'].append('unsafe live RAM shrink remains pending; maintenance applies after usage falls')
        report['memoryEventDelta'] = {key: after[key] - before[key] for key in after} if before is not None else None
        save('runtime', None, small)
        wait(lambda: applied('runtime', 1000, 2048))
        assert view()['computer']['saved'] == browser
        save('computer', None, browser)
        wait(lambda: applied('computer', 1000, 768))
        report['checks'].append('reset restores operator defaults without erasing the other target')
        report['afterBackendChecks'] = view()
        ready = {'origin': plane.owner_origin, 'company': company, 'namespace': ns, 'root': str(root), 'pid': os.getpid()}
        (out / 'ready.json').write_text(json.dumps(ready))
        print(json.dumps({'phase': 'ready-for-browser' if args.hold_for_browser else 'checks-complete', 'origin': plane.owner_origin, 'company': company}), flush=True)
        report['sharedPool'] = args.pool_parent
        if args.with_browser:
            env = {key: os.environ[key] for key in ('PATH', 'HOME', 'LANG') if key in os.environ}
            env.update(RESTLESS_SMOKE_ORIGIN=plane.owner_origin, RESTLESS_SMOKE_COMPANY=company, RESTLESS_VERIFY_OUTPUT=str(out / 'browser'))
            subprocess.run(['node', 'scripts/verify-resources.mjs'], cwd=repo / 'web', env=env, check=True, timeout=180)
            report['browser'] = 'desktop/phone editor and reading/pending/error layout checks passed'
        if args.hold_for_browser:
            wait(lambda: (out / 'finish').exists(), 900)
        report['finalLimits'] = view()
        report['status'] = 'pass'
    except BaseException as error:
        report['status'] = 'fail'
        report['error'] = str(error)
        raise
    finally:
        if worker and worker.poll() is None:
            worker.terminate()
            worker.wait(timeout=10)
        if plane:
            if plane.log_path.exists():
                (out / 'daemon.log').write_text(plane.log_path.read_text())
            report['cleanupProblems'] = plane.close()
            if report['cleanupProblems']:
                report['status'] = 'fail'
        report['finishedAt'] = datetime.datetime.now(datetime.timezone.utc).isoformat()
        (out / 'evidence.json').write_text(json.dumps(report, indent=2))
        print(json.dumps(report), flush=True)
        for item in root.rglob('*'):
            try:
                item.chmod(448 if item.is_dir() else 384)
            except OSError:
                pass
assert report['status'] == 'pass'
