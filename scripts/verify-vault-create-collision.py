#!/usr/bin/env python3
"""Exercise Vault create conflicts against a local-only fake Vault and real owner API."""
import argparse, json, os, secrets, signal, sys, tempfile, threading
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlsplit, unquote
repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane

class FakeVault(BaseHTTPRequestHandler):
    values = {'resend.production': 'synthetic-old-value'}
    writes = 0
    fail_reads = False
    def log_message(self, *_): pass
    def reply(self, status, body):
        data = json.dumps(body).encode()
        self.send_response(status); self.send_header('Content-Type','application/json')
        self.send_header('Content-Length',str(len(data))); self.end_headers(); self.wfile.write(data)
    def body(self): return json.loads(self.rfile.read(int(self.headers.get('Content-Length','0'))) or b'{}')
    def do_POST(self):
        path = urlsplit(self.path).path
        if path == '/api/v1/auth/universal-auth/login': return self.reply(200, {'accessToken':'synthetic-token'})
        if path == '/api/v2/folders': return self.reply(200, {})
        return self.reply(404, {})
    def do_GET(self):
        path = urlsplit(self.path).path
        if path.startswith('/api/v4/secrets/'):
            if self.fail_reads: return self.reply(503,{})
            name = unquote(path.rsplit('/',1)[-1]); value = self.values.get(name)
            return self.reply(404,{}) if value is None else self.reply(200,{'secret':{'secretValue':value}})
        return self.reply(404,{})
    def do_PATCH(self):
        if urlsplit(self.path).path != '/api/v4/secrets/batch': return self.reply(404,{})
        body=self.body(); self.__class__.writes += 1
        for item in body['secrets']: self.values[item['secretKey']] = item['secretValue']
        return self.reply(200,{})

def interrupt(*_): raise RuntimeError('fixture interrupted')

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--daemon',required=True);parser.add_argument('--image',required=True)
    args=parser.parse_args()
    signal.signal(signal.SIGTERM,interrupt);signal.signal(signal.SIGALRM,interrupt);signal.alarm(300)
    fake=ThreadingHTTPServer(('127.0.0.1',0),FakeVault)
    threading.Thread(target=fake.serve_forever,daemon=True).start()
    company='vault_'+secrets.token_hex(4)+'_test'
    with tempfile.TemporaryDirectory(prefix='s109-vault-collision-') as tmp:
        plane=Plane(Path(tmp),company,args.daemon,args.image,{
            'INFISICAL_API_URL':f'http://127.0.0.1:{fake.server_port}',
            'INFISICAL_PROJECT_ID':'synthetic-project','INFISICAL_ENVIRONMENT':'test',
            'INFISICAL_UNIVERSAL_AUTH_CLIENT_ID':'synthetic-client',
            'INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET':'synthetic-secret',
        })
        try:
            plane.start_database();plane.start();plane.wait_for_port(plane.offset+7788)
            plane.create_company(company,f'name = "{company}"\n')
            def revision():
                status, view = plane.owner(company,'/setup'); assert status == 200, status
                return view['revision']
            def create(name, value, replace=False):
                return plane.owner(company,'/vault/secret',{'binding':name,'secret':value,'revision':revision(),'create':True,'replace':replace})
            code,_=create('resend','synthetic-new-value')
            assert code == 409, f'unbound existing secret was not protected: {code}'
            assert FakeVault.values['resend.production']=='synthetic-old-value' and FakeVault.writes==0
            FakeVault.fail_reads=True
            code,_=create('another','synthetic-new-value')
            assert code == 503 and FakeVault.writes==0, f'unavailable Vault did not fail closed: {code}'
            FakeVault.fail_reads=False
            code,_=create('another','synthetic-new-value')
            assert code == 200 and FakeVault.values['another']=='synthetic-new-value'
            code,_=create('resend','synthetic-confirmed-value',True)
            assert code == 200 and FakeVault.values['resend.production']=='synthetic-confirmed-value'
            print('PASS unbound collision preserved; unavailable lookup failed closed; create and confirmed replace worked',flush=True)
        finally:
            signal.alarm(0);signal.signal(signal.SIGTERM,signal.SIG_IGN)
            problems=plane.close();fake.shutdown();fake.server_close()
            print('cleanup:',json.dumps(problems),flush=True)
            if problems: raise RuntimeError('fixture cleanup failed')

if __name__=='__main__': main()
