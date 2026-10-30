#!/usr/bin/env python3
"""Exercise a scheduled tool call through a local-only mock provider."""
import argparse, json, os, secrets, shutil, signal, subprocess, sys, tempfile, time, urllib.request
from pathlib import Path
repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane, free_port

def wait_for(check, seconds, reason):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        value = check()
        if value: return value
        time.sleep(.5)
    raise AssertionError(reason)

def interrupted(*_):
    raise RuntimeError('fixture interrupted')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--daemon', required=True)
    parser.add_argument('--image', required=True)
    args = parser.parse_args()
    assert (repo / 'infra/host-tools/node_modules').is_dir(), 'Install host-tool dependencies first'
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGALRM, interrupted)
    signal.alarm(900)
    with tempfile.TemporaryDirectory(prefix='s109-pi-tool-wire-') as folder:
        root = Path(folder); (root/'plane').mkdir(); (root/'home').mkdir()
        log = root/'requests.jsonl'; scenario = root/'scenario.json'; port = free_port()
        scenario.write_text(json.dumps({'rules':[{'match':'.*','steps':[{'tool':{'name':'bash','input':{'i':'Verify local tool','command':'printf s109_tool_roundtrip_ok'}}},{'text':'The local tool roundtrip finished.'}]}]}))
        source = (repo/'tools/mock-model/server.mjs').read_text()
        original = "const MODELS = ['claude-sonnet-5-5', 'claude-opus-5-5', 'claude-haiku-4-5-20251001'];"
        assert original in source
        mock_script = root/'mock.mjs'
        mock_script.write_text(source.replace(original, "const MODELS = ['claude-sonnet-4-6'];", 1))
        mock_log = (root/'mock.log').open('w')
        mock = subprocess.Popen(['node', str(mock_script), '--port', str(port), '--scenario', str(scenario), '--log', str(log)], stdout=mock_log, stderr=subprocess.STDOUT, start_new_session=True)
        suffix = secrets.token_hex(3); company = f'piwire_{suffix}_test'
        plane = Plane(root/'plane', company, args.daemon, args.image, {
            'HOME':str(root/'home'), 'RESTLESS_TEST_DISABLE_SCHEDULER':'0',
            'RESTLESS_SOURCE_ROOT':str(repo), 'RESTLESS_OMP_BIN':str(repo/'infra/host-tools/omp-source'),
            'RESTLESS_NODE_BIN':shutil.which('node'), 'RESTLESS_ANTHROPIC_BASE_URL':f'http://127.0.0.1:{port}/v1',
            'SMOKE_RECOVERY_KEY':'synthetic-key', 'RESTLESS_COMPANY_CPUS':'0.5',
            'RESTLESS_COMPANY_MEMORY':'768m', 'RESTLESS_COMPUTER_MEMORY':'768m', 'RUST_LOG':'info'})
        def ready():
            try:
                with urllib.request.urlopen(f'http://127.0.0.1:{port}/v1/models', timeout=2) as response: return response.status == 200
            except OSError: return False
        def calls():
            rows = [json.loads(row) for row in log.read_text().splitlines()] if log.exists() else []
            return [row for row in rows if row['method']=='POST' and row['path']=='/v1/messages']
        try:
            wait_for(ready, 15, 'mock provider unavailable')
            plane.start_database(); plane.start()
            plane.create_company(company, f'name = "{company}"\nmodel = "anthropic/claude-sonnet-4-6"\nmission = "Wait for owner requests; do not invent projects."\nspend_ceiling_usd = 1\nauto_sleep_after_minutes = 0\n[credentials]\n"model.inference" = "env:SMOKE_RECOVERY_KEY"\n')
            plane.raw(company, {'cmd':'up'}, timeout=600)
            plane.wait_for_port(plane.relay_port, 120)
            ready_seen = [None, time.monotonic()]
            def gateway_settled():
                text = plane.log_path.read_text(); latest=text.rfind('model gateway ready')
                if latest < 0 or latest < text.rfind('reloading account model services'):
                    ready_seen[:] = [None,time.monotonic()]; return False
                if latest != ready_seen[0]: ready_seen[:] = [latest,time.monotonic()]
                return time.monotonic()-ready_seen[1] >= 12
            wait_for(gateway_settled,120,'model gateway did not settle')
            for actor, role in [('wait-direction', 'lead'), ('wait-writer', 'writer')]:
                plane.raw(company, {'cmd':'actor-create', 'as_actor':actor, 'role':role, 'name':'auto',
                    'model':'anthropic/claude-sonnet-4-6', 'reason':'Isolated tool wire test', 'actor':'owner'})
            plane.raw(company, {'cmd':'team-create','name':'Wait team','to':'wait-direction','body':'Verify tool wire','actor':'owner'})
            plane.raw(company, {'cmd':'team-assign','as_actor':'wait-writer','name':'Wait team','actor':'owner','reason':'Isolated tool wire test'})
            before=len(calls())
            work=plane.raw(company, {'cmd':'work-add','actor':'wait-writer','role':'writer',
                'title':'wire-roundtrip','body':'Run the local wire-roundtrip check.', 'as_actor':'wait-direction'})['work_id']
            def blocked():
                rows=plane.raw(company, {'cmd':'work-attempts','id':work})
                terminal = {'blocked', 'produced', 'failed', 'interrupted', 'cancelled'}
                return rows if rows and rows[-1].get('state') in terminal else None
            attempts=wait_for(blocked, 180, 'the scheduled attempt did not finish or block')
            provider_rows = [json.loads(row) for row in log.read_text().splitlines() if row]
            provider_rows = [row for row in provider_rows if row['method'] == 'POST' and row['path'] == '/v1/messages'][before:]
            tool_results = [block for row in provider_rows for message in row.get('body', {}).get('messages', []) for block in (message.get('content') if isinstance(message.get('content'), list) else []) if isinstance(block, dict) and block.get('type') == 'tool_result']
            assert provider_rows, 'provider was not called'
            assert attempts[-1]['state'] == 'produced', f"work did not finish: {attempts[-1]['state']}"
            assert any('s109_tool_roundtrip_ok' in json.dumps(result.get('content')) and not result.get('is_error', False) for result in tool_results), 'successful tool output was not returned to the provider'
            print(json.dumps({'result':'scheduled tool roundtrip reached the provider', 'attemptState': attempts[-1]['state'], 'providerCalls':len(provider_rows), 'toolResultObserved':True}),flush=True)
        finally:
            print('mock provider calls:', len(calls()), flush=True)
            signal.alarm(0)
            signal.signal(signal.SIGTERM, signal.SIG_IGN)
            signal.signal(signal.SIGALRM, signal.SIG_IGN)
            problems=plane.close(); print('cleanup:',json.dumps(problems),flush=True)
            if mock.poll() is None:
                os.killpg(mock.pid,signal.SIGTERM)
                try: mock.wait(timeout=10)
                except subprocess.TimeoutExpired: os.killpg(mock.pid,signal.SIGKILL); mock.wait(timeout=10)
            mock_log.close()
            if problems: raise RuntimeError('fixture cleanup failed')
if __name__ == '__main__': main()
