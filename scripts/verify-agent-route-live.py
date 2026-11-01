#!/usr/bin/env python3
"""Real local owner status and model picker with synthetic credentials; no provider call.

Requires a built cockpit, a local Core daemon, local company image, local OMP
and PostgreSQL/Docker. Owns and removes one uniquely named _test company.
"""
import argparse, json, os, secrets, signal, subprocess, sys, tempfile, time
from pathlib import Path
repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane

def interrupted(*_):
    raise RuntimeError('fixture interrupted')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--daemon', required=True)
    parser.add_argument('--image', required=True)
    parser.add_argument('--omp', required=True)
    args = parser.parse_args()
    assert (repo / 'web/build/index.html').exists(), 'build the cockpit first'
    assert (repo / 'web/node_modules/playwright').exists(), 'install web dependencies first'
    signal.signal(signal.SIGTERM, interrupted)
    signal.signal(signal.SIGALRM, interrupted)
    signal.alarm(600)
    company = 'route_' + secrets.token_hex(4) + '_test'
    with tempfile.TemporaryDirectory(prefix='s109-agent-route-') as folder:
        plane = Plane(Path(folder), company, args.daemon, args.image, {
            'RESTLESS_COCKPIT_DIR':str(repo / 'web/build'),
            'RESTLESS_OMP_BIN':args.omp,
            'SMOKE_OPENAI':'synthetic-only-key',
        })
        try:
            plane.start_database(); plane.start(); plane.wait_for_port(plane.offset + 7788)
            plane.create_company(company, f'name = "{company}"\nmodel = "anthropic/claude-sonnet-4-6"\nauto_sleep_after_minutes = 0\n[credentials]\n"model.inference.openai" = "env:SMOKE_OPENAI"\n')
            plane.raw(company, {'cmd':'up'}, timeout=300)
            plane.raw(company, {'cmd':'actor-create','as_actor':'site-writer','role':'writer','name':'auto',
                'model':'openai/gpt-example','reason':'Disposable model route test','actor':'owner'})
            for _ in range(120):
                status, view = plane.owner(company, '/intelligence')
                if status == 200 and view.get('company_issue') and any(c.get('provider') == 'openai' and c.get('loaded') for c in view.get('connections', [])):
                    break
                time.sleep(1)
            else:
                raise RuntimeError('test routes did not reach the expected distinct states')
            env = os.environ.copy()
            env.update(RESTLESS_SMOKE_ORIGIN=plane.owner_origin, RESTLESS_SMOKE_COMPANY=company)
            subprocess.run(['node', str(repo / 'web/scripts/verify-agent-route-live.mjs')], cwd=repo / 'web', env=env, check=True, timeout=180)
        finally:
            signal.alarm(0)
            signal.signal(signal.SIGTERM, signal.SIG_IGN)
            problems = plane.close()
            print('cleanup:', json.dumps(problems), flush=True)
            if problems: raise RuntimeError('fixture cleanup failed')

if __name__ == '__main__': main()
