"""Run the heavy-job pressure smoke in a new disposable account pool.

Requires a built RESTLESSD_BIN, RESTLESS_CLOUD_DIR, a local company image,
Docker and passwordless sudo for a uniquely named runtime systemd slice. The
fixture refuses to replace an existing unit, owns every test resource it creates,
and reports cleanup problems. It does not use a model provider.
"""
import hashlib
import json
import os
import secrets
import shlex
import shutil
import signal
import subprocess
import sys
import tempfile
import time
from pathlib import Path
repo = Path(__file__).resolve().parents[1]
with open(os.environ['RESTLESSD_BIN'], 'rb') as daemon_file:
    print('daemon sha256:', hashlib.file_digest(daemon_file, 'sha256').hexdigest(), flush=True)
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane
suffix = secrets.token_hex(3)
namespace = 'job_' + suffix + '_test'
company = namespace
peer = 'job_' + suffix + '_b_test'
slice = 'restless-s109job' + suffix + '.slice'
unit = Path('/run/systemd/system') / slice
if unit.exists():
    raise RuntimeError('temporary slice already exists; refusing to alter it')

def stop(*_):
    raise RuntimeError('fixture interrupted')
signal.signal(signal.SIGTERM, stop)
signal.signal(signal.SIGALRM, stop)
signal.alarm(1200)
with tempfile.TemporaryDirectory(prefix='s109-job-fixture-') as folder:
    root = Path(folder)
    (root / 'home').mkdir()
    (root / 'plane').mkdir()
    (root / 'bin').mkdir()
    (root / 'bin/getconf').write_text('#!/bin/sh\necho 4\n')
    (root / 'bin/getconf').chmod(0o755)
    # Inject one scoped inspect failure without stopping or modifying Docker.
    docker = shutil.which('docker')
    if not docker:
        raise RuntimeError('Docker is required')
    flag = root / 'docker-fail-inspect'
    wrapper = root / 'bin/docker'
    wrapper.write_text(
        '#!/bin/sh\n' +
        'if [ "$1" = create ] && [ -f ' + shlex.quote(str(root / 'docker-delay-create')) + ' ]; then\n' +
        '  ' + shlex.quote(docker) + ' "$@" || exit $?\n' +
        '  touch ' + shlex.quote(str(root / 'docker-create-accepted')) + '\n' +
        '  exec sleep 70\n' +
        'fi\n' +
        'if [ "$1" = rename ] && [ -f ' + shlex.quote(str(root / 'docker-delay-rename')) + ' ]; then\n' +
        '  ' + shlex.quote(docker) + ' "$@" || exit $?\n' +
        '  exec sleep 25\n' +
        'fi\n' +
        'if [ "$1" = rename ] && [ -f ' + shlex.quote(str(root / 'docker-fail-rename')) + ' ]; then\n' +
        '  echo "test rename failure" >&2; exit 1\n' +
        'fi\n' +
        'if [ "$1" = inspect ] && [ -f ' + shlex.quote(str(flag)) + ' ]; then\n' +
        '  target=$(cat ' + shlex.quote(str(flag)) + ')\n' +
        '  for arg in "$@"; do [ "$arg" = "$target" ] && { echo "test inspect failure" >&2; exit 1; }; done\n' +
        'fi\nexec ' + shlex.quote(docker) + ' "$@"\n'
    )
    wrapper.chmod(0o755)
    (root / 'meminfo').write_text('MemTotal: 8388608 kB\n')
    source = (Path(os.environ['RESTLESS_CLOUD_DIR']) / 'services/vm-provider/agent/restless-agent.sh').read_text()
    start = source.index('install_workload_budget() {')
    end = source.index('\nupdate_agent()', start)
    setup = root / 'setup.sh'
    setup.write_text('log(){ :; }\n' + source[start:end] + '\ninstall_workload_budget ' + slice + '\n')
    setup.chmod(0o700)
    env = os.environ.copy()
    env.update(PATH=str(root / 'bin') + ':' + env['PATH'], RESTLESS_HOST_MEMINFO=str(root / 'meminfo'), RESTLESS_WORKLOAD_UNIT_DIR='/run/systemd/system')
    plane = None
    cleanup_failed = False
    try:
        subprocess.run(['sudo', '-n', 'env', 'PATH=' + env['PATH'], 'RESTLESS_HOST_MEMINFO=' + str(root / 'meminfo'), 'RESTLESS_WORKLOAD_UNIT_DIR=/run/systemd/system', 'bash', str(setup)], check=True, timeout=30)
        pool = '/sys/fs/cgroup/restless.slice/' + slice
        plane = Plane(root / 'plane', namespace, os.environ['RESTLESSD_BIN'], os.environ.get('RESTLESS_COMPANY_IMAGE', 'restless-company-image:release-connections'), {'HOME': str(root / 'home'), 'PATH': env['PATH'], 'RESTLESS_OMP_BIN': str(repo / 'infra/host-tools/omp-source'), 'RESTLESS_SOURCE_ROOT': str(repo), 'RESTLESS_COMPANY_CPUS': '1', 'RESTLESS_COMPANY_MEMORY': '1024m', 'RESTLESS_COMPUTER_MEMORY': '512m', 'RESTLESS_WORKLOAD_CGROUP_PARENT': slice, 'RESTLESS_WORKLOAD_CGROUP_ROOT': pool, 'RESTLESS_TEST_DISABLE_SCHEDULER': '0'})
        plane.start_database()
        plane.start()
        plane.wait_for_port(plane.offset + 7788)
        plane.create_company(company, f'name = "{company}"\nauto_sleep_after_minutes = 0\n')
        plane.raw(company, {'cmd': 'up'}, timeout=300)
        plane.create_company(peer, f'name = "{peer}"\nauto_sleep_after_minutes = 0\n')
        plane.raw(peer, {'cmd': 'up'}, timeout=300)
        env.update(RESTLESS_SMOKE_COMPANY=company, RESTLESS_SMOKE_PEER_COMPANY=peer, RESTLESS_RESOURCE_NAMESPACE=namespace, RESTLESS_RESOURCE_TEST_SLICE=slice, RESTLESS_SMOKE_ORIGIN=plane.owner_origin, RESTLESS_JOB_RESTART_DIR=str(root), RESTLESS_COMPANY_IMAGE=os.environ.get('RESTLESS_COMPANY_IMAGE', 'restless-company-image:release-connections'))
        original_high = (Path(pool) / 'memory.high').read_text().strip()
        proc = subprocess.Popen(['node', str(repo / 'scripts/verify-heavy-job-admission.mjs')], env=env)
        for i in range(360):
            if (root / 'ready.json').exists():
                break
            if proc.poll() is not None:
                raise RuntimeError('pressure probe stopped before restart')
            time.sleep(0.5)
        else:
            raise RuntimeError('pressure probe did not reach restart')
        queued = json.loads((root / 'ready.json').read_text())['jobName']
        marker = '/company/run/s109-heavy-job-' + namespace
        effect = subprocess.run(['docker', 'exec', '-u', 'company', plane.container(company),
                                 'test', '-e', marker], timeout=15)
        assert effect.returncode != 0, 'job ran before pressure was released'
        plane.raw(company, {'cmd': 'down'}, timeout=90)
        after_stop = subprocess.run(['docker', 'inspect', '-f', '{{.State.Status}}', queued],
                                    capture_output=True, text=True, check=True, timeout=15).stdout.strip()
        runtime_after_stop = subprocess.run(['docker', 'inspect', '-f', '{{.State.Status}}', plane.container(company)], capture_output=True, text=True, check=True, timeout=15).stdout.strip()
        print('after explicit stop:', json.dumps({'job': after_stop, 'runtime': runtime_after_stop}), flush=True)
        assert after_stop == 'created', ('job started during company stop', after_stop)
        assert runtime_after_stop != 'running', 'the company must really be stopped'
        plane.stop()
        state = subprocess.run(['docker', 'inspect', '-f', '{{.State.Status}}', queued], capture_output=True, text=True, check=True, timeout=15).stdout.strip()
        assert state == 'created', state
        log = plane.log_path.open('a')
        plane.process = subprocess.Popen([plane.daemon_bin], cwd=plane.root, env=plane.env(), stdout=log, stderr=subprocess.STDOUT, start_new_session=True)
        plane.wait_for_port(plane.offset + 7788, seconds=40)
        import urllib.request
        with urllib.request.urlopen(plane.owner_origin + '/health', timeout=5) as health:
            assert health.status == 200
        assert subprocess.run(['docker', 'inspect', '-f', '{{.State.Status}}', queued], capture_output=True, text=True, check=True, timeout=15).stdout.strip() == 'created'
        subprocess.run(['sudo', '-n', 'systemctl', 'set-property', '--runtime', slice, 'MemoryHigh=' + original_high], check=True, timeout=30)
        for i in range(120):
            try:
                plane.raw(None, {'cmd': 'company-list'}, timeout=3)
                break
            except (OSError, RuntimeError):
                time.sleep(0.5)
        else:
            raise RuntimeError('daemon did not recover after capacity returned')
        plane.raw(None, {'cmd': 'schedule-wake'})
        time.sleep(6)
        after_restart = subprocess.run(['docker', 'inspect', '-f', '{{.State.Status}}', queued], capture_output=True, text=True, check=True, timeout=15).stdout.strip()
        runtime_after_restart = subprocess.run(['docker', 'inspect', '-f', '{{.State.Status}}', plane.container(company)], capture_output=True, text=True, check=True, timeout=15).stdout.strip()
        print('after restart and capacity recovery:', json.dumps({'job': after_restart, 'runtime': runtime_after_restart}), flush=True)
        assert after_restart == 'created', ('stopped company must keep the job waiting', after_restart, runtime_after_restart)
        plane.raw(company, {'cmd': 'up'}, timeout=300)
        print('stopped company kept its job waiting until explicitly started', flush=True)
        (root / 'resumed').write_text('ready')
        proc.wait(timeout=420)
        print('pressure regression exit:', proc.returncode, flush=True)
        if proc.returncode == 0:
            # A retired requester must not acquire new execution after its job waited.
            plane.raw(company, {'cmd': 'actor-create', 'as_actor': 'design-writer', 'role': 'writer',
                                'name': 'Test Writer', 'actor': 'owner', 'reason': 'Disposable verification'})
            status, sample = plane.owner(company, '/company/compute')
            assert status == 200
            working = sample['sample']['work_admission']['working_set_bytes']
            high = working + min(128 * 1024 * 1024, working // 2)
            subprocess.run(['sudo', '-n', 'systemctl', 'set-property', '--runtime', slice, 'MemoryHigh=' + str(high)], check=True, timeout=30)
            status, sample = plane.owner(company, '/company/compute')
            assert status == 200 and sample['sample']['work_admission']['waiting_for_memory']
            retired_job = plane.container(company) + '-job-' + secrets.token_hex(6)
            (root / 'retired-job.json').write_text(json.dumps({'name': retired_job}))
            marker = '/company/run/retired-job-probe'
            subprocess.run(['docker', 'create', '--name', retired_job, '--label', 'restless.job=1',
                            '--label', 'restless.job.queue=1', '--label', 'restless.job.scope=namespace:' + namespace,
                            '--label', 'restless.company=' + company, '--label', 'restless.job.actor=design-writer',
                            '--memory', '128m', '--cgroup-parent', slice, '-u', 'company', '-v',
                            plane.volume(company) + ':/company', '--entrypoint', 'sh',
                            os.environ.get('RESTLESS_COMPANY_IMAGE', 'restless-company-image:release-connections'),
                            '-c', 'printf x > ' + marker], check=True, capture_output=True, timeout=30)
            plane.raw(company, {'cmd': 'actor-retire', 'as_actor': 'design-writer', 'actor': 'owner',
                                'reason': 'Disposable verification'})
            subprocess.run(['sudo', '-n', 'systemctl', 'set-property', '--runtime', slice,
                            'MemoryHigh=' + original_high], check=True, timeout=30)
            plane.raw(None, {'cmd': 'schedule-wake'})
            for _ in range(80):
                result = plane.raw(company, {'cmd': 'job-status', 'job': {'id': retired_job[-12:]}})
                if result['status'] != 'waiting_for_capacity':
                    break
                time.sleep(0.5)
            assert result['status'] == 'failed' and 'no longer active' in result.get('reason', ''), result
            effect = subprocess.run(['docker', 'exec', '-u', 'company', plane.container(company),
                                     'test', '-e', marker], timeout=15)
            assert effect.returncode != 0, 'retired actor job must have no effect'
            print('retired actor job refused without an effect', flush=True)
            orphan = plane.container(company) + '-job-' + secrets.token_hex(6)
            (root / 'cleanup-job.json').write_text(json.dumps({'name': orphan}))
            subprocess.run(['docker', 'create', '--name', orphan, '--label', 'restless.job=1', '--label', 'restless.company=' + company, '--entrypoint', 'sh', os.environ.get('RESTLESS_COMPANY_IMAGE', 'restless-company-image:release-connections'), '-c', 'exit 0'], capture_output=True, check=True, timeout=20)
            plane.raw(company, {'cmd': 'down', 'destroy': True}, timeout=180)
            assert subprocess.run(['docker', 'inspect', orphan], capture_output=True).returncode != 0
            plane.companies.remove(company)
            print('company destruction removed its saved job and volume', flush=True)
    finally:
        flag.unlink(missing_ok=True)
        (root / 'docker-delay-create').unlink(missing_ok=True)
        (root / 'docker-fail-rename').unlink(missing_ok=True)
        if 'proc' in locals() and proc.poll() is None:
            proc.terminate()
            try:
                proc.wait(timeout=20)
            except subprocess.TimeoutExpired:
                proc.kill()
                proc.wait()
        if unit.exists() and 'original_high' in locals():
            subprocess.run(['sudo', '-n', 'systemctl', 'set-property', '--runtime', slice, 'MemoryHigh=' + original_high], check=True, timeout=30)
        if plane:
            problems = plane.close()
            print('plane cleanup problems:', json.dumps(problems), flush=True)
            if problems:
                cleanup_failed = True
                if (root / 'ready.json').exists():
                    record = json.loads((root / 'ready.json').read_text())
                    for name in [record['jobName'], record.get('legacyName')]:
                        if name:
                            assert any((name.startswith(plane.container(c) + '-job-') for c in [company, peer]))
                            subprocess.run(['docker', 'unpause', name], capture_output=True)
                            subprocess.run(['docker', 'rm', '-f', name], capture_output=True)
                if (root / 'retired-job.json').exists():
                    name = json.loads((root / 'retired-job.json').read_text())['name']
                    assert name.startswith(plane.container(company) + '-job-')
                    subprocess.run(['docker', 'rm', '-f', name, name + '-refused'], capture_output=True)
                if (root / 'cleanup-job.json').exists():
                    name = json.loads((root / 'cleanup-job.json').read_text())['name']
                    assert any((name.startswith(plane.container(c) + '-job-') for c in [company, peer]))
                    subprocess.run(['docker', 'rm', '-f', name], capture_output=True)
                if (root / 'queue.json').exists():
                    for name in json.loads((root / 'queue.json').read_text()):
                        assert any((name.startswith(plane.container(c) + '-job-') for c in [company, peer]))
                        subprocess.run(['docker', 'unpause', name], capture_output=True)
                        subprocess.run(['docker', 'rm', '-f', name], capture_output=True)
                if (root / 'legacy.json').exists():
                    name = json.loads((root / 'legacy.json').read_text())['legacyName']
                    assert any((name.startswith(plane.container(c) + '-job-') for c in [company, peer]))
                    subprocess.run(['docker', 'rm', '-f', name], capture_output=True)
                for c in [company, peer]:
                    subprocess.run(['docker', 'rm', '-f', 'restless-' + namespace + '-pc-' + c], capture_output=True)
                    for volume in ['restless-' + namespace + '-att-' + c, plane.volume(c)]:
                        subprocess.run(['docker', 'volume', 'rm', volume], capture_output=True)
                print('fallback cleanup used', flush=True)
        if unit.exists():
            cg = Path('/sys/fs/cgroup/restless.slice') / slice
            for i in range(30):
                if not cg.exists() or not any((p.read_text().strip() for p in cg.rglob('cgroup.procs') if p.exists())):
                    break
                time.sleep(1)
            else:
                raise RuntimeError('fixture pool still has processes; leaving it intact')
            subprocess.run(['sudo', '-n', 'systemctl', 'stop', slice], check=True)
            subprocess.run(['sudo', '-n', 'rm', str(unit)], check=True)
            subprocess.run(['sudo', '-n', 'systemctl', 'daemon-reload'], check=True)
            print('temporary pool removed:', not unit.exists(), flush=True)
        for item in root.rglob('*'):
            try:
                item.chmod(0o700 if item.is_dir() else 0o600)
            except OSError:
                pass
if proc.returncode:
    raise SystemExit(proc.returncode)
if cleanup_failed:
    raise RuntimeError('fixture required fallback cleanup')
