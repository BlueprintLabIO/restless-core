#!/usr/bin/env python3
"""Real ten-minute memory recovery, including an idle late-created company.

Uses disposable `_test` companies, a private database and an explicitly owned
temporary cgroup. No provider credentials, model calls or external effects.
"""
import argparse, json, os, pathlib, subprocess, sys, tempfile, time, urllib.request, uuid
repo=pathlib.Path(__file__).resolve().parent.parent
sys.path.insert(0,str(repo/'scripts/lib'))
from smoke_plane import Plane,docker

def run(*args,check=True,**kw):return subprocess.run(args,capture_output=True,text=True,check=check,**kw)

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output',type=pathlib.Path,required=True)
    parser.add_argument('--image',default='restless-company-image:release-connections')
    args=parser.parse_args();args.output.mkdir(parents=True,exist_ok=True)
    key=uuid.uuid4().hex[:8];namespace=f's111memory_{key}_test'
    parent=f'restless-s111memory-{key}-test.slice';unit=f's111memory_{key}_test.service'
    report={'status':'running'};plane=None
    with tempfile.TemporaryDirectory(prefix='s111-memory-') as directory:
      root=pathlib.Path(directory);(root/'plane').mkdir();(root/'home').mkdir()
      try:
        run('sudo','-n','systemd-run','--unit='+unit,'--slice='+parent,'--property=RuntimeMaxSec=1000','sleep','960')
        run('sudo','-n','systemctl','set-property','--runtime',parent,'CPUQuota=100%','MemoryMax=1610612736','MemoryHigh=1449551462','MemorySwapMax=0')
        group=run('systemctl','show',parent,'-p','ControlGroup','--value').stdout.strip()
        pool=pathlib.Path('/sys/fs/cgroup'+group)
        assert group.endswith('/'+parent) and str(pool).startswith('/sys/fs/cgroup/restless.slice/')
        plane=Plane(root/'plane',namespace,str(repo/'target/debug/restlessd'),args.image,{
          'HOME':str(root/'home'),'RESTLESS_SOURCE_ROOT':str(repo),
          'RESTLESS_WORKLOAD_CGROUP_PARENT':parent,'RESTLESS_WORKLOAD_CGROUP_ROOT':str(pool),
          'RESTLESS_COMPANY_CPUS':'0.5','RESTLESS_COMPANY_MEMORY':'1280m','RESTLESS_COMPUTER_MEMORY':'256m'})
        plane.start_database();plane.start();plane.wait_for_port(plane.offset+7788)
        # Arrive after the watcher's boot-time catalogue has already been read.
        companies=[f's111memory_{key}_{letter}_test' for letter in ('a','b')]
        for company in companies:
          plane.create_company(company,f'name="{company}"\nmodel="anthropic/claude-sonnet-4-6"\nspend_ceiling_usd=1\n')
          plane.raw(company,{'cmd':'up'},timeout=300)
          docker('exec',plane.container(company),'sh','-c','echo preserved-memory-proof > /company/s111-memory-proof')
        def started(company):return docker('inspect','-f','{{.State.StartedAt}}',plane.container(company)).stdout.strip()
        before={c:started(c) for c in companies}
        # An unmarked, idle process retains anonymous RAM; there is no running
        # Work Attempt for the recovery policy to mistake for required demand.
        docker('exec','-d',plane.container(companies[0]),'python3','-c','import time; memory=bytearray(1000*1024*1024); time.sleep(900)')
        began=time.monotonic();notice_at=None;restarted_at=None
        def health():
          with urllib.request.urlopen(plane.owner_origin+'/health',timeout=4) as response:return json.load(response)
        while time.monotonic()-began<780:
          elapsed=time.monotonic()-began
          for company in companies:
            assert plane.owner(company,'/principal')[0]==200, 'Owner entry was closed by another company memory pressure'
          assert started(companies[1])==before[companies[1]], 'The healthy neighbour restarted'
          _,conversation=plane.owner(companies[0],'/actors/exec/conversation')
          # The shortage shows on the company's card; a restart that works is never a chat message.
          assert not any('memory' in m.get('body','').lower() for m in conversation.get('messages',[])), 'A self-healing memory shortage reached the owner chat'
          if health().get('roll_forward',{}).get('companies',{}).get(companies[0],{}).get('outcome')=='waiting_for_memory' and notice_at is None:
            notice_at=elapsed;assert 280<=notice_at<=345, 'Half-deadline card was not timely'
          if started(companies[0])!=before[companies[0]]:
            if restarted_at is None:restarted_at=elapsed
            assert 590<=elapsed<=700, 'Runtime restart did not respect the ten-minute budget'
            if health().get('roll_forward',{}).get('companies',{}).get(companies[0],{}).get('outcome')=='recovered':break
          time.sleep(5)
        assert notice_at is not None and restarted_at is not None, 'Idle retained-memory Runtime never recovered'
        for company in companies:
          assert docker('exec',plane.container(company),'cat','/company/s111-memory-proof').stdout.strip()=='preserved-memory-proof'
        report={'status':'passed','half_deadline_notice_seconds':notice_at,'scoped_restart_seconds':restarted_at,
          'late_created_company_recovered':True,'idle_runtime_recovered':True,'neighbour_not_restarted':True,
          'authenticated_principals_available_throughout':True,'durable_volume_markers_preserved':True}
      finally:
        if plane:
          # Only this fixture's exact disposable names may be removed if the
          # pressure process outlived a failed assertion.
          if report['status']!='passed':
            for company in plane.companies:docker('stop','--time','1',plane.container(company),check=False)
          report['cleanup']=plane.close()
          (args.output/'daemon.log').write_text(plane.log_path.read_text() if plane.log_path.exists() else '')
        run('sudo','-n','systemctl','stop',unit,check=False)
        run('sudo','-n','systemctl','stop',parent,check=False)
        run('sudo','-n','systemctl','revert',parent,check=False)
        (args.output/'evidence.json').write_text(json.dumps(report,indent=2))
    assert report['status']=='passed' and not report.get('cleanup'),report
    print(json.dumps(report))

if __name__=='__main__':main()
