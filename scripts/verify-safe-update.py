#!/usr/bin/env python3
"""Sprint 111: real Docker canary failure, restoration and real scripted Exec.

Only isolated `_test` companies, a private database and a local scripted model
are used. No production model, credentials, company or registry repository.
"""
import argparse, datetime, hashlib, json, os, signal, subprocess, sys, tempfile, time, urllib.request, uuid
from pathlib import Path
repo = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane, docker, free_port

def wait(check, seconds, message):
    end = time.monotonic() + seconds
    while time.monotonic() < end:
        value = check()
        if value: return value
        time.sleep(.5)
    raise AssertionError(message)

def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument('--image', default='restless-company-image:release-connections')
    ap.add_argument('--output', type=Path, required=True)
    args = ap.parse_args();args.output.mkdir(parents=True, exist_ok=True)
    key = uuid.uuid4().hex[:8];namespace = f's111_{key}_test'
    daemon = os.environ.get('RESTLESSD_BIN', str(repo/'target/debug/restlessd'))
    tags = [];manifests = [];plane = model = None
    report = {'started_at':datetime.datetime.now(datetime.timezone.utc).isoformat(), 'status':'running'}
    with tempfile.TemporaryDirectory(prefix='s111-update-') as directory:
        root = Path(directory)
        try:
            # A local-only repository is excluded from release mirroring.
            for kind in ('old','broken','good'):
                tag = f'127.0.0.1:5055/test/s111-{key}:{kind}'
                if kind == 'old': docker('tag', args.image, tag)
                else:
                    data = f'FROM {args.image}\nLABEL s111.test={kind}\n'
                    if kind == 'broken': data += 'ENTRYPOINT ["/bin/sh","-c","exit 23"]\n'
                    subprocess.run(['docker','build','-t',tag,'-'],input=data,text=True,check=True,stdout=subprocess.DEVNULL)
                tags.append(tag);docker('push',tag)
                refs = json.loads(docker('image','inspect','--format','{{json .RepoDigests}}',tag).stdout)
                ref = next(ref for ref in refs if ref.startswith(f'127.0.0.1:5055/test/s111-{key}@'))
                manifests.append(ref)
            port = free_port();scenario = root/'scenario.json';requests = root/'requests.jsonl'
            scenario.write_text(json.dumps({'rules':[{'match':'.*','steps':[{'text':'Platform Exec check passed.','delay_ms':100}]}]}))
            model_log = (args.output/'model.log').open('w')
            model = subprocess.Popen(['node',str(repo/'tools/mock-model/server.mjs'),'--port',str(port),'--scenario',str(scenario),'--log',str(requests)],stdout=model_log,stderr=subprocess.STDOUT,start_new_session=True)
            def model_ready():
                try:
                    with urllib.request.urlopen(f'http://127.0.0.1:{port}/v1/models',timeout=2) as r:return r.status==200
                except OSError:return False
            wait(model_ready,15,'Scripted model did not start')
            (root/'plane').mkdir();(root/'home').mkdir()
            plane = Plane(root/'plane',namespace,daemon,manifests[0],{
                'HOME':str(root/'home'),'RESTLESS_SOURCE_ROOT':str(repo),
                'RESTLESS_OMP_BIN':str(repo/'infra/host-tools/omp-source'),
                'RESTLESS_NODE_BIN':subprocess.check_output(['which','node'],text=True).strip(),
                'RESTLESS_ANTHROPIC_BASE_URL':f'http://127.0.0.1:{port}/v1',
                'S111_MODEL_KEY':'scripted-test-only','RESTLESS_COMPANY_CPUS':'1',
                'RESTLESS_COMPANY_MEMORY':'1536m','RESTLESS_COMPUTER_MEMORY':'512m'})
            plane.start_database();plane.start();plane.wait_for_port(plane.offset+7788)
            companies = [f's111_{key}_{letter}_test' for letter in ('a','b')]
            for company in companies:
                plane.create_company(company,f'name="{company}"\nmodel="anthropic/claude-opus-5-5"\nmission="Answer the platform canary only. Do not create projects."\nspend_ceiling_usd=10\n[credentials]\n"model.inference"="env:S111_MODEL_KEY"\n')
                plane.raw(company,{'cmd':'up'},timeout=600)
                docker('exec',plane.container(company),'sh','-c','echo durable-proof > /company/s111-proof')
            def image(company):return docker('inspect','-f','{{.Image}}',plane.container(company)).stdout.strip()
            old = {c:image(c) for c in companies}
            def health():
                try:
                    with urllib.request.urlopen(plane.owner_origin+'/health',timeout=3) as r:return json.load(r)
                except OSError:return {}
            plane.stop();plane.image=manifests[1];plane.start()
            failed = wait(lambda:(h if (h:=health()).get('roll_forward',{}).get('stage',{}).get('stage')=='canary_failed' else None),240,'Broken canary did not stop the rollout')
            assert {c:image(c) for c in companies}==old, 'A broken canary changed another company or did not restore its own image'
            for c in companies:assert docker('exec',plane.container(c),'cat','/company/s111-proof').stdout.strip()=='durable-proof'
            report['broken_canary']={'pass':True,'health':failed,'images':old}
            plane.stop();plane.image=manifests[2];plane.start()
            good = wait(lambda:(h if (h:=health()).get('roll_forward',{}).get('stage',{}).get('stage')=='done' else None),300,'Healthy canary did not complete')
            target = docker('image','inspect','--format','{{.Id}}',manifests[2]).stdout.strip()
            assert all(image(c)==target for c in companies), 'A company remained on the previous image'
            calls = [json.loads(line) for line in requests.read_text().splitlines()]
            assert any(c['method']=='POST' and c['path']=='/v1/messages' for c in calls), 'No real scripted Exec inference ran'
            for c in companies:
                assert plane.owner(c,'/principal')[0]==200
                assert docker('exec',plane.container(c),'cat','/company/s111-proof').stdout.strip()=='durable-proof'
            report['healthy_canary']={'pass':True,'health':good,'scripted_model_calls':len(calls)}
            report['status']='passed'
        finally:
            if plane:
                (args.output/'daemon.log').write_text(plane.log_path.read_text() if plane.log_path.exists() else '')
                report['cleanup']=plane.close()
            if model and model.poll() is None:
                os.killpg(model.pid,signal.SIGTERM)
                try:model.wait(timeout=10)
                except subprocess.TimeoutExpired:os.killpg(model.pid,signal.SIGKILL);model.wait()
            for ref in manifests:
                try:
                    request=urllib.request.Request('http://127.0.0.1:5055/v2/test/s111-'+key+'/manifests/'+ref.split('@')[1],method='DELETE')
                    urllib.request.urlopen(request,timeout=5).close()
                except OSError:pass
            for tag in tags:docker('image','rm',tag,check=False)
            (args.output/'evidence.json').write_text(json.dumps(report,indent=2))
    assert report['status']=='passed' and not report.get('cleanup'),report
    print(json.dumps({'status':'passed','evidence':str(args.output/'evidence.json')}))

if __name__=='__main__':main()
