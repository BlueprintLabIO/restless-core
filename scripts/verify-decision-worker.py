"""Exercise decision deadlines, Undo, and restart using an isolated real plane.

Set RESTLESSD_BIN to the daemon to verify. Requires the identity service's pg
package and a locally available company image. No provider or model calls are made.
"""
import hashlib
import json
import os
import secrets
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path

repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane  # noqa: E402

output = Path(os.environ.get('RESTLESS_VERIFY_OUTPUT', '/tmp/restless-decision-worker'))
output.mkdir(parents=True, exist_ok=True)


def interrupted(*_):
    raise RuntimeError('decision worker probe interrupted')


signal.signal(signal.SIGTERM, interrupted)
signal.signal(signal.SIGALRM, interrupted)
signal.alarm(300)
with tempfile.TemporaryDirectory(prefix='restless-decision-worker-') as folder:
    root = Path(folder)
    (root / 'home').mkdir()
    plane_root = root / '.restless-dev' / 'plane'
    plane_root.mkdir(parents=True)
    namespace = 'decision_' + secrets.token_hex(3) + '_test'
    company = namespace[:-5] + '_a_test'
    plane = Plane(
        plane_root, namespace, os.environ['RESTLESSD_BIN'],
        os.environ.get('RESTLESS_COMPANY_IMAGE', 'restless-company-image:release-connections'),
        {'HOME': str(root / 'home')},
    )
    report = {
        'scenarios': [],
        'daemonSha256': hashlib.sha256(Path(plane.daemon_bin).read_bytes()).hexdigest(),
    }
    try:
        plane.start_database()
        plane.start()
        plane.wait_for_port(plane.offset + 7788)
        plane.create_company(company, f'name = "{company}"\nauto_sleep_after_minutes = 0\n')
        for kind, script in [
            ('handoff', 'verify-inbox-undo-restart.mjs'),
            ('document', 'verify-document-undo-restart.mjs'),
        ]:
            record_path = root / (kind + '.json')
            env = os.environ.copy()
            env.update(
                RESTLESS_SMOKE_ORIGIN=plane.owner_origin,
                RESTLESS_SMOKE_COMPANY=company,
                RESTLESS_HOME=str(plane.root),
                RESTLESS_RESTART_RECORD=str(record_path),
            )

            def run(mode):
                result = subprocess.run(
                    ['node', str(repo / 'web/scripts' / script), mode],
                    env=env, text=True, capture_output=True, timeout=45,
                )
                if result.returncode:
                    raise RuntimeError(kind + ' ' + mode + ': ' + result.stderr[-2000:])
                print(kind, mode, result.stdout.strip(), flush=True)

            run('queue')
            plane.stop()
            time.sleep(6)
            run('paused')
            plane.start()
            plane.wait_for_port(plane.offset + 7788)
            run('verify')
            report['scenarios'].append(kind + ' restart preserves deadline and applies once')

            run('queue')
            run('verify')
            report['scenarios'].append(kind + ' normal deadline applies once')

            run('queue')
            record = json.loads(record_path.read_text())
            path = '/handoff-decisions' if kind == 'handoff' else '/document-decisions'
            status, canceled = plane.owner(company, path + '/' + record['id'], method='DELETE')
            assert status == 200 and canceled['state'] == 'canceled', (status, canceled)
            time.sleep(6)
            status, results = plane.owner(company, path)
            assert status == 200
            match = [item for item in results['items'] if item['id'] == record['id']]
            assert len(match) == 1 and match[0]['state'] == 'canceled'
            databases = plane.sql(
                "SELECT datname FROM pg_database WHERE datname LIKE '%" + company + "%'"
            ).splitlines()
            assert len(databases) == 1
            if kind == 'handoff':
                table, column, identifier, expected = (
                    'owner_handoffs', 'state', record['handoff_id'], 'pending'
                )
            else:
                table, column, identifier, expected = (
                    'native_documents', 'status', record['document'], 'in_review'
                )
            actual = plane.sql(
                'SET search_path TO "' + company + '"; SELECT ' + column +
                "::text FROM " + table + " WHERE id='" + identifier + "'",
                database=databases[0],
            )
            assert actual.splitlines()[-1] == expected, (kind, actual)
            report['scenarios'].append(kind + ' Undo keeps source unchanged after deadline')
        report['status'] = 'pass'
    finally:
        report['cleanupProblems'] = plane.close()
        (output / 'evidence.json').write_text(json.dumps(report, indent=2))
        for item in root.rglob('*'):
            try:
                item.chmod(0o700 if item.is_dir() else 0o600)
            except OSError:
                pass
        assert not report['cleanupProblems']
