"""Smoke a stopped test company's replacement; all resources are disposable and local."""
import os, secrets, signal, subprocess, sys, tempfile
from pathlib import Path
repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane

def run(*args, check=True, **kwargs):
    return subprocess.run(args, check=check, text=True, capture_output=True, timeout=60, **kwargs)

def interrupt(*_): raise RuntimeError('fixture interrupted')
signal.signal(signal.SIGTERM, interrupt)
signal.signal(signal.SIGALRM, interrupt)
signal.alarm(600)
nonce=secrets.token_hex(3)
company='stopped_'+nonce+'_test'
slice_name='restless-s109stopped'+nonce+'.slice'
unit=Path('/run/systemd/system')/slice_name
assert not unit.exists()
original=os.environ.get('RESTLESS_COMPANY_IMAGE','restless-company-image:release-connections')
alternate=os.environ['RESTLESS_TEST_ALTERNATE_IMAGE']
test_image='restless-stoppedfixture:'+nonce
with tempfile.TemporaryDirectory(prefix='s109-stopped-reconcile-') as folder:
    root=Path(folder)
    for name in ['home','plane','bin']: (root/name).mkdir()
    (root/'bin/getconf').write_text('#!/bin/sh\necho 4\n'); (root/'bin/getconf').chmod(0o755)
    (root/'meminfo').write_text('MemTotal: 8388608 kB\n')
    source=(Path(os.environ['RESTLESS_CLOUD_DIR'])/'services/vm-provider/agent/restless-agent.sh').read_text()
    start,end=source.index('install_workload_budget() {'),source.index('\nupdate_agent()')
    setup=root/'setup.sh'; setup.write_text('log(){ :; }\n'+source[start:end]+'\ninstall_workload_budget '+slice_name+'\n')
    pool=Path('/sys/fs/cgroup/restless.slice')/slice_name
    plane=None; own_unit=False; tagged=False
    try:
        old=run('docker','image','inspect','-f','{{.Id}}',original).stdout.strip()
        new=run('docker','image','inspect','-f','{{.Id}}',alternate).stdout.strip()
        assert old != new
        run('docker','tag',original,test_image); tagged=True
        path=str(root/'bin')+':'+os.environ['PATH']; own_unit=True
        run('sudo','-n','env','PATH='+path,'RESTLESS_HOST_MEMINFO='+str(root/'meminfo'),
            'RESTLESS_WORKLOAD_UNIT_DIR=/run/systemd/system','bash',str(setup))
        plane=Plane(root/'plane',company,os.environ['RESTLESSD_BIN'],test_image,{
            'HOME':str(root/'home'),'PATH':path,'RESTLESS_OMP_BIN':str(repo/'infra/host-tools/omp-source'),
            'RESTLESS_SOURCE_ROOT':str(repo),'RESTLESS_COMPANY_CPUS':'1','RESTLESS_COMPANY_MEMORY':'1024m',
            'RESTLESS_WORKLOAD_CGROUP_PARENT':slice_name,'RESTLESS_WORKLOAD_CGROUP_ROOT':str(pool)})
        plane.start_database();plane.start();plane.wait_for_port(plane.offset+7788)
        plane.create_company(company,'name = "'+company+'"\nauto_sleep_after_minutes = 0\n')
        plane.raw(company,{'cmd':'up'},timeout=300)
        container=plane.container(company)
        run('docker','exec','-u','company',container,'sh','-c','printf retained > /company/run/stopped-upgrade-test')
        before=run('docker','inspect','-f','{{.Id}}',container).stdout.strip()
        plane.raw(company,{'cmd':'down'},timeout=120)
        assert run('docker','inspect','-f','{{.State.Status}}',container).stdout.strip() != 'running'
        run('docker','tag',alternate,test_image)
        plane.raw(company,{'cmd':'up','reconcile':True},timeout=300)
        after=run('docker','inspect','-f','{{.Id}}',container).stdout.strip()
        assert before != after
        assert run('docker','inspect','-f','{{.Image}}',container).stdout.strip() == new
        assert run('docker','exec','-u','company',container,'cat','/company/run/stopped-upgrade-test').stdout == 'retained'
        assert plane.owner(company,'/principal')[0] == 200
        print('stopped Runtime reconciled to a different local image and retained its files',flush=True)
    finally:
        errors=[]
        if plane:
            try: errors.extend(plane.close())
            except Exception as error: errors.append(str(error))
            print('plane cleanup:',errors,flush=True)
        if own_unit and unit.exists():
            if pool.exists() and any(p.read_text().strip() for p in pool.rglob('cgroup.procs')):
                raise RuntimeError('owned slice still has processes; leaving it intact')
            run('sudo','-n','systemctl','stop',slice_name)
            assert unit.parent==Path('/run/systemd/system') and unit.name==slice_name
            run('sudo','-n','rm','-f','--',str(unit))
            run('sudo','-n','rm','-rf','--',str(unit)+'.d')
            run('sudo','-n','systemctl','daemon-reload')
            print('temporary slice removed:',not unit.exists(),flush=True)
        if tagged: run('docker','image','rm',test_image,check=False)
        if errors: raise RuntimeError('fixture cleanup failed')
