"""Disposable local smoke for process-pressure admission of a real heavy job.

Requires RESTLESSD_BIN, RESTLESS_CLOUD_DIR, Docker, an existing company image
and passwordless sudo. Owns one uniquely named test slice, database and company;
never targets an existing account or paid model provider.
"""
import json, os, secrets, shutil, signal, subprocess, sys, tempfile, time
from pathlib import Path
repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane

def run(*args, check=True, **kwargs):
    return subprocess.run(args, check=check, text=True, capture_output=True, timeout=60, **kwargs)

def interrupt(*_):
    raise RuntimeError('fixture interrupted')
signal.signal(signal.SIGTERM, interrupt)
signal.signal(signal.SIGALRM, interrupt)
signal.alarm(900)
nonce = secrets.token_hex(3)
namespace = 'pid_' + nonce + '_test'
company = namespace
slice_name = 'restless-s109pid' + nonce + '.slice'
unit = Path('/run/systemd/system') / slice_name
assert not unit.exists(), 'refusing to alter an existing slice'
image = os.environ.get('RESTLESS_COMPANY_IMAGE', 'restless-company-image:release-connections')
with tempfile.TemporaryDirectory(prefix='s109-pid-admission-') as folder:
    root = Path(folder)
    for name in ['home', 'plane', 'bin']:
        (root / name).mkdir()
    (root / 'bin/getconf').write_text('#!/bin/sh\necho 4\n')
    (root / 'bin/getconf').chmod(0o755)
    (root / 'meminfo').write_text('MemTotal: 8388608 kB\n')
    source = (Path(os.environ['RESTLESS_CLOUD_DIR']) / 'services/vm-provider/agent/restless-agent.sh').read_text()
    start, end = source.index('install_workload_budget() {'), source.index('\nupdate_agent()')
    setup = root / 'setup.sh'
    setup.write_text('log(){ :; }\n' + source[start:end] + '\ninstall_workload_budget ' + slice_name + '\n')
    pool = Path('/sys/fs/cgroup/restless.slice') / slice_name
    plane = None
    owned_unit = False
    job = None
    try:
        own_path = str(root / 'bin') + ':' + os.environ['PATH']
        owned_unit = True
        run('sudo', '-n', 'env', 'PATH=' + own_path, 'RESTLESS_HOST_MEMINFO=' + str(root / 'meminfo'),
            'RESTLESS_WORKLOAD_UNIT_DIR=/run/systemd/system', 'bash', str(setup))
        plane = Plane(root / 'plane', namespace, os.environ['RESTLESSD_BIN'], image, {
            'HOME': str(root / 'home'), 'PATH': own_path,
            'RESTLESS_OMP_BIN': str(repo / 'infra/host-tools/omp-source'), 'RESTLESS_SOURCE_ROOT': str(repo),
            'RESTLESS_COMPANY_CPUS': '1', 'RESTLESS_COMPANY_MEMORY': '1024m',
            'RESTLESS_COMPUTER_MEMORY': '512m', 'RESTLESS_WORKLOAD_CGROUP_PARENT': slice_name,
            'RESTLESS_WORKLOAD_CGROUP_ROOT': str(pool), 'RESTLESS_TEST_DISABLE_SCHEDULER': '0'})
        plane.start_database(); plane.start(); plane.wait_for_port(plane.offset + 7788)
        plane.create_company(company, 'name = "' + company + '"\nauto_sleep_after_minutes = 0\n')
        plane.raw(company, {'cmd': 'up'}, timeout=300)
        current = int((pool / 'pids.current').read_text().strip())
        assert current > 0
        maximum = current + 24
        run('sudo', '-n', 'systemctl', 'set-property', '--runtime', slice_name, 'TasksMax=' + str(maximum))
        status, reading = plane.owner(company, '/company/compute')
        assert status == 200 and reading['sample']['work_admission']['waiting_for_processes'], reading
        assert not reading['sample']['work_admission']['waiting_for_memory'], reading
        for path in ['/principal', '/attention', '/work-graph']:
            status, _ = plane.owner(company, path)
            assert status == 200, (path, status)
        container = plane.container(company)
        marker = '/company/run/s109-pid-capacity-marker'
        output = run('docker', 'exec', '-u', 'company', container, 'restless', 'job', 'run',
                     '--cpus', '0.2', '--memory', '128m', '--timeout', '30', '--', 'printf x >> ' + marker).stdout
        job_id = json.loads(output)['job']
        assert len(job_id) == 12 and all(c in '0123456789abcdef' for c in job_id)
        job = container + '-job-' + job_id
        state = run('docker', 'inspect', '-f', '{{.State.Status}}', job).stdout.strip()
        assert state == 'created', state
        job_status = json.loads(run('docker', 'exec', '-u', 'company', container, 'restless', 'job', 'status', job_id).stdout)
        assert job_status['status'] == 'waiting_for_capacity', job_status
        assert run('docker', 'exec', '-u', 'company', container, 'test', '-e', marker, check=False).returncode != 0
        time.sleep(6)
        assert run('docker', 'inspect', '-f', '{{.State.Status}}', job).stdout.strip() == 'created'
        run('sudo', '-n', 'systemctl', 'set-property', '--runtime', slice_name, 'TasksMax=4096')
        for _ in range(120):
            result = run('docker', 'exec', '-u', 'company', container, 'cat', marker, check=False)
            if result.returncode == 0:
                assert result.stdout == 'x'
                break
            time.sleep(0.5)
        else:
            raise RuntimeError('queued job did not resume after process headroom returned')
        time.sleep(6)
        assert run('docker', 'exec', '-u', 'company', container, 'cat', marker).stdout == 'x', 'job repeated its effect'
        after = int((pool / 'pids.current').read_text().strip())
        events = (pool / 'pids.events').read_text().strip()
        print(json.dumps({'company': company, 'initial_tasks': current, 'test_cap': maximum,
                          'job_waited': True, 'job_resumed_once': True,
                          'tasks_after': after, 'events': events, 'owner_entry': 'healthy'}), flush=True)
    finally:
        problems = []
        # Leave room for the plane's own graceful teardown, even after failure.
        if owned_unit and unit.exists():
            try:
                run('sudo', '-n', 'systemctl', 'set-property', '--runtime', slice_name, 'TasksMax=4096')
            except Exception as error:
                problems.append('restore test task cap: ' + str(error))
        if job:
            run('docker', 'rm', '-f', job, check=False)
        if plane:
            try:
                problems.extend(plane.close())
            except Exception as error:
                problems.append(str(error))
            print('plane cleanup:', json.dumps(problems), flush=True)
        if owned_unit and unit.exists():
            if pool.exists() and any(p.read_text().strip() for p in pool.rglob('cgroup.procs')):
                raise RuntimeError('fixture pool still contains processes; leaving slice intact')
            run('sudo', '-n', 'systemctl', 'stop', slice_name)
            assert unit.parent == Path('/run/systemd/system') and unit.name == slice_name
            run('sudo', '-n', 'rm', '-f', '--', str(unit))
            run('sudo', '-n', 'rm', '-rf', '--', str(unit) + '.d')
            run('sudo', '-n', 'systemctl', 'daemon-reload')
            print('temporary slice removed:', not unit.exists(), flush=True)
        if problems: raise RuntimeError('temporary plane cleanup failed')
