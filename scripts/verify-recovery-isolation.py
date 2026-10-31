"""Disposable two-company restart smoke for orphan recovery admission."""
import json, os, secrets, signal, subprocess, sys, tempfile, time
from pathlib import Path
repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane

def interrupt(*_):
    raise RuntimeError('fixture interrupted')
signal.signal(signal.SIGTERM, interrupt)
signal.signal(signal.SIGALRM, interrupt)
signal.alarm(900)

def run(*args, check=True):
    return subprocess.run(args, check=check, text=True, capture_output=True, timeout=120)

def until(predicate, message, seconds=120):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate(): return
        time.sleep(0.5)
    raise RuntimeError(message)

suffix = secrets.token_hex(3)
namespace = 'orphan_' + suffix + '_test'
a, b = 'held_' + suffix + '_test', 'other_' + suffix + '_test'
with tempfile.TemporaryDirectory(prefix='s109-recovery-') as folder:
    root = Path(folder)
    plane = Plane(root, namespace, os.environ['RESTLESSD_BIN'],
                  os.environ.get('RESTLESS_COMPANY_IMAGE', 'restless-company-image:release-connections'),
                  {'RESTLESS_OMP_BIN': str(repo / 'infra/host-tools/omp-source'),
                   'RESTLESS_SOURCE_ROOT': str(repo), 'RESTLESS_TEST_DISABLE_SCHEDULER': '0',
                   'RESTLESS_COMPANY_CPUS': '1', 'RESTLESS_COMPANY_MEMORY': '1024m',
                   'RESTLESS_COMPUTER_MEMORY': '512m'})
    jobs = []
    try:
        plane.start_database(); plane.start(); plane.wait_for_port(plane.offset + 7788)
        for company in (a, b):
            plane.create_company(company, 'name = ' + json.dumps(company) + '\nauto_sleep_after_minutes = 0\n')
            plane.raw(company, {'cmd': 'up'}, timeout=300)
        plane.stop()
        bad = '/tmp/restless-agent-' + suffix + '.sid'
        run('docker', 'exec', plane.container(a), 'sh', '-c', 'printf invalid > "$1"', 'fixture', bad)
        log_offset = plane.log_path.stat().st_size
        plane.start(); plane.wait_for_port(plane.offset + 7788)
        def recovered():
            log = plane.log_path.read_text()[log_offset:]
            return 'could not verify orphan agent cleanup' in log and 'runtime orphan recovery complete' in log
        until(recovered, 'restart did not observe cleanup failure')
        for company in (a, b):
            status, _ = plane.owner(company, '/principal')
            assert status == 200, (company, status)
            marker = '/company/run/recovery-fixture-' + suffix
            output = run('docker', 'exec', '-u', 'company', plane.container(company),
                         'restless', 'job', 'run', '--cpus', '0.2', '--memory', '128m',
                         '--timeout', '30', '--', 'printf x >> ' + marker).stdout
            job_id = json.loads(output)['job']
            jobs.append((company, job_id, marker))
        def effect(company, marker):
            return run('docker', 'exec', '-u', 'company', plane.container(company), 'cat', marker, check=False)
        until(lambda: effect(b, jobs[1][2]).stdout == 'x', 'other company job did not run')
        time.sleep(7)
        assert effect(a, jobs[0][2]).returncode != 0, 'held company ran before cleanup recovered'
        run('docker', 'exec', plane.container(a), 'rm', '-f', bad)
        until(lambda: effect(a, jobs[0][2]).stdout == 'x', 'held company did not resume')
        time.sleep(6)
        assert all(effect(company, marker).stdout == 'x' for company, _, marker in jobs)
        print('PASS: cleanup failure held one company; other company ran; held job resumed once', flush=True)
    finally:
        for company, job_id, _ in jobs:
            run('docker', 'rm', '-f', plane.container(company) + '-job-' + job_id, check=False)
        problems = plane.close()
        print('cleanup:', json.dumps(problems), flush=True)
        if problems: raise RuntimeError('fixture cleanup failed')
