"""Run a real local Exec and queued job against an isolated scripted provider."""
import json, os, secrets, shutil, signal, subprocess, sys, tempfile, time, urllib.request
from pathlib import Path
repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane, free_port

def run(*args, check=True):
    return subprocess.run(args, check=check, text=True, capture_output=True, timeout=120)
def interrupt(*_): raise RuntimeError('fixture interrupted')
signal.signal(signal.SIGTERM, interrupt); signal.signal(signal.SIGALRM, interrupt); signal.alarm(900)
def until(predicate, message, seconds=120):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if predicate(): return
        time.sleep(.5)
    raise RuntimeError(message)
nonce = secrets.token_hex(3)
namespace = 'agentjob_' + nonce + '_test'
company = namespace
slice_name = 'restless-s109aj' + nonce + '.slice'
unit = Path('/run/systemd/system') / slice_name
pool = Path('/sys/fs/cgroup/restless.slice') / slice_name
assert not unit.exists()
with tempfile.TemporaryDirectory(prefix='s109-agentjob-') as folder:
    root = Path(folder)
    for name in ('home', 'plane', 'bin'): (root / name).mkdir()
    (root / 'bin/getconf').write_text('#!/bin/sh\necho 4\n'); (root / 'bin/getconf').chmod(0o755)
    (root / 'meminfo').write_text('MemTotal: 8388608 kB\n')
    source = (Path(os.environ['RESTLESS_CLOUD_DIR']) / 'services/vm-provider/agent/restless-agent.sh').read_text()
    start, end = source.index('install_workload_budget() {'), source.index('\nupdate_agent()')
    setup = root / 'setup.sh'; setup.write_text('log(){ :; }\n' + source[start:end] + '\ninstall_workload_budget ' + slice_name + '\n')
    scenario = root / 'scenario.json'
    scenario.write_text(json.dumps({'rules': [{'match': '.*', 'steps': [{'text': ' '.join('fixture' + str(i) for i in range(80)), 'delay_ms': 500}]}]}))
    mocklog = root / 'requests.jsonl'; port = free_port(); mock = None; plane = None; job = None; job2 = None; owned = False
    try:
        path = str(root / 'bin') + ':' + os.environ['PATH']; owned = True
        run('sudo', '-n', 'env', 'PATH=' + path, 'RESTLESS_HOST_MEMINFO=' + str(root / 'meminfo'),
            'RESTLESS_WORKLOAD_UNIT_DIR=/run/systemd/system', 'bash', str(setup))
        mock = subprocess.Popen(['node', str(repo / 'tools/mock-model/server.mjs'), '--port', str(port),
                                 '--scenario', str(scenario), '--log', str(mocklog)],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, start_new_session=True)
        def mock_ready():
            try:
                with urllib.request.urlopen(f'http://127.0.0.1:{port}/v1/models', timeout=2) as response: return response.status == 200
            except OSError: return False
        until(mock_ready, 'scripted provider did not start', 15)
        plane = Plane(root / 'plane', namespace, os.environ['RESTLESSD_BIN'],
                      os.environ.get('RESTLESS_COMPANY_IMAGE', 'restless-company-image:release-connections'),
                      {'HOME': str(root / 'home'), 'PATH': path, 'RESTLESS_SOURCE_ROOT': str(repo),
                       'RESTLESS_OMP_BIN': os.environ.get('RESTLESS_TEST_OMP_BIN', str(repo / 'infra/host-tools/omp-source')), 'RESTLESS_NODE_BIN': shutil.which('node'),
                       'RESTLESS_TEST_DISABLE_SCHEDULER': '0', 'RESTLESS_COMPANY_CPUS': '1',
                       'RESTLESS_COMPANY_MEMORY': '1024m', 'RESTLESS_COMPUTER_MEMORY': '512m',
                       'RESTLESS_WORKLOAD_CGROUP_PARENT': slice_name, 'RESTLESS_WORKLOAD_CGROUP_ROOT': str(pool),
                       'RESTLESS_ANTHROPIC_BASE_URL': f'http://127.0.0.1:{port}/v1', 'SMOKE_AGENT_KEY': 'synthetic-only-key'})
        plane.start_database(); plane.start(); plane.wait_for_port(plane.offset + 7788)
        plane.create_company(company, f'name = "{company}"\nmodel = "anthropic/claude-opus-5-5"\n'
            'mission = "Wait for the owner."\nspend_ceiling_usd = 10\nauto_sleep_after_minutes = 0\n'
            '[credentials]\n"model.inference" = "env:SMOKE_AGENT_KEY"\n')
        plane.raw(company, {'cmd': 'up'}, timeout=300); plane.wait_for_port(plane.relay_port, 120)
        # Wait for the model gateway to settle after credential reload.
        last = [None, time.monotonic()]
        def gateway_ready():
            log = plane.log_path.read_text(); ready = log.rfind('model gateway ready')
            if ready < 0 or ready < log.rfind('reloading account model services'):
                last[:] = [None, time.monotonic()]; return False
            if ready != last[0]: last[:] = [ready, time.monotonic()]
            return time.monotonic() - last[1] >= 12
        until(gateway_ready, 'model gateway did not settle')
        plane.raw(company, {'cmd': 'tell', 'body': 'Please answer once for the local resource fixture.'})
        def compute():
            status, data = plane.owner(company, '/company/compute')
            return data if status == 200 else {}
        def actor_active():
            data = compute(); cap = data.get('actor_capacity') or {}
            return (cap.get('shared_active') or 0) >= 1 and mocklog.exists() and '/v1/messages' in mocklog.read_text()
        until(actor_active, 'real Exec did not start with the scripted provider', 180)
        # While Exec is active, bring the shared soft threshold above the live
        # working set but below that set plus the actor reservation. A saved
        # browser refresh must remain pending instead of replacing the browser.
        browser = plane.container(company).replace('-co-', '-pc-', 1)
        before_browser = run('docker', 'inspect', '-f', '{{.Id}}', browser).stdout.strip()
        original_high = (pool / 'memory.high').read_text().strip()
        observed = compute()['sample']['work_admission']
        artificial_high = observed['working_set_bytes'] + 512 * 1024 * 1024
        assert artificial_high < int((pool / 'memory.max').read_text().strip())
        try:
            run('sudo', '-n', 'systemctl', 'set-property', '--runtime', slice_name,
                'MemoryHigh=' + str(artificial_high))
            limited = compute()['sample']['work_admission']
            assert not limited['waiting_for_memory'], limited
            assert limited['working_set_bytes'] + 512 * 1024 * 1024 >= limited['resume_below_bytes']
            status, result = plane.owner(company, '/machines/home-sites', {'sites': ['example.test']}, method='PUT')
            assert status == 200 and result['computer_refresh_pending'] is True, (status, result)
            assert run('docker', 'inspect', '-f', '{{.Id}}', browser).stdout.strip() == before_browser
            assert actor_active(), 'agent ended before the browser check'
            print('PASS: browser refresh preserved the live computer while agent headroom was reserved', flush=True)
        finally:
            run('sudo', '-n', 'systemctl', 'set-property', '--runtime', slice_name, 'MemoryHigh=' + original_high)
        data = compute(); sample = data['sample']['work_admission']
        assert sample['scope'] == 'shared' and not sample['waiting_for_memory'], sample
        # Leave 256 MiB below the soft threshold; an active actor holds a 512 MiB
        # allowance, so the job must wait even though current counters alone fit it.
        available = sample['resume_below_bytes'] - sample['working_set_bytes']
        job_mib = (available - 256 * 1024 * 1024) // (1024 * 1024)
        assert 256 <= job_mib < 8192, job_mib
        marker = '/company/run/agentjob-' + nonce
        out = run('docker', 'exec', '-u', 'company', plane.container(company), 'restless', 'job', 'run',
                  '--cpus', '0.2', '--memory', str(job_mib) + 'm', '--timeout', '30', '--', 'printf x >> ' + marker).stdout
        job_id = json.loads(out)['job']; job = plane.container(company) + '-job-' + job_id
        until(lambda: run('docker', 'inspect', '-f', '{{.State.Status}}', job).stdout.strip() == 'created', 'job was not queued', 10)
        time.sleep(6)
        assert actor_active(), 'agent ended before the contention observation'
        observed = compute()['sample']['work_admission']
        actual_limit = int(run('docker', 'inspect', '-f', '{{.HostConfig.Memory}}', job).stdout.strip())
        assert not observed['waiting_for_memory'] and not observed['waiting_for_processes'], observed
        committed_without_claim = observed['working_set_bytes'] + actual_limit
        assert committed_without_claim <= observed['resume_below_bytes'], 'live counters alone already prevent the job'
        assert committed_without_claim + 512 * 1024 * 1024 > observed['resume_below_bytes'], 'actor claim does not create the contention'
        assert run('docker', 'inspect', '-f', '{{.State.Status}}', job).stdout.strip() == 'created', 'job consumed reserved agent headroom'
        def effect(): return run('docker', 'exec', '-u', 'company', plane.container(company), 'cat', marker, check=False)
        assert effect().returncode != 0
        until(lambda: effect().stdout == 'x', 'job did not resume after the agent finished', 180)
        time.sleep(6); assert effect().stdout == 'x'
        assert plane.owner(company, '/principal')[0] == 200
        print('PASS: real Exec held a competing job; job resumed once after the agent finished', flush=True)
        until(lambda: (compute().get('actor_capacity') or {}).get('shared_active') == 0,
              'first Exec did not release its reservation')
        # Bound the pool to 2 GiB in this disposable slice. Select a job limit
        # between the agent's reservation threshold and the job soft threshold.
        run('sudo', '-n', 'systemctl', 'set-property', '--runtime', slice_name,
            'MemoryMax=2147483648', 'MemoryHigh=2040109465')
        def no_agent_wait():
            data = compute(); cap = data.get('actor_capacity') or {}; sample = data.get('sample', {}).get('work_admission', {})
            return cap.get('waiting_for_memory') is False and sample.get('scope') == 'shared' and not sample.get('waiting_for_memory')
        until(no_agent_wait, 'baseline memory did not settle after reducing the fixture pool')
        data = compute(); sample = data['sample']['work_admission']
        maximum = int((pool / 'memory.max').read_text().strip())
        actor_threshold = min(sample['resume_below_bytes'], maximum - 512 * 1024 * 1024)
        assert sample['working_set_bytes'] < actor_threshold < sample['resume_below_bytes']
        target = (actor_threshold + sample['resume_below_bytes']) // 2
        reserved_mib = (target - sample['working_set_bytes']) // (1024 * 1024)
        assert reserved_mib >= 128, reserved_mib
        long_marker = '/company/run/agentjob-long-' + nonce
        command = 'printf started > ' + long_marker + '; sleep 45; printf finished > ' + long_marker
        out = run('docker', 'exec', '-u', 'company', plane.container(company), 'restless', 'job', 'run',
                  '--cpus', '0.2', '--memory', str(reserved_mib) + 'm', '--timeout', '60', '--', command).stdout
        job2_id = json.loads(out)['job']; job2 = plane.container(company) + '-job-' + job2_id
        until(lambda: run('docker', 'inspect', '-f', '{{.State.Status}}', job2).stdout.strip() == 'running',
              'job-first fixture did not start', 60)
        def reservation_blocks_actor():
            data = compute(); cap = data.get('actor_capacity') or {}; sample = data.get('sample', {}).get('work_admission', {})
            return cap.get('waiting_for_memory') is True and sample.get('scope') == 'shared' and not sample.get('waiting_for_memory')
        until(reservation_blocks_actor, 'running job memory did not defer agent capacity', 20)
        sample = compute()['sample']['work_admission']
        actual_limit = int(run('docker', 'inspect', '-f', '{{.HostConfig.Memory}}', job2).stdout.strip())
        assert sample['working_set_bytes'] < actor_threshold <= sample['working_set_bytes'] + actual_limit
        def model_calls():
            return sum(1 for line in mocklog.read_text().splitlines()
                       if json.loads(line).get('method') == 'POST' and json.loads(line).get('path') == '/v1/messages')
        calls_before = model_calls()
        # Keep the second response short after the first test's slow stream.
        scenario.write_text(json.dumps({'rules': [{'match': '.*', 'steps': [{'text': 'The local fixture reply is complete.'}]}]}))
        plane.raw(company, {'cmd': 'tell', 'body': 'Reply after the running fixture job finishes.'})
        time.sleep(7)
        assert run('docker', 'inspect', '-f', '{{.State.Status}}', job2).stdout.strip() == 'running'
        assert model_calls() == calls_before, 'agent called the model while the running job held capacity'
        assert (compute().get('actor_capacity') or {}).get('shared_active') == 0
        assert plane.owner(company, '/principal')[0] == 200
        until(lambda: model_calls() > calls_before, 'agent did not resume after the job finished', 120)
        print('PASS: running job held Exec until its memory reservation was released', flush=True)
        def refreshed():
            status, machines = plane.owner(company, '/machines')
            return status == 200 and machines.get('computer_refresh_pending') is False
        until(refreshed, 'saved browser refresh did not apply after capacity returned', 150)
        assert run('docker', 'inspect', '-f', '{{.Id}}', browser).stdout.strip() != before_browser
        sites = run('docker', 'inspect', '-f', '{{index .Config.Labels "io.restless.home-sites"}}', browser).stdout.strip()
        assert sites == 'example.test', sites
        print('PASS: saved browser refresh applied after capacity returned', flush=True)
    finally:
        problems = []
        if job: run('docker', 'rm', '-f', job, check=False)
        if job2: run('docker', 'rm', '-f', job2, check=False)
        if plane:
            try: problems.extend(plane.close())
            except Exception as error: problems.append(str(error))
        if mock and mock.poll() is None:
            os.killpg(mock.pid, signal.SIGTERM)
            try: mock.wait(timeout=10)
            except subprocess.TimeoutExpired: os.killpg(mock.pid, signal.SIGKILL); mock.wait()
        if owned and unit.exists():
            if pool.exists() and any(p.read_text().strip() for p in pool.rglob('cgroup.procs')): problems.append('fixture pool still has processes')
            else:
                run('sudo', '-n', 'systemctl', 'stop', slice_name)
                run('sudo', '-n', 'rm', '-f', '--', str(unit))
                run('sudo', '-n', 'rm', '-rf', '--', str(unit) + '.d')
                run('sudo', '-n', 'systemctl', 'daemon-reload')
        print('cleanup:', json.dumps(problems), flush=True)
        if problems: raise RuntimeError('fixture cleanup failed')
