"""Measure idle decision SQL in two disposable companies without model calls.

Set RESTLESSD_BIN to the daemon under test and RESTLESS_VERIFY_OUTPUT to a
private output directory. Only statement counts, binary identity, and cleanup
status are recorded; database logs remain inside the disposable container.
"""
import datetime
import hashlib
import json
import os
import secrets
import signal
import sys
import tempfile
import time
from pathlib import Path

repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane, docker  # noqa: E402

output = Path(os.environ.get('RESTLESS_VERIFY_OUTPUT', '/tmp/restless-decision-idle'))
output.mkdir(parents=True, exist_ok=True)
binary = Path(os.environ['RESTLESSD_BIN'])


def interrupted(*_):
    raise RuntimeError('idle decision probe interrupted')


signal.signal(signal.SIGTERM, interrupted)
signal.signal(signal.SIGALRM, interrupted)
signal.alarm(180)
with tempfile.TemporaryDirectory(prefix='restless-decision-idle-') as folder:
    root = Path(folder)
    (root / 'home').mkdir()
    (root / 'plane').mkdir()
    namespace = 'decision_' + secrets.token_hex(3) + '_test'
    plane = Plane(
        root / 'plane', namespace, str(binary),
        os.environ.get('RESTLESS_COMPANY_IMAGE', 'restless-company-image:release-connections'),
        {'HOME': str(root / 'home')},
    )
    report = {'daemonSha256': hashlib.sha256(binary.read_bytes()).hexdigest()}
    try:
        plane.start_database()
        plane.start()
        plane.wait_for_port(plane.offset + 7788)
        for suffix in ['a', 'b']:
            company = namespace[:-5] + '_' + suffix + '_test'
            plane.create_company(company, f'name = "{company}"\nauto_sleep_after_minutes = 0\n')
        plane.sql("ALTER SYSTEM SET log_statement='all'")
        plane.sql('SELECT pg_reload_conf()')
        # Allow several complete repair sweeps after both companies exist.
        time.sleep(6)
        start = datetime.datetime.now(datetime.timezone.utc).isoformat()
        for _ in range(40):
            lines = docker('logs', '--since', start, plane.pg_container).stderr
            if lines.count('SELECT GREATEST(0, EXTRACT(EPOCH FROM') >= 2:
                break
            time.sleep(0.5)
        else:
            raise RuntimeError('both idle company deadline reads did not start')
        start = datetime.datetime.now(datetime.timezone.utc).isoformat()
        time.sleep(12)
        lines = docker('logs', '--since', start, plane.pg_container).stderr
        report.update(
            companies=2, observationSeconds=12,
            handoffDueQueries=lines.count('SELECT * FROM owner_handoff_decisions WHERE state='),
            documentDueQueries=lines.count('SELECT * FROM owner_document_decisions WHERE state='),
            deadlineQueries=lines.count('SELECT GREATEST(0, EXTRACT(EPOCH FROM'),
        )
        if report['handoffDueQueries'] or report['documentDueQueries']:
            raise AssertionError('quiet companies must not scan either due queue')
        if report['deadlineQueries'] < 2:
            raise AssertionError('the repair sweep must still read the deadline')
        report['status'] = 'pass'
        print(json.dumps(report), flush=True)
    finally:
        report['cleanupProblems'] = plane.close()
        (output / 'evidence.json').write_text(json.dumps(report, indent=2))
        for item in root.rglob('*'):
            try:
                item.chmod(0o700 if item.is_dir() else 0o600)
            except OSError:
                pass
        assert not report['cleanupProblems']
