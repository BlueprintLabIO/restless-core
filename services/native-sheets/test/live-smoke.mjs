// Opt-in test against a disposable, running company. Both collaborators use
// real upstream Models; the Rust service and Postgres accept every revision.
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { Model } from '../src/upstream.mjs';
import WebSocket from 'ws';

const origin=process.env.RESTLESS_SHEETS_SMOKE_URL;
const company=process.env.RESTLESS_SHEETS_SMOKE_COMPANY;
assert(origin && company?.endsWith('_test'),'set an explicit disposable *_test company and owner URL');
const root=`${origin}/api/companies/${encodeURIComponent(company)}/sheets`;
const request=async(path='',body,method=body?'POST':'GET')=>{
  const response=await fetch(root+path,{method,headers:{'content-type':'application/json',origin},body:body?JSON.stringify(body):undefined});
  const value=await response.json();assert(response.ok,`${response.status}: ${JSON.stringify(value)}`);return value;
};
async function until(condition,label) {
  const end=Date.now()+15000;
  while(!condition()){assert(Date.now()<end,`timeout: ${label}`);await new Promise(r=>setTimeout(r,50));}
}
const id=randomUUID();
const created=await request('',{id,title:'Sydney resale smoke',visibility:'company'});
await request(`/${id}/operations`,{expected_revision:created.head_revision,key:randomUUID(),action:{action:'set_cells',values:[
  ['record_id','Item','Buy','Sell','Fees','Profit','ROI'],
  ['gfx','GFX 50S II',2100,3200,100,'=D2-C2-E2','=F2/C2'],
  ['gpu','RTX 3090',800,1150,50,'=D3-C3-E3','=F3/C3']
]}});
const clients=[];
function collaborator() {
  const c={cursor:0,pending:new Map(),model:null,socket:null,receive:null,clientId:'',token:'',error:null,ready:false};
  const transport={onNewMessage(_id,callback){c.receive=callback;},async sendMessage(message){
    if(message.type==='SNAPSHOT')return;
    if(message.nextRevisionId)c.pending.set(message.nextRevisionId,message);
    if(c.socket?.readyState===WebSocket.OPEN)c.socket.send(JSON.stringify(message));
  },leave(){}};
  c.connect=()=>{
    const url=new URL(`${root}/${id}/collaboration`);url.protocol=url.protocol.replace('http','ws');
    if(c.model){url.searchParams.set('after',c.cursor);url.searchParams.set('client_id',c.clientId);url.searchParams.set('reconnect_token',c.token);}
    c.ready=false;c.socket=new WebSocket(url,{origin});c.socket.on('error',error=>c.error=error.message);
    c.socket.addEventListener('message',event=>{
      const envelope=JSON.parse(event.data);
      if(envelope.type==='ERROR'){c.error=envelope.message;return;}
      if(envelope.type==='BOOTSTRAP'){
        const state=envelope.state;
        if(!c.model){c.clientId=envelope.client_id;c.token=envelope.reconnect_token;c.model=new Model(state.snapshot,{transportService:transport,client:{id:c.clientId,name:'Smoke'}},state.messages.map(m=>m.message));c.cursor=state.sheet.sequence;}
        else for(const m of state.messages){if(m.sequence>c.cursor){c.receive(m.message);c.pending.delete(m.message.nextRevisionId);c.cursor=m.sequence;}}
        for(const message of c.pending.values())c.socket.send(JSON.stringify(message));c.ready=true;
      } else if(envelope.type==='MESSAGE' && envelope.sequence>c.cursor){c.receive(envelope.message);c.pending.delete(envelope.message.nextRevisionId);c.cursor=envelope.sequence;}
    });
  };
  c.connect();clients.push(c);return c;
}
try {
  const a=collaborator(),b=collaborator();await until(()=>a.ready&&b.ready,'two collaboration bootstraps');
  assert.notEqual(a.clientId,b.clientId,'distinct Models receive distinct server-owned IDs');
  const sheetId=a.model.getters.getActiveSheetId();
  assert(a.model.dispatch('ADD_COLUMNS_ROWS',{sheetId,dimension:'ROW',position:'before',base:1,quantity:1}).isSuccessful);
  assert(b.model.dispatch('UPDATE_CELL',{sheetId,col:3,row:1,content:'3100'}).isSuccessful);
  await until(()=>!a.pending.size&&!b.pending.size&&a.cursor===b.cursor,'structural collision settles');
  assert.equal(a.error,null);assert.equal(b.error,null);
  for(const c of [a,b]){
    assert.equal(c.model.getters.getEvaluatedCell({sheetId,col:1,row:2}).value,'GFX 50S II');
    assert.equal(c.model.getters.getEvaluatedCell({sheetId,col:3,row:2}).value,3100);
    assert.equal(c.model.getters.getEvaluatedCell({sheetId,col:5,row:2}).value,900);
  }
  let state=await request(`/${id}`);
  const key=randomUUID(), expected_revision=state.sheet.head_revision;
  const action={action:'update_record',range:'A1:G4',record_id:'gfx',values:{Sell:3150}};
  const receipt=await request(`/${id}/operations`,{key,expected_revision,action});
  await until(()=>a.model.getters.getEvaluatedCell({sheetId,col:5,row:2}).value===950&&b.model.getters.getEvaluatedCell({sheetId,col:5,row:2}).value===950,'human models receive headless edits');
  a.socket.close();await new Promise(r=>setTimeout(r,250));
  state=await request(`/${id}`);
  await request(`/${id}/operations`,{key:randomUUID(),expected_revision:state.sheet.head_revision,action:{action:'set_cells',start:'A101',values:[['tail','CSV row 101']]}});
  a.connect();await until(()=>a.ready&&a.cursor===b.cursor,'disconnected collaborator catches up');
  assert.equal(a.model.getters.getEvaluatedCell({sheetId,col:1,row:100}).value,'CSV row 101');
  assert.deepEqual(await request(`/${id}/operations`,{key,expected_revision,action}),receipt,'lost receipt returns after subsequent edits');
  const exported=await request(`/${id}/operations`,{action:{action:'export_csv'}});
  assert.match(exported.result.csv,/tail,CSV row 101/);
  const recovered=collaborator();await until(()=>recovered.ready,'fresh Model recovers persisted log');
  assert.equal(recovered.model.getters.getEvaluatedCell({sheetId,col:5,row:2}).value,950);
  let bootstrapped=false;
  const invalid=new WebSocket(`${root.replace(/^http/,'ws')}/${id}/collaboration?after=999999`,{origin});
  invalid.addEventListener('message',()=>bootstrapped=true);invalid.addEventListener('error',()=>{});
  await new Promise(r=>setTimeout(r,500));invalid.close();assert(!bootstrapped);
  console.log(JSON.stringify({ok:true,company,sheet:id,revision:recovered.model.exportData().revisionId,checks:['upstream structural OT','human/headless convergence','disconnected catchup','lost-response idempotency','full-range CSV','fresh persisted replay','ahead cursor rejection']}));
} finally {for(const c of clients)c.socket?.close();}
