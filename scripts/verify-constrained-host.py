#!/usr/bin/env python3
"""Constrained-host resource gate (Sprint 109 A0.6).

Runs the real daemon against a disposable account workload pool sized like the
production plan (8 GB / 4 CPU by default, the pool Cloud's VM agent installs),
with a scripted model provider and several `_test` companies. It checks:

  two-chats     two conversations at once in one deliberately small Runtime
                (768 MiB by default) both answer, with no child OOM kill;
  simultaneous  every company asleep, then woken at the same moment by owner
                messages: all answer, the pool never OOM-kills, entry stays up;
  idle          a quiet window makes zero model calls;
  restart       the daemon restarts (a release roll-forward's restart) while
                companies hold state; owner entry answers and the next turn works.

Throughout, owner entry (health, principal, Inbox, Work) is probed and its
latency recorded. Everything the gate creates is removed, including on failure.

Needs: a built restlessd (RESTLESSD_BIN), an installed company image, Docker
with the systemd cgroup driver, passwordless sudo for one uniquely named
transient slice, and the Cloud VM agent script (RESTLESS_CLOUD_AGENT, or
RESTLESS_CLOUD_DIR/services/vm-provider/agent/restless-agent.sh) whose
install_workload_budget defines the pool. No real model credentials are used.
"""
import argparse
import datetime
import hashlib
import json
import os
import secrets
import shutil
import signal
import subprocess
import sys
import tempfile
import threading
import time
import urllib.error
import urllib.request
import uuid
from pathlib import Path

repo = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane, docker, free_port  # noqa: E402

REPLY = 'Constrained host reply complete.'


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def wait_for(check, seconds, description, interval=0.5):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if check():
            return
        time.sleep(interval)
    raise AssertionError(description)


def stop(process):
    if process is not None and process.poll() is None:
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=10)


def read_events(path):
    try:
        return {key: int(value) for key, value in
                (line.split() for line in (path / 'memory.events').read_text().splitlines())}
    except OSError:
        return None


def read_int(path, name):
    try:
        return int((path / name).read_text().strip())
    except (OSError, ValueError):
        return None


class Probe(threading.Thread):
    """Owner entry under load: the reads a person needs to get in and see their Inbox and Work."""

    def __init__(self, plane, companies, peer):
        super().__init__(daemon=True)
        self.plane, self.companies, self.peer = plane, companies, peer
        self.halt = threading.Event()
        self.paused = threading.Event()
        self.resting = threading.Event()
        self.results = []

    def pause(self):
        """Stop probing, and wait until no probe is in flight (for a deliberate daemon stop)."""
        self.resting.clear()
        self.paused.set()
        self.resting.wait(60)

    def get(self, url):
        start = time.monotonic()
        try:
            request = urllib.request.Request(url, headers={'Origin': self.plane.owner_origin})
            with urllib.request.urlopen(request, timeout=15) as response:
                status = response.status
        except urllib.error.HTTPError as error:
            status = error.code
        except OSError:
            status = 0
        return status, round((time.monotonic() - start) * 1000)

    def run(self):
        base = self.plane.owner_origin
        while not self.halt.is_set():
            if not self.paused.is_set():
                for company in self.companies + [self.peer]:
                    for path in ('/health', f'/api/companies/{company}/principal',
                                 f'/api/companies/{company}/attention', f'/api/companies/{company}/work-graph'):
                        status, ms = self.get(base + path)
                        self.results.append({'at': now(), 'path': path, 'status': status, 'ms': ms})
            else:
                self.resting.set()
            self.halt.wait(2)

    def summary(self, since=0):
        rows = self.results[since:]
        failed = [row for row in rows if row['status'] != 200]
        return {'checks': len(rows), 'failed': len(failed), 'failures': failed[:10],
                'maxMs': max((row['ms'] for row in rows), default=None)}


def install_pool(root, slice_name, agent, memory_kib, cpus):
    """The exact pool Cloud's VM agent installs, for a host of this size."""
    source = agent.read_text()
    start = source.index('install_workload_budget() {')
    end = source.index('\nupdate_agent()', start)
    (root / 'bin').mkdir(exist_ok=True)
    (root / 'bin/getconf').write_text(f'#!/bin/sh\necho {cpus}\n')
    (root / 'bin/getconf').chmod(0o755)
    (root / 'meminfo').write_text(f'MemTotal: {memory_kib} kB\n')
    setup = root / 'pool.sh'
    setup.write_text('log(){ :; }\n' + source[start:end] + f'\ninstall_workload_budget {slice_name}\n')
    setup.chmod(0o700)
    subprocess.run(['sudo', '-n', 'env', f'PATH={root / "bin"}:{os.environ["PATH"]}',
                    f'RESTLESS_HOST_MEMINFO={root / "meminfo"}', 'RESTLESS_WORKLOAD_UNIT_DIR=/run/systemd/system',
                    'bash', str(setup)], check=True, timeout=60)
    return Path('/sys/fs/cgroup/restless.slice') / slice_name


def remove_pool(slice_name, pool):
    unit = Path('/run/systemd/system') / slice_name
    if not unit.exists():
        return []
    for _ in range(30):
        if not pool.exists() or not any(p.read_text().strip() for p in pool.rglob('cgroup.procs') if p.exists()):
            break
        time.sleep(1)
    else:
        return ['pool still has processes; left intact: ' + slice_name]
    subprocess.run(['sudo', '-n', 'systemctl', 'stop', slice_name], check=True, timeout=60)
    subprocess.run(['sudo', '-n', 'rm', str(unit)], check=True, timeout=30)
    subprocess.run(['sudo', '-n', 'systemctl', 'daemon-reload'], check=True, timeout=60)
    return []


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument('--image', default=os.environ.get('RESTLESS_COMPANY_IMAGE', 'restless-company-image:release-connections'))
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--companies', type=int, default=3, help='companies sharing the account pool')
    parser.add_argument('--host-memory-gib', type=float, default=8)
    parser.add_argument('--host-cpus', type=int, default=4)
    parser.add_argument('--runtime-memory', default='768m', help='the stressed company Runtime ceiling')
    parser.add_argument('--idle-seconds', type=int, default=60)
    parser.add_argument('--reply-seconds', type=int, default=240, help='longest a queued reply may wait')
    parser.add_argument('--only', choices=['two-chats', 'simultaneous', 'idle', 'restart'], action='append')
    args = parser.parse_args()
    scenarios = args.only or ['two-chats', 'simultaneous', 'idle', 'restart']
    daemon = Path(os.environ.get('RESTLESSD_BIN', str(repo / 'target/debug/restlessd')))
    assert daemon.is_file(), 'Build restlessd with scripts/restless-cargo first'
    agent = Path(os.environ.get('RESTLESS_CLOUD_AGENT') or
                 Path(os.environ.get('RESTLESS_CLOUD_DIR', repo.parent / 'restless-cloud'))
                 / 'services/vm-provider/agent/restless-agent.sh')
    assert agent.is_file(), f'Cloud VM agent script not found at {agent}; set RESTLESS_CLOUD_AGENT'
    args.output.mkdir(parents=True, exist_ok=True)
    with daemon.open('rb') as binary:
        daemon_hash = hashlib.file_digest(binary, 'sha256').hexdigest()
    report = {'startedAt': now(), 'daemonSha256': daemon_hash,
              'image': docker('image', 'inspect', '--format', '{{.Id}}', args.image).stdout.strip(),
              'sourceRevision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip(),
              'host': {'memoryGiB': args.host_memory_gib, 'cpus': args.host_cpus},
              'runtimeMemory': args.runtime_memory, 'scenarios': {}, 'status': 'running'}
    suffix = secrets.token_hex(3)
    namespace = f'gate_{suffix}_test'
    slice_name = f'restless-a06gate{suffix}.slice'
    assert not (Path('/run/systemd/system') / slice_name).exists()
    mock = plane = probe = None
    pool = None
    problems = []
    with tempfile.TemporaryDirectory(prefix='restless-constrained-host-') as directory:
        root = Path(directory)
        log_path = root / 'requests.jsonl'
        mock_stdout = (root / 'mock.log').open('w')
        port = free_port()
        scenario = root / 'scenario.json'
        # Slow streamed replies, so two conversations really overlap.
        scenario.write_text(json.dumps({'rules': [{'match': '.*', 'steps': [
            {'text': REPLY + ' ' + ' '.join(['Streaming steadily.'] * 40), 'delay_ms': 150}]}]}))

        def calls():
            rows = [json.loads(line) for line in log_path.read_text().splitlines()] if log_path.exists() else []
            return [row for row in rows if row['method'] == 'POST' and row['path'] == '/v1/messages']

        try:
            pool = install_pool(root, slice_name, agent, int(args.host_memory_gib * 1024 * 1024), args.host_cpus)
            report['pool'] = {name: (pool / name).read_text().strip()
                              for name in ('memory.high', 'memory.max', 'cpu.max', 'memory.swap.max')}
            mock = subprocess.Popen(['node', str(repo / 'tools/mock-model/server.mjs'), '--port', str(port),
                                     '--scenario', str(scenario), '--log', str(log_path)],
                                    stdout=mock_stdout, stderr=subprocess.STDOUT, start_new_session=True)

            def mock_ready():
                try:
                    with urllib.request.urlopen(f'http://127.0.0.1:{port}/v1/models', timeout=2) as response:
                        return response.status == 200
                except OSError:
                    return False
            wait_for(mock_ready, 15, 'Scripted provider did not start')
            (root / 'plane').mkdir()
            (root / 'home').mkdir()
            plane = Plane(root / 'plane', namespace, str(daemon), args.image, {
                'HOME': str(root / 'home'),
                'PATH': f'{root / "bin"}:{os.environ["PATH"]}',
                'RESTLESS_TEST_DISABLE_SCHEDULER': '0',
                'RESTLESS_SOURCE_ROOT': str(repo),
                'RESTLESS_OMP_BIN': str(repo / 'infra/host-tools/omp-source'),
                'RESTLESS_NODE_BIN': shutil.which('node'),
                'RESTLESS_ANTHROPIC_BASE_URL': f'http://127.0.0.1:{port}/v1',
                'SMOKE_GATE_KEY': 'synthetic-gate-key',
                'RESTLESS_COMPANY_CPUS': '1', 'RESTLESS_COMPANY_MEMORY': args.runtime_memory,
                'RESTLESS_COMPUTER_MEMORY': '768m',
                'RESTLESS_WORKLOAD_CGROUP_PARENT': slice_name, 'RESTLESS_WORKLOAD_CGROUP_ROOT': str(pool),
                'RUST_LOG': 'info',
            })
            plane.start_database()
            plane.start()
            companies = [f'gate_{suffix}_{number}_test' for number in range(1, args.companies + 1)]
            peer = f'gate_{suffix}_peer_test'
            for company in companies + [peer]:
                body = (f'name = "{company}"\nmodel = "anthropic/claude-opus-5-5"\n'
                        'mission = "Answer the owner briefly; do not invent projects."\n'
                        'spend_ceiling_usd = 10\nauto_sleep_after_minutes = 0\n'
                        '[credentials]\n"model.inference" = "env:SMOKE_GATE_KEY"\n')
                plane.create_company(company, body)
                plane.raw(company, {'cmd': 'up'}, timeout=600)
            plane.wait_for_port(plane.relay_port, 120)
            ready_seen = [None, time.monotonic()]

            def gateway_settled():
                log = plane.log_path.read_text()
                ready = log.rfind('model gateway ready')
                if ready < 0 or ready < log.rfind('reloading account model services'):
                    ready_seen[:] = [None, time.monotonic()]
                    return False
                if ready != ready_seen[0]:
                    ready_seen[:] = [ready, time.monotonic()]
                return time.monotonic() - ready_seen[1] >= 12
            wait_for(gateway_settled, 180, 'Model routes did not become stably ready')
            probe = Probe(plane, companies, peer)
            probe.start()

            def runtime_scope(company):
                cid = docker('inspect', '-f', '{{.Id}}', plane.container(company), check=False).stdout.strip()
                return pool / f'docker-{cid}.scope' if cid else None

            def replies(company, actor):
                status, view = plane.owner(company, f'/actors/{actor}/conversation')
                if status != 200:
                    return -1
                return sum(message['from_actor'] == actor and REPLY in message['body']
                           for message in view.get('messages', []))

            def send(company, actor, text):
                boundary = uuid.uuid4().hex
                fields = {'body': text, 'client_command_id': str(uuid.uuid4())}
                payload = ''.join(f'--{boundary}\r\nContent-Disposition: form-data; name="{key}"\r\n\r\n{value}\r\n'
                                  for key, value in fields.items()) + f'--{boundary}--\r\n'
                request = urllib.request.Request(
                    f'{plane.owner_origin}/api/companies/{company}/actors/{actor}/conversation',
                    data=payload.encode(), method='POST',
                    headers={'Origin': plane.owner_origin, 'Content-Type': f'multipart/form-data; boundary={boundary}'})
                with urllib.request.urlopen(request, timeout=60) as response:
                    assert response.status in (200, 201, 202), response.status

            def settled(company):
                status, view = plane.owner(company, '/cockpit')
                return status == 200 and not any(person['session_running'] for person in view['people'])

            pool_events_before = read_events(pool)

            if 'two-chats' in scenarios:
                company = companies[0]
                plane.raw(company, {'cmd': 'actor-create', 'as_actor': 'gate-writer', 'role': 'writer',
                                    'name': 'Gate Writer', 'actor': 'owner', 'reason': 'Disposable verification'})
                scope = runtime_scope(company)
                events_before = read_events(scope)
                before = {actor: replies(company, actor) for actor in ('exec', 'gate-writer')}
                started = time.monotonic()
                samples = []
                send(company, 'exec', 'Two chats at once: please reply.')
                send(company, 'gate-writer', 'Two chats at once: please reply too.')
                answered = {}
                deadline = started + args.reply_seconds
                while time.monotonic() < deadline and len(answered) < 2:
                    for actor in ('exec', 'gate-writer'):
                        if actor not in answered and replies(company, actor) > before[actor]:
                            answered[actor] = round(time.monotonic() - started, 1)
                    status, compute = plane.owner(company, '/company/compute')
                    samples.append({'t': round(time.monotonic() - started, 1),
                                    'memory': read_int(scope, 'memory.current'),
                                    'actors': compute.get('actor_capacity') if status == 200 else None,
                                    'admission': (compute.get('sample') or {}).get('work_admission') if status == 200 else None})
                    time.sleep(2)
                events_after = read_events(scope)
                result = {'answeredAfterSeconds': answered, 'samples': samples,
                          'runtimeEventsBefore': events_before, 'runtimeEventsAfter': events_after}
                result['pass'] = len(answered) == 2 and (events_after or {}).get('oom_kill', 0) == (events_before or {}).get('oom_kill', 0)
                report['scenarios']['two-chats'] = result
                print(json.dumps({'scenario': 'two-chats', 'pass': result['pass'], 'answered': answered}), flush=True)
                wait_for(lambda: settled(company), 180, 'two-chats turns did not settle')

            if 'simultaneous' in scenarios:
                # An owner stop is explicit and never woken; a sleeping computer is what a message wakes.
                for company in companies:
                    wait_for(lambda: settled(company), 180, f'{company} did not settle before sleep')
                    def asleep_now():
                        # Sleep refuses while startup work is still settling; it sleeps once quiet.
                        try:
                            plane.raw(company, {'cmd': 'sleep'}, timeout=180)
                            return True
                        except RuntimeError as error:
                            if 'busy' not in str(error):
                                raise
                            return False
                    wait_for(asleep_now, 300, f'{company} never became quiet enough to sleep', interval=5)
                asleep = {company: docker('inspect', '-f', '{{.State.Status}}', plane.container(company),
                                          check=False).stdout.strip() for company in companies}
                assert all(state != 'running' for state in asleep.values()), asleep
                before = {company: replies(company, 'exec') for company in companies}
                mark = len(probe.results)
                started = time.monotonic()
                threads = [threading.Thread(target=send, args=(company, 'exec', 'Everyone at once: please reply.'))
                           for company in companies]
                for thread in threads:
                    thread.start()
                for thread in threads:
                    thread.join()
                answered = {}
                peak = 0
                while time.monotonic() < started + args.reply_seconds and len(answered) < len(companies):
                    peak = max(peak, read_int(pool, 'memory.current') or 0)
                    for company in companies:
                        if company not in answered and replies(company, 'exec') > before[company]:
                            answered[company] = round(time.monotonic() - started, 1)
                    time.sleep(2)
                result = {'answeredAfterSeconds': answered, 'poolPeakBytes': peak,
                          'entry': probe.summary(mark)}
                result['pass'] = len(answered) == len(companies) and result['entry']['failed'] == 0
                report['scenarios']['simultaneous'] = result
                print(json.dumps({'scenario': 'simultaneous', 'pass': result['pass'], 'answered': answered}), flush=True)
                for company in companies:
                    wait_for(lambda: settled(company), 180, f'{company} did not settle')

            if 'idle' in scenarios:
                baseline = len(calls())
                time.sleep(10)
                baseline = len(calls())
                time.sleep(args.idle_seconds)
                result = {'seconds': args.idle_seconds, 'modelCalls': len(calls()) - baseline}
                result['pass'] = result['modelCalls'] == 0
                report['scenarios']['idle'] = result
                print(json.dumps({'scenario': 'idle', **result}), flush=True)

            if 'restart' in scenarios:
                probe.pause()
                plane.stop()
                log = plane.log_path.open('a')
                restarted = time.monotonic()
                plane.process = subprocess.Popen([plane.daemon_bin], cwd=plane.root, env=plane.env(), stdout=log,
                                                 stderr=subprocess.STDOUT, start_new_session=True)
                plane.wait_for_port(plane.offset + 7788, seconds=90)
                entry_seconds = round(time.monotonic() - restarted, 1)
                mark = len(probe.results)
                probe.paused.clear()
                company = companies[-1]
                before = replies(company, 'exec')
                wait_for(gateway_settled, 180, 'Model routes did not return after restart')
                send(company, 'exec', 'After the restart: please reply.')
                answered = None
                started = time.monotonic()
                while time.monotonic() < started + args.reply_seconds:
                    if replies(company, 'exec') > before:
                        answered = round(time.monotonic() - started, 1)
                        break
                    time.sleep(2)
                time.sleep(4)
                result = {'ownerEntrySeconds': entry_seconds, 'answeredAfterSeconds': answered,
                          'entry': probe.summary(mark)}
                result['pass'] = answered is not None and result['entry']['failed'] == 0
                report['scenarios']['restart'] = result
                print(json.dumps({'scenario': 'restart', 'pass': result['pass'], 'answered': answered}), flush=True)

            pool_events_after = read_events(pool)
            report['poolEvents'] = {'before': pool_events_before, 'after': pool_events_after}
            report['entry'] = probe.summary()
            pool_oom = (pool_events_after or {}).get('oom_kill', 0) - (pool_events_before or {}).get('oom_kill', 0)
            ok = all(result['pass'] for result in report['scenarios'].values())
            ok = ok and pool_oom == 0 and report['entry']['failed'] == 0
            report['status'] = 'pass' if ok else 'fail'
        except BaseException as error:
            report['status'] = 'fail'
            report['error'] = f'{type(error).__name__}: {error}'
            raise
        finally:
            if probe is not None:
                probe.halt.set()
                probe.join(timeout=20)
            if plane is not None:
                if plane.log_path.exists():
                    shutil.copy(plane.log_path, args.output / 'daemon.log')
                problems.extend(plane.close())
            stop(mock)
            mock_stdout.close()
            if pool is not None:
                try:
                    problems.extend(remove_pool(slice_name, pool))
                except Exception as error:  # noqa: BLE001 - reported below
                    problems.append(f'pool removal: {error}')
            report['cleanupProblems'] = problems
            report['finishedAt'] = now()
            if problems:
                report['status'] = 'fail'
            (args.output / 'evidence.json').write_text(json.dumps(report, indent=2) + '\n')
            print(json.dumps({key: report[key] for key in ('status', 'cleanupProblems')} |
                             {'scenarios': {name: result.get('pass') for name, result in report['scenarios'].items()}}),
                  flush=True)
            for path in root.rglob('*'):
                try:
                    path.chmod(0o700 if path.is_dir() else 0o600)
                except OSError:
                    pass
    assert report['status'] == 'pass', report.get('error') or 'a scenario failed; see evidence.json'


if __name__ == '__main__':
    def interrupted(signum, frame):
        raise RuntimeError(f'Interrupted by signal {signum}')
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGALRM, interrupted)
    signal.alarm(3600)
    main()
