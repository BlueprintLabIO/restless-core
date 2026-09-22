#!/usr/bin/env python3
"""Exercise new-profile database provisioning against real Docker/PostgreSQL."""
import os
from pathlib import Path
import socket
import stat
import subprocess
import tempfile
import tomllib
import uuid

ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.run(args, check=True, text=True, capture_output=True, **kwargs)


with tempfile.TemporaryDirectory(prefix='restless-first-install-') as directory:
    state = Path(directory) / 'state'
    namespace = 'launch' + uuid.uuid4().hex[:10] + '_test'
    name = f'restless-{namespace}-postgres'
    with socket.socket() as port_reservation:
        port_reservation.bind(('127.0.0.1', 0))
        port = port_reservation.getsockname()[1]
    env = dict(os.environ, RESTLESS_HOME=str(state), RESTLESS_RESOURCE_NAMESPACE=namespace,
               RESTLESS_PORT_OFFSET=str(port - 7797))
    env.pop('RESTLESS_PLANE_DATABASE_URL', None)

    def provision():
        return run('bash', '-c', 'source "$1"; dev_database_ensure', 'test',
                   str(ROOT / 'scripts/lib/dev-database.sh'), env=env)

    try:
        provision()
        config = (state / 'orgintel.toml').read_bytes()
        assert tomllib.loads(config.decode())['database_url'].startswith('postgres://restless:')
        for filename in ('orgintel.toml', 'postgres.env'):
            assert stat.S_IMODE((state / filename).stat().st_mode) == 0o600
        run('docker', 'exec', name, 'psql', '-U', 'restless', '-d', 'restless',
            '-v', 'ON_ERROR_STOP=1', '-c', 'CREATE TABLE first_install(value text); INSERT INTO first_install VALUES (\'preserved\');')
        run('docker', 'stop', '--time', '5', name)
        provision()
        assert (state / 'orgintel.toml').read_bytes() == config
        result = run('docker', 'exec', name, 'psql', '-U', 'restless', '-d', 'restless',
                     '-Atc', 'SELECT value FROM first_install')
        assert result.stdout.strip() == 'preserved'
        print('PASS fresh database, private credentials, restart and preserved data')
        external = Path(directory) / 'external'
        external.mkdir()
        saved = 'database_url = "postgres://existing.invalid/company"\n'
        (external / 'orgintel.toml').write_text(saved)
        env['RESTLESS_HOME'] = str(external)
        provision()
        assert (external / 'orgintel.toml').read_text() == saved
        assert not (external / 'postgres.env').exists()
        print('PASS existing external database configuration is preserved')
    finally:
        subprocess.run(['docker', 'rm', '-f', name], capture_output=True, check=False)
        subprocess.run(['docker', 'volume', 'rm', f'{name}-data'], capture_output=True, check=False)
        assert subprocess.run(['docker', 'container', 'inspect', name], capture_output=True).returncode != 0
        assert subprocess.run(['docker', 'volume', 'inspect', f'{name}-data'], capture_output=True).returncode != 0
        print('PASS disposable database container and volume removed')
