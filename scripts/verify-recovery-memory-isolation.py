"""Exercise restart recovery under one company's real cgroup memory pressure."""
import json, os, secrets, signal, subprocess, sys, tempfile, time
from pathlib import Path
repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane

def run(*args, check=True):
    return subprocess.run(args, check=check, text=True, capture_output=True, timeout=120)
def interrupt(*_):
    raise RuntimeError('fixture interrupted')
signal.signal(signal.SIGTERM, interrupt)
signal.signal(signal.SIGALRM, interrupt)
signal.alarm(900)
def until(predicate, message, seconds=120):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate(): return
        time.sleep(.5)
    raise RuntimeError(message)

nonce = secrets.token_hex(3)
namespace = 'memrec_' + nonce + '_test'
a, b = 'memheld_' + nonce + '_test', 'memother_' + nonce + '_test'
slice_name = 'restless-s109memrec' + nonce + '.slice'
unit = Path('/run/systemd/system') / slice_name
pool = Path('/sys/fs/cgroup/restless.slice') / slice_name
assert not unit.exists(), 'refusing to alter an existing slice'
with tempfile.TemporaryDirectory(prefix='s109-memrec-') as folder:
    root = Path(folder)
    for name in ('home', 'plane', 'bin'): (root / name).mkdir()
    (root / 'bin/getconf').write_text('#!/bin/sh\necho 4\n')
    (root / 'bin/getconf').chmod(0o755)
    (root / 'meminfo').write_text('MemTotal: 8388608 kB\n')
    source = (Path(os.environ['RESTLESS_CLOUD_DIR']) / 'services/vm-provider/agent/restless-agent.sh').read_text()
    start, end = source.index('install_workload_budget() {'), source.index('\nupdate_agent()')
    setup = root / 'setup.sh'
    setup.write_text('log(){ :; }\n' + source[start:end] + '\ninstall_workload_budget ' + slice_name + '\n')
    path = str(root / 'bin') + ':' + os.environ['PATH']
    plane = None; jobs = []; scope = None; owned_unit = False
    try:
        owned_unit = True
        run('sudo', '-n', 'env', 'PATH=' + path, 'RESTLESS_HOST_MEMINFO=' + str(root / 'meminfo'),
            'RESTLESS_WORKLOAD_UNIT_DIR=/run/systemd/system', 'bash', str(setup))
        plane = Plane(root / 'plane', namespace, os.environ['RESTLESSD_BIN'],
                      os.environ.get('RESTLESS_COMPANY_IMAGE', 'restless-company-image:release-connections'),
                      {'HOME': str(root / 'home'), 'PATH': path,
                       'RESTLESS_OMP_BIN': str(repo / 'infra/host-tools/omp-source'),
                       'RESTLESS_SOURCE_ROOT': str(repo), 'RESTLESS_TEST_DISABLE_SCHEDULER': '0',
                       'RESTLESS_COMPANY_CPUS': '1', 'RESTLESS_COMPANY_MEMORY': '1024m',
                       'RESTLESS_COMPUTER_MEMORY': '512m', 'RESTLESS_WORKLOAD_CGROUP_PARENT': slice_name,
                       'RESTLESS_WORKLOAD_CGROUP_ROOT': str(pool)})
        plane.start_database(); plane.start(); plane.wait_for_port(plane.offset + 7788)
        for company in (a, b):
            plane.create_company(company, 'name = ' + json.dumps(company) + '\nauto_sleep_after_minutes = 0\n')
            plane.raw(company, {'cmd': 'up'}, timeout=300)
        plane.stop()
        container = plane.container(a)
        cid = run('docker', 'inspect', '-f', '{{.Id}}', container).stdout.strip()
        assert len(cid) == 64 and all(c in '0123456789abcdef' for c in cid)
        scope = pool / ('docker-' + cid + '.scope')
        assert scope.is_dir() and scope.parent == pool
        # Force a real, bounded per-company working set over its soft threshold,
        # while staying below the company hard limit and the shared pool limit.
        run('sudo', '-n', 'sh', '-c', 'printf %s 536870912 > "$1"', 'fixture', str(scope / 'memory.high'))
        run('docker', 'exec', '-d', container, 'python3', '-c',
            'import time; x=bytearray(340*1024*1024); open("/tmp/memrec-ready","w").write("ready"); time.sleep(600)')
        until(lambda: run('docker', 'exec', container, 'test', '-e', '/tmp/memrec-ready', check=False).returncode == 0,
              'memory load did not start')
        bad = '/tmp/restless-agent-' + nonce + '.sid'
        run('docker', 'exec', container, 'sh', '-c', 'printf invalid > "$1"', 'fixture', bad)
        log_offset = plane.log_path.stat().st_size
        plane.start(); plane.wait_for_port(plane.offset + 7788)
        def reading(company):
            status, value = plane.owner(company, '/company/compute')
            return value['sample']['work_admission'] if status == 200 else {}
        until(lambda: reading(a).get('waiting_for_memory') and reading(a).get('scope') == 'company',
              'company memory pressure was not observed')
        assert not reading(b).get('waiting_for_memory'), 'shared pool or healthy company is under pressure'
        for company in (a, b):
            status, _ = plane.owner(company, '/principal'); assert status == 200
            marker = '/company/run/memrec-fixture-' + nonce
            result = run('docker', 'exec', '-u', 'company', plane.container(company), 'restless', 'job', 'run',
                         '--cpus', '0.2', '--memory', '128m', '--timeout', '30', '--', 'printf x >> ' + marker)
            job_id = json.loads(result.stdout)['job']; jobs.append((company, job_id, marker))
        def effect(company, marker):
            return run('docker', 'exec', '-u', 'company', plane.container(company), 'cat', marker, check=False)
        until(lambda: effect(b, jobs[1][2]).stdout == 'x', 'other company did not run')
        time.sleep(7)
        assert effect(a, jobs[0][2]).returncode != 0
        assert 'could not verify orphan agent cleanup' not in plane.log_path.read_text()[log_offset:]
        # Release only this synthetic load; malformed marker must still hold recovery.
        run('docker', 'exec', container, 'pkill', '-f', '^python3 -c import time; x=bytearray', check=False)
        run('sudo', '-n', 'sh', '-c', 'printf max > "$1"', 'fixture', str(scope / 'memory.high'))
        until(lambda: not reading(a).get('waiting_for_memory'), 'memory pressure did not clear')
        until(lambda: 'could not verify orphan agent cleanup' in plane.log_path.read_text()[log_offset:],
              'recovery did not retry after pressure cleared')
        assert effect(a, jobs[0][2]).returncode != 0
        run('docker', 'exec', container, 'rm', '-f', bad)
        until(lambda: effect(a, jobs[0][2]).stdout == 'x', 'held company did not resume')
        time.sleep(6)
        assert all(effect(company, marker).stdout == 'x' for company, _, marker in jobs)
        print('PASS: real company memory pressure delayed its recovery while its peer ran; resumed once', flush=True)
    finally:
        problems = []
        if scope and scope.exists():
            try: run('sudo', '-n', 'sh', '-c', 'printf max > "$1"', 'fixture', str(scope / 'memory.high'))
            except Exception as error: problems.append('restore memory.high: ' + str(error))
        for company, job_id, _ in jobs:
            run('docker', 'rm', '-f', plane.container(company) + '-job-' + job_id, check=False)
        if plane:
            try: problems.extend(plane.close())
            except Exception as error: problems.append(str(error))
        if owned_unit and unit.exists():
            if pool.exists() and any(p.read_text().strip() for p in pool.rglob('cgroup.procs')):
                problems.append('fixture pool still contains processes')
            else:
                run('sudo', '-n', 'systemctl', 'stop', slice_name)
                run('sudo', '-n', 'rm', '-f', '--', str(unit))
                run('sudo', '-n', 'rm', '-rf', '--', str(unit) + '.d')
                run('sudo', '-n', 'systemctl', 'daemon-reload')
        print('cleanup:', json.dumps(problems), flush=True)
        if problems: raise RuntimeError('fixture cleanup failed')
