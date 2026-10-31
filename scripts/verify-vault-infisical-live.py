"""Disposable local Infisical and owner API smoke; synthetic values only."""
import concurrent.futures,json,os,signal,socket,subprocess,sys,tempfile,urllib.parse,urllib.request,urllib.error,uuid
from pathlib import Path
repo=Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo/'scripts/lib'))
from smoke_plane import Plane
def run(*args,**kwargs): return subprocess.run(args,check=True,capture_output=True,text=True,**kwargs)
def interrupt(*_): raise RuntimeError('fixture interrupted')
signal.signal(signal.SIGTERM,interrupt);signal.signal(signal.SIGALRM,interrupt);signal.alarm(900)
with tempfile.TemporaryDirectory(prefix='s109-vault-atomic-') as d:
 state=Path(d);namespace='atomic'+uuid.uuid4().hex[:10]+'_test';project='restless-'+namespace+'-vault'
 with socket.socket() as sock:
  sock.bind(('127.0.0.1',0));port=sock.getsockname()[1]
 env=dict(os.environ,RESTLESS_HOME=d,RESTLESS_RESOURCE_NAMESPACE=namespace,RESTLESS_PORT_OFFSET=str(port-7793),STACK_REPO_ROOT=str(repo),RESTLESS_INFISICAL_RUNTIME_ENV=str(state/'infisical/runtime.env'),RESTLESS_INFISICAL_PORT=str(port))
 env.pop('INFISICAL_API_URL',None)
 compose=['docker','compose','--project-name',project,'--file',str(repo/'infra/infisical/compose.yml'),'--env-file',str(state/'infisical/runtime.env')]
 def api(method,path,body=None,token=None):
  headers={'Content-Type':'application/json'}
  if token: headers['Authorization']='Bearer '+token
  req=urllib.request.Request(f'http://127.0.0.1:{port}'+path,data=json.dumps(body).encode() if body is not None else None,headers=headers,method=method)
  try:
   with urllib.request.urlopen(req,timeout=15) as response: return response.status,json.load(response)
  except urllib.error.HTTPError as e: return e.code,{}
 plane=None
 try:
  run('bash','-c','source "$STACK_REPO_ROOT/scripts/lib/dev-vault.sh"; dev_vault_ensure',env=env,timeout=400)
  settings=dict(line.split('=',1) for line in (state/'infisical/authority.env').read_text().splitlines() if '=' in line)
  code,login=api('POST','/api/v1/auth/universal-auth/login',{'clientId':settings['INFISICAL_UNIVERSAL_AUTH_CLIENT_ID'],'clientSecret':settings['INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET'],'organizationSlug':settings['INFISICAL_ORGANIZATION_SLUG']});assert code==200
  token=login['accessToken']; key='ATOMIC_'+uuid.uuid4().hex.upper();base={'projectId':settings['INFISICAL_PROJECT_ID'],'environment':'prod','secretPath':'/','type':'shared'}
  def create(value): return api('POST','/api/v4/secrets/'+key,{**base,'secretValue':value},token)[0]
  with concurrent.futures.ThreadPoolExecutor(max_workers=2) as ex: codes=list(ex.map(create,['synthetic-first','synthetic-second']))
  code,body=api('GET','/api/v4/secrets/'+key+'?'+urllib.parse.urlencode({**base,'viewSecretValue':'true'}),token=token)
  assert sum(200 <= c < 300 for c in codes)==1,(codes,'expected one create')
  assert code==200 and body['secret']['secretValue'] in ['synthetic-first','synthetic-second']
  previous=body['secret']['secretValue']; repeat=create('synthetic-third')
  code,body=api('GET','/api/v4/secrets/'+key+'?'+urllib.parse.urlencode({**base,'viewSecretValue':'true'}),token=token)
  assert not 200<=repeat<300 and code==200 and body['secret']['secretValue']==previous
  print('PASS concurrent provider creation produced one winner and duplicate did not overwrite',flush=True)
  plane_root=state/'plane';plane_root.mkdir()
  company='owner_'+uuid.uuid4().hex[:8]+'_test'
  plane=Plane(plane_root,company,os.environ['RESTLESSD_BIN'],os.environ.get('RESTLESS_COMPANY_IMAGE','restless-company-image:release-connections'),{
   'INFISICAL_API_URL':f'http://127.0.0.1:{port}',
   'INFISICAL_PROJECT_ID':settings['INFISICAL_PROJECT_ID'],
   'INFISICAL_ENVIRONMENT':'prod',
   'INFISICAL_UNIVERSAL_AUTH_CLIENT_ID':settings['INFISICAL_UNIVERSAL_AUTH_CLIENT_ID'],
   'INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET':settings['INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET'],
  })
  plane.start_database();plane.start();plane.wait_for_port(plane.offset+7788)
  plane.create_company(company,f'name = "{company}"\n')
  def revision():
   status,view=plane.owner(company,'/setup');assert status==200
   return view['revision']
  def owner_create(name,value,replace=False):
   return plane.owner(company,'/vault/secret',{'binding':name,'secret':value,'revision':revision(),'create':True,'replace':replace})[0]
  path='/companies/'+company
  scoped={**base,'secretPath':path}
  def get(name):
   status,view=api('GET','/api/v4/secrets/'+name+'?'+urllib.parse.urlencode({**scoped,'viewSecretValue':'true'}),token=token)
   assert status==200
   return view['secret']['secretValue']
  assert owner_create('first','synthetic-first')==200
  assert get('first')=='synthetic-first'
  status,_=api('POST','/api/v4/secrets/unbound',{**scoped,'secretValue':'synthetic-old'},token)
  assert status==200
  assert owner_create('unbound','synthetic-overwrite')==409
  assert get('unbound')=='synthetic-old'
  assert owner_create('unbound','synthetic-confirmed',True)==200
  assert get('unbound')=='synthetic-confirmed'
  status,_=plane.owner(company,'/vault/secret/rename',{'reference':'infisical:'+path+'/first','name':'renamed','revision':revision()})
  assert status==200
  assert get('renamed')=='synthetic-first'
  print('PASS real owner API created, protected, explicitly replaced and renamed real Infisical secrets',flush=True)
 finally:
  problems=[]
  if plane:
   try: problems.extend(plane.close())
   except Exception as error: problems.append(str(error))
  try:
   if (state/'infisical/runtime.env').exists(): run(*compose,'down','--volumes',env=env,timeout=90)
  except Exception as error: problems.append('vault cleanup: '+str(error))
  print('cleanup:',json.dumps(problems),flush=True)
  if problems: raise RuntimeError('fixture cleanup failed')
