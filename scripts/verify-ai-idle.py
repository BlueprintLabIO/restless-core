#!/usr/bin/env python3
"""Prove model-connected quiet companies spend no calls across a normal repair sweep.

Uses the existing isolated smoke plane and scripted provider. No real model credentials.
Build restlessd first, then pass an installed company image and an evidence directory.
"""
import argparse
import datetime
import hashlib
import json
import os
from pathlib import Path
import re
import secrets
import shutil
import signal
import subprocess
import sys
import tempfile
import time
import urllib.request

sys.path.insert(0, str(Path(__file__).resolve().parent / 'lib'))
from smoke_plane import Plane, docker, free_port


def now():
    return datetime.datetime.now(datetime.timezone.utc).isoformat()


def wait_for(check, seconds, description):
    deadline = time.monotonic() + seconds
    while time.monotonic() < deadline:
        if check():
            return
        time.sleep(0.5)
    raise AssertionError(description)


def stop(process):
    if process is not None and process.poll() is None:
        os.killpg(process.pid, signal.SIGTERM)
        try:
            process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            os.killpg(process.pid, signal.SIGKILL)
            process.wait(timeout=10)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--image', required=True, help='Installed company Runtime image')
    parser.add_argument('--output', required=True, type=Path)
    parser.add_argument('--seconds', type=int, default=330)
    args = parser.parse_args()
    repo = Path(__file__).resolve().parent.parent
    repair = re.search(r'REPAIR_SWEEP_INTERVAL: Duration = Duration::from_secs\((\d+)\)',
                       (repo / 'crates/restless-engine/src/schedule.rs').read_text())
    assert repair, 'Read the actual repair sweep before choosing an observation interval'
    assert args.seconds >= int(repair[1]) + 10, 'Observe longer than a normal repair sweep'
    daemon = Path(os.environ.get('RESTLESSD_BIN', str(repo / 'target/debug/restlessd')))
    assert daemon.is_file(), 'Build restlessd with scripts/restless-cargo first'
    image_id = docker('image', 'inspect', '--format', '{{.Id}}', args.image).stdout.strip()
    args.output.mkdir(parents=True, exist_ok=True)
    with daemon.open('rb') as binary:
        daemon_hash = hashlib.file_digest(binary, 'sha256').hexdigest()
    report = {'startedAt': now(), 'image': image_id,
              'daemonSha256': daemon_hash,
              'sourceRevision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo, text=True).strip(),
              'schedulerDisabled': False, 'repairSweepSeconds': int(repair[1]),
              'observationSeconds': args.seconds, 'status': 'running'}
    mock = plane = None
    problems = []
    with tempfile.TemporaryDirectory(prefix='restless-ai-idle-') as directory:
        root = Path(directory)
        log_path = root / 'requests.jsonl'
        mock_stdout = (root / 'mock.log').open('w')
        port = free_port()
        scenario = root / 'scenario.json'
        scenario.write_text(json.dumps({'rules': [{'match': '.*', 'steps': [
            {'text': 'The idle verification reply is complete. No further work is due.'}]}]}))
        def calls():
            rows = [json.loads(line) for line in log_path.read_text().splitlines()] if log_path.exists() else []
            return [row for row in rows if row['method'] == 'POST' and row['path'] == '/v1/messages']

        try:
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
            plane_root = root / 'plane'
            plane_root.mkdir()
            home = root / 'home'
            home.mkdir()
            suffix = secrets.token_hex(3)
            plane = Plane(plane_root, f'aiidle_{suffix}_test', str(daemon), args.image, {
                'HOME': str(home),
                'RESTLESS_TEST_DISABLE_SCHEDULER': '0',
                'RESTLESS_SOURCE_ROOT': str(repo),
                'RESTLESS_OMP_BIN': str(repo / 'infra/host-tools/omp-source'),
                'RESTLESS_NODE_BIN': shutil.which('node'),
                'RESTLESS_ANTHROPIC_BASE_URL': f'http://127.0.0.1:{port}/v1',
                'SMOKE_QUIET_KEY_ONE': 'synthetic-idle-one',
                'SMOKE_QUIET_KEY_TWO': 'synthetic-idle-two',
                'RESTLESS_COMPANY_CPUS': '0.5', 'RESTLESS_COMPANY_MEMORY': '768m',
                'RESTLESS_COMPUTER_MEMORY': '768m', 'RUST_LOG': 'info',
            })
            assert plane.env()['RESTLESS_TEST_DISABLE_SCHEDULER'] == '0'
            plane.start_database()
            plane.start()
            companies = [f'quiet_{suffix}_{number}_test' for number in (1, 2)]
            for company, credential in zip(companies, ('ONE', 'TWO')):
                body = (f'name = "{company}"\nmodel = "anthropic/claude-opus-5-5"\n'
                        'mission = "Wait for owner requests; do not invent projects."\n'
                        'spend_ceiling_usd = 10\nauto_sleep_after_minutes = 0\n'
                        f'[credentials]\n"model.inference" = "env:SMOKE_QUIET_KEY_{credential}"\n')
                plane.create_company(company, body)
                plane.raw(company, {'cmd': 'up'}, timeout=600)
            plane.wait_for_port(plane.relay_port, 120)
            # New credential sets reload the gateway. A listening port alone
            # can belong to the generation that is about to stop.
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
            wait_for(gateway_settled, 120, 'Final credential routes did not become stably ready')
            for company in companies:
                plane.raw(company, {'cmd': 'tell', 'body': 'Reply once to verify this connection; no follow-up work is due.'})
                def answered():
                    status, view = plane.owner(company, '/actors/exec/conversation')
                    return status == 200 and any(message['from_actor'] == 'exec' and
                        'idle verification reply is complete' in message['body'] for message in view['messages'])
                wait_for(answered, 180, f'{company} did not produce the positive-control reply')
            assert len(calls()) >= len(companies), 'Actual owner replies must reach the observed provider'
            # A final completion request may follow the visible reply. Begin only
            # after all active turns end, then require ten seconds of quiet.
            def settled():
                for company in companies:
                    status, view = plane.owner(company, '/cockpit')
                    assert status == 200 and view['source_health']['orgintel'] == 'available'
                    assert any(person['actor_id'] == 'exec' for person in view['people'])
                    if any(person['session_running'] for person in view['people']):
                        return False
                return True
            wait_for(settled, 120, 'Positive-control turns did not settle')
            baseline = len(calls())
            time.sleep(10)
            assert len(calls()) == baseline, 'Positive-control calls continued after settlement'
            report['positiveControlCalls'] = baseline
            report['idleStartedAt'] = now()
            start = time.monotonic()
            samples = []
            while time.monotonic() - start < args.seconds:
                assert plane.process.poll() is None, 'Daemon stopped during idle observation'
                assert mock.poll() is None, 'Provider stopped during idle observation'
                assert len(calls()) == baseline, 'A quiet company made a model call'
                samples.append({'at': now(), 'modelCalls': len(calls()) - baseline})
                print(json.dumps({'phase': 'idle', 'elapsedSeconds': round(time.monotonic() - start),
                                  'modelCalls': len(calls()) - baseline}), flush=True)
                time.sleep(min(30, max(0, args.seconds - (time.monotonic() - start))))
            report['idleElapsedSeconds'] = time.monotonic() - start
            report['idleModelCalls'] = len(calls()) - baseline
            assert report['idleModelCalls'] == 0
            # Repeat the positive control after the quiet window: an unavailable
            # gateway must not masquerade as efficient idle operation.
            for company in companies:
                status, before = plane.owner(company, '/actors/exec/conversation')
                assert status == 200
                replies = sum(message['from_actor'] == 'exec' and
                              'idle verification reply is complete' in message['body']
                              for message in before['messages'])
                plane.raw(company, {'cmd': 'tell', 'body': 'Reply once again after the idle window; no follow-up work is due.'})
                def answered_again():
                    status, view = plane.owner(company, '/actors/exec/conversation')
                    return status == 200 and sum(message['from_actor'] == 'exec' and
                        'idle verification reply is complete' in message['body']
                        for message in view['messages']) > replies
                wait_for(answered_again, 180, f'{company} could not reply after idle')
            assert len(calls()) > baseline, 'Post-idle control did not reach the provider'
            report['postIdleControlCalls'] = len(calls()) - baseline
            report['samples'] = samples
            report['status'] = 'pass'
        except BaseException as error:
            report['status'] = 'fail'
            report['error'] = str(error)
            raise
        finally:
            if plane is not None:
                if plane.log_path.exists():
                    log = plane.log_path.read_text()
                    report['schedulerDisableWarningSeen'] = 'automatic scheduler disabled' in log
                    shutil.copy(plane.log_path, args.output / 'daemon.log')
                    if report['schedulerDisableWarningSeen']:
                        report['status'] = 'fail'
                        problems.append('Scheduler disabled warning was observed')
                problems.extend(plane.close())
            stop(mock)
            mock_stdout.close()
            report['cleanupProblems'] = problems
            report['finishedAt'] = now()
            if problems:
                report['status'] = 'fail'
            (args.output / 'evidence.json').write_text(json.dumps(report, indent=2) + '\n')
            print(json.dumps(report), flush=True)
            # No private DB URL or profile survives the temporary root, including
            # files created with company ownership inside a mounted directory.
            for path in root.rglob('*'):
                try:
                    path.chmod(0o700 if path.is_dir() else 0o600)
                except OSError:
                    pass
    assert report['status'] == 'pass', '; '.join(problems)


if __name__ == '__main__':
    def interrupted(signum, frame):
        raise RuntimeError(f'Interrupted by signal {signum}')
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGALRM, interrupted)
    signal.alarm(1800)
    main()
