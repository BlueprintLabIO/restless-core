import assert from 'node:assert/strict';
import {createServer,request as httpRequest} from 'node:http';
import {randomBytes, randomUUID} from 'node:crypto';
import {spawn} from 'node:child_process';
import {readFile,writeFile,mkdir,access} from 'node:fs/promises';
import {openSync,closeSync} from 'node:fs';
import {configure} from '../scripts/configure.mjs';
import {readConfig} from '../src/config.mjs';
import {createIdentityService} from '../src/server.mjs';
import {HocuspocusProvider,HocuspocusProviderWebsocket} from '@hocuspocus/provider';
import pg from 'pg';
import WS from 'ws';
import * as Y from 'yjs';
const {RESTLESS_HOME:root,RESTLESS_AUTH_TEST_PORT:authPort,RESTLESS_PORT_OFFSET:offset,RESTLESS_TEST_COMPANY:company}=process.env;
assert(company.endsWith('_test'));
const origin=`http://127.0.0.1:${authPort}`, core=`http://127.0.0.1:${Number(offset)+7788}`, coreHost='plane.localhost';

const messages=[];
const clients=[];
const traceSockets=new Set();
let ownerId=randomUUID(),planeId=randomUUID();
let server,daemon,cellPool,identity,service,serviceConfig,traceProxy;
const ownerPort=Number(offset)+7788+45;
const logFd=openSync(process.env.RESTLESS_TEST_DAEMON_LOG,'w');
const pause=ms=>new Promise(resolve=>setTimeout(resolve,ms));
async function until(fn,label,seconds=90){const end=Date.now()+seconds*1000;while(Date.now()<end){if(await fn())return;await pause(500);}throw Error(`Timed out: ${label}`);}
async function coreCall(path,body,cookie='',expected=200,method){
 const response=await new Promise((resolve,reject)=>{
  const request=httpRequest(core+path,{method:method??(body===undefined?'GET':'POST'),headers:{Host:coreHost,Origin:`http://${coreHost}`,'Content-Type':'application/json',Cookie:cookie,'Idempotency-Key':randomUUID()}},res=>{
   const chunks=[];res.on('data',chunk=>chunks.push(chunk));res.on('end',()=>resolve({status:res.statusCode,headers:res.headers,text:Buffer.concat(chunks).toString()}));
  });request.setTimeout(10000,()=>request.destroy(Error('Core request timed out')));request.on('error',reject);request.end(body===undefined?undefined:JSON.stringify(body));
 });
 assert.equal(response.status,expected,`${path}: ${response.text.slice(0,350)}`);
 return {data:response.text?JSON.parse(response.text):null,cookie:(response.headers['set-cookie']??[]).map(item=>item.split(';')[0]).join('; ')};
}
async function stopDaemon(){
 if(!daemon||daemon.exitCode!==null||daemon.signalCode!==null)return;
 await new Promise(resolve=>{const timer=setTimeout(()=>{daemon.kill('SIGKILL');resolve();},10000);daemon.once('exit',()=>{clearTimeout(timer);resolve();});daemon.kill('SIGTERM');});
}
async function configureCli(args){
 return await new Promise((resolve,reject)=>{
  const child=spawn(process.execPath,['scripts/configure.mjs',...args],{cwd:process.env.RESTLESS_TEST_REPO+'/services/self-hosted-identity',stdio:['ignore','pipe','pipe']});let stderr='';
  child.stdout.resume();child.stderr.on('data',chunk=>{if(stderr.length<8192)stderr+=chunk.toString().slice(0,8192-stderr.length);});
  child.once('error',reject);child.once('exit',(code,signal)=>resolve({code,signal,stderr}));
 });
}
function startDaemon(network){daemon=spawn(process.env.RESTLESS_TEST_DAEMON,[],{cwd:process.env.RESTLESS_TEST_REPO,env:{...process.env,RESTLESS_OWNER_ADDR:`127.0.0.1:${ownerPort}`,RESTLESS_ENTRY_MODE:network?'network':'local',RESTLESS_RUNTIME_MODE:'local',RESTLESS_ENTRY_ALLOW_INSECURE_HTTP:'1',RESTLESS_ENTRY_ISSUER:origin,RESTLESS_ENTRY_JWKS_URL:origin+'/.well-known/jwks.json',RESTLESS_ENTRY_OWNER_ID:ownerId,RESTLESS_ENTRY_PLANE_ID:planeId,RESTLESS_ENTRY_HOST:coreHost},stdio:['ignore',logFd,logFd]});}
try{
 traceProxy=createServer((req,res)=>{
  const upstream=httpRequest(`http://127.0.0.1:${ownerPort}${req.url}`,{method:req.method,headers:req.headers},response=>{
   if(req.url==='/entry'||req.url===`/${company}`)console.log('NAVIGATION',req.method,req.url,{site:req.headers['sec-fetch-site'],mode:req.headers['sec-fetch-mode'],dest:req.headers['sec-fetch-dest'],origin:req.headers.origin,hasCookie:!!req.headers.cookie,status:response.statusCode});
   res.writeHead(response.statusCode,response.headers);response.pipe(res);
  });upstream.on('error',()=>{res.writeHead(502);res.end();});req.pipe(upstream);
 });
 traceProxy.on('connection',socket=>{traceSockets.add(socket);socket.once('close',()=>traceSockets.delete(socket));});
 traceProxy.on('upgrade',(req,client,head)=>{
  const upstream=httpRequest(`http://127.0.0.1:${ownerPort}${req.url}`,{method:req.method,headers:req.headers});
  upstream.on('upgrade',(res,socket,incoming)=>{client.write(`HTTP/1.1 ${res.statusCode} ${res.statusMessage}\r\n`+res.rawHeaders.reduce((s,v,i,a)=>i%2?s:s+a[i]+': '+a[i+1]+'\r\n','')+'\r\n');if(incoming.length)client.write(incoming);if(head.length)socket.write(head);socket.pipe(client);client.pipe(socket);client.on('error',()=>socket.destroy());socket.on('error',()=>client.destroy());client.on('close',()=>socket.destroy());});upstream.on('error',()=>client.destroy());upstream.end();
 });
 await new Promise(resolve=>traceProxy.listen(Number(offset)+7788,'127.0.0.1',resolve));
 startDaemon(false);
 await until(async()=>{if(daemon.exitCode!==null)throw Error('Core exited; inspect isolated daemon log');try{cellPool??=new pg.Pool({connectionString:(await readFile(`${root}/cells/${company}/database.url`,'utf8')).trim(),max:2});const result=await cellPool.query(`SELECT company_id,cell_id FROM "${company}".company_access_identity`);identity=result.rows[0];if(!identity)return false;await readFile(`${root}/native-documents/${identity.cell_id}.json`);return true;}catch{return false;}},'local company bootstrap');
 await stopDaemon();
 await writeFile(root+'/smtp.json',JSON.stringify({host:'localhost',from:'Restless launch check <accounts@restless-launch.test>'}),{mode:0o600});
 const sameCellDatabase=new URL((await readFile(`${root}/cells/${company}/database.url`,'utf8')).trim());
 sameCellDatabase.protocol=sameCellDatabase.protocol==='postgres:'?'postgresql:':'postgres:';
 sameCellDatabase.searchParams.set('application_name','restless-identity-rejection');
 await writeFile(root+'/rejected-account-database.url',sameCellDatabase.toString(),{mode:0o600});
 await assert.rejects(configure({coreHome:root,company,companyName:'Launch Studio',companyImage:process.env.RESTLESS_COMPANY_IMAGE,origin,coreOrigin:`http://${coreHost}:${Number(offset)+7788}`,ownerEmail:'owner@restless-launch.test',databaseUrlFile:root+'/rejected-account-database.url',smtpFile:root+'/smtp.json',output:root+'/rejected-accounts',port:Number(authPort)}),/separate database|company data/i);
 await assert.rejects(access(root+'/rejected-accounts'));
 console.log('PASS accounts configuration refuses the selected company database despite an equivalent URL spelling');
 await writeFile(root+'/account-database.url',process.env.RESTLESS_AUTH_TEST_DATABASE,{mode:0o600});
 const configured=await configureCli(['--core-home',root,'--company',company,'--company-name','Launch Studio','--company-image',process.env.RESTLESS_COMPANY_IMAGE,'--origin',origin,'--core-origin',`http://${coreHost}:${Number(offset)+7788}`,'--owner-email','owner@restless-launch.test','--database-url-file',root+'/account-database.url','--smtp-file',root+'/smtp.json','--output',root+'/accounts','--port',String(authPort)]);
 const configureFailure=configured.stderr.replaceAll(root,'[private state]').replace(/postgres(?:ql)?:\/\/\S+/gi,'[database URL]');
 assert.equal(configured.code,0,`documented configure CLI must succeed${configureFailure?`: ${configureFailure}`:''}`);
 serviceConfig=await readConfig(root+'/accounts/identity.json');
 const coreEntry=await readFile(root+'/accounts/core-entry.env','utf8');assert.match(coreEntry,/^export RESTLESS_ENTRY_MODE='network'$/m);assert.match(coreEntry,/^export RESTLESS_RUNTIME_MODE='local'$/m);
 ownerId=serviceConfig.ownerId;planeId=serviceConfig.planeId;
 console.log('PASS private account configuration generated from the existing Core company identity');
 service=await createIdentityService(serviceConfig,{sendMail:async mail=>messages.push(mail)});
 server=createServer(service.handle);await new Promise(resolve=>server.listen(Number(authPort),'127.0.0.1',resolve));
 startDaemon(true);
 await until(async()=>{try{return(await coreCall('/health')).data.status==='ok';}catch{return false;}},'authenticated Core gateway');
 await pause(3000);
 console.log('PASS authenticated network entry provisions local Documents with its public token issuer');
 async function authCall(path,body,cookie='',expected=200){
  const response=await fetch(origin+'/api/auth'+path,{method:body===undefined?'GET':'POST',headers:{Origin:origin,'Content-Type':'application/json',Cookie:cookie},body:body===undefined?undefined:JSON.stringify(body),redirect:'manual'});
  assert.equal(response.status,expected,`Auth ${path}: unexpected status`);
  const text=await response.text();return {data:text?JSON.parse(text):null,cookie:response.headers.getSetCookie().map(item=>item.split(';')[0]).join('; ')};
 }
 async function signup(name,email){const password='RestlessLaunchTest!123';await authCall('/sign-up/email',{name,email,password});await authCall('/sign-in/email',{email,password},'',403);const mail=messages.find(m=>m.to===email&&m.subject==='Verify your Restless email');assert(mail);const verified=await fetch(mail.text.match(/https?:\/\/\S+/)[0],{redirect:'manual'});assert([200,302].includes(verified.status));const result=await authCall('/sign-in/email',{email,password});return {...result,user:result.data.user};}
 const owner=await signup('Launch Owner','owner@restless-launch.test');
 await pause(11000);
 async function accountCall(path,body,cookie='',expected=200){
  const response=await fetch(origin+path,{method:body===undefined?'GET':'POST',headers:{Origin:origin,'Content-Type':'application/json',Cookie:cookie},body:body===undefined?undefined:JSON.stringify(body),redirect:'manual'});
  const data=await response.json();assert.equal(response.status,expected,`${path}: ${JSON.stringify(data)}`);return data;
 }
 assert((await accountCall('/api/company',undefined,owner.cookie)).canBootstrap);
 await accountCall('/api/company/bootstrap',{},owner.cookie);
 await accountCall('/api/invitations',{email:'colleague@restless-launch.test',role:'member'},owner.cookie);
 const mail=messages.find(m=>m.to==='colleague@restless-launch.test');assert(mail,'Invitation email must be sent');
 const invitationId=new URL(mail.text.match(/https?:\/\/\S+/)[0]).searchParams.get('invite');assert(invitationId);
 const colleague=await signup('Launch Colleague','colleague@restless-launch.test');
 await pause(11000);
 const outsider=await signup('Launch Outsider','outsider@restless-launch.test');
 await accountCall('/api/invitations/accept',{id:invitationId},outsider.cookie,403);
 await accountCall('/api/invitations/accept',{id:invitationId},colleague.cookie);
 await accountCall('/api/auth/organization/create',{name:'Bypass',slug:'bypass'},owner.cookie,404);
 const ownerAssertion=(await accountCall('/api/enter',{},owner.cookie)).assertion;
 owner.coreCookie=(await coreCall('/entry',{assertion:ownerAssertion})).cookie;
 colleague.coreCookie=(await coreCall('/entry',{assertion:(await accountCall('/api/enter',{},colleague.cookie)).assertion})).cookie;
 await coreCall('/entry',{assertion:ownerAssertion},'',401);
 const base=`/api/companies/${company}`;
 owner.principal=(await coreCall(base+'/principal',undefined,owner.coreCookie)).data;
 colleague.principal=(await coreCall(base+'/principal',undefined,colleague.coreCookie)).data;
 assert.notEqual(owner.principal.actor_id,colleague.principal.actor_id);
 assert.equal(colleague.principal.membership_role,'member');
 console.log('PASS verified invitations produce independent Core principals; replay refused');
 const humanActors=(await cellPool.query(`SELECT id,display,role FROM "${company}".actors WHERE id=ANY($1::text[])`,[[owner.principal.actor_id,colleague.principal.actor_id]])).rows;
 assert.equal(humanActors.find(actor=>actor.id===owner.principal.actor_id).display,'Launch Owner');
 assert.equal(humanActors.find(actor=>actor.id===colleague.principal.actor_id).display,'Launch Colleague');
 assert(humanActors.every(actor=>actor.role==='company-member'));
 console.log('PASS verified account names reach distinct human Actors without changing organisational roles');

 const created=(await coreCall(base+'/documents',{title:'Launch collaboration check',kind:'brief',visibility:'participants',content_json:{type:'doc',content:[{type:'paragraph',attrs:{block_id:randomUUID()},content:[{type:'text',text:'A proposal written together.'}]}]},reason:'Isolated collaboration test'},owner.coreCookie,201)).data;
 const doc=(await coreCall(base+`/documents/${created.document_id}`,undefined,owner.coreCookie)).data.document;
 await coreCall(base+`/documents/${doc.id}`,undefined,colleague.coreCookie,404);
 const shared=(await coreCall(base+`/documents/${doc.id}/participants/${colleague.principal.actor_id}`,{expected_document_version:doc.version,access:'comment'},owner.coreCookie,200,'PUT')).data;
 await coreCall(base+`/documents/${doc.id}`,undefined,colleague.coreCookie);
 await coreCall(base+`/documents/${doc.id}/comments`,{content_json:{type:'doc',content:[{type:'paragraph',attrs:{block_id:randomUUID()},content:[{type:'text',text:'Please clarify the scope.'}]}]}},colleague.coreCookie,201);
 console.log('PASS private document stays private until shared; invited human can comment');
 const readToken=(await coreCall(base+`/documents/${doc.id}/collaboration/token`,{},colleague.coreCookie)).data;
 assert.equal(readToken.access,'read','Comment access must not mint a write token');
 const beforeEdit=(await coreCall(base+`/documents/${doc.id}`,undefined,owner.coreCookie)).data.document;
 await coreCall(base+`/documents/${doc.id}/participants/${colleague.principal.actor_id}`,{expected_document_version:beforeEdit.version,access:'edit'},owner.coreCookie,200,'PUT');
 async function connect(account){
  const issued=(await coreCall(base+`/documents/${doc.id}/collaboration/token`,{},account.coreCookie)).data;
  assert.equal(issued.access,'write');
  class AuthenticatedSocket extends WS {constructor(url,protocols){super(url,protocols,{headers:{Host:coreHost,Origin:`http://${coreHost}`,Cookie:account.coreCookie}});}}
  const socket=new HocuspocusProviderWebsocket({url:core.replace('http:','ws:')+`/api/companies/${identity.company_id}/documents/${doc.id}/collaboration`,WebSocketPolyfill:AuthenticatedSocket,autoConnect:false,delay:1000,onClose:()=>socket.disconnect()});
  const document=new Y.Doc();const client={socket,document,closed:0};
  client.provider=new HocuspocusProvider({websocketProvider:socket,name:`${identity.company_id}:${doc.id}`,token:issued.token,document,onClose:()=>client.closed++});
  clients.push(client);client.provider.attach();void socket.connect().catch(()=>{});
  await until(()=>client.provider.isSynced,'authenticated live document sync',20);return client;
 }
 function append(client,text){const paragraph=new Y.XmlElement('paragraph');paragraph.setAttribute('block_id',randomUUID());const content=new Y.XmlText();content.insert(0,text);paragraph.insert(0,[content]);const fragment=client.document.getXmlFragment('default');fragment.insert(fragment.length,[paragraph]);}
 const ownerClient=await connect(owner),colleagueClient=await connect(colleague);
 append(ownerClient,'Owner clarified the project scope.');append(colleagueClient,'Colleague added the delivery schedule.');
 const hasBoth=client=>['Owner clarified','Colleague added'].every(text=>client.document.getXmlFragment('default').toString().includes(text));
 await until(()=>hasBoth(ownerClient)&&hasBoth(colleagueClient),'concurrent edits reach both authenticated humans',20);
 ownerClient.provider.destroy();ownerClient.socket.destroy();colleagueClient.provider.destroy();colleagueClient.socket.destroy();
 await pause(1500);
 const reconnected=await connect(colleague);assert(hasBoth(reconnected),'Reconnection must retain both contributions');
 console.log('PASS real authenticated document coediting, comment-only token scope and reconnect preserve both contributions');
 const removalTarget=(await accountCall('/api/company',undefined,owner.cookie)).members.find(member=>member.email===colleague.user.email);



 await accountCall('/api/members/remove',{id:removalTarget.id},colleague.cookie,403);
 if((await accountCall('/api/company',undefined,owner.cookie)).members.some(member=>member.id===removalTarget.id))await accountCall('/api/members/remove',{id:removalTarget.id},owner.cookie);
 await until(()=>reconnected.closed>0,'removal closes existing document connection',10);
 reconnected.provider.destroy();reconnected.socket.destroy();
 await coreCall(base+'/principal',undefined,colleague.coreCookie,401);
 await accountCall('/api/enter',{},colleague.cookie,403);
 await coreCall(base+`/documents/${doc.id}`,undefined,owner.coreCookie);
 console.log('PASS removal revokes an existing Core session and new entry while preserving owner access');
 if(!(await accountCall('/api/company',undefined,owner.cookie)).members.some(member=>member.email===outsider.user.email)){
 await accountCall('/api/invitations',{email:outsider.user.email,role:'member'},owner.cookie);
 let outageInvite=new URL(messages.filter(m=>m.to===outsider.user.email&&m.subject.startsWith('Join ')).at(-1).text.match(/https?:\/\/\S+/)[0]).searchParams.get('invite');
 await accountCall('/api/invitations/accept',{id:outageInvite},outsider.cookie);
 }
 const outageTarget=(await accountCall('/api/company',undefined,owner.cookie)).members.find(member=>member.email===outsider.user.email);
 await coreCall('/entry',{assertion:(await accountCall('/api/enter',{},outsider.cookie)).assertion});
 await stopDaemon();
 const pending=await accountCall('/api/members/remove',{id:outageTarget.id},owner.cookie,503);
 assert.match(pending.message,/New entry is blocked/);
 await accountCall('/api/enter',{},outsider.cookie,403);
 await new Promise(resolve=>server.close(resolve));await service.close();
 service=await createIdentityService(serviceConfig,{sendMail:async mail=>messages.push(mail)});
 server=createServer(service.handle);await new Promise(resolve=>server.listen(Number(authPort),'127.0.0.1',resolve));
 await accountCall('/api/enter',{},outsider.cookie,403);
 assert((await accountCall('/api/company',undefined,owner.cookie)).members.find(member=>member.id===outageTarget.id).removing);
 startDaemon(true);
 await until(async()=>{try{return(await coreCall('/health')).data.status==='ok';}catch{return false;}},'Core recovery after removal outage');
 await accountCall('/api/members/remove',{id:outageTarget.id},owner.cookie);
 await accountCall('/api/enter',{},outsider.cookie,403);
 assert(!(await accountCall('/api/company',undefined,owner.cookie)).members.some(member=>member.id===outageTarget.id));
 console.log('PASS removal during Core outage blocks entry across identity restart and completes on retry');
 await accountCall('/api/invitations',{email:outsider.user.email,role:'member'},owner.cookie);
 const cancelledId=new URL(messages.filter(m=>m.to===outsider.user.email&&m.subject.startsWith('Join ')).at(-1).text.match(/https?:\/\/\S+/)[0]).searchParams.get('invite');
 await accountCall('/api/invitations/cancel',{id:cancelledId},owner.cookie);
 await accountCall('/api/invitations/accept',{id:cancelledId},outsider.cookie,400);
 const crossOrigin=await fetch(origin+'/api/invitations',{method:'POST',headers:{Origin:'https://untrusted.example','Content-Type':'application/json',Cookie:owner.cookie},body:JSON.stringify({email:outsider.user.email,role:'admin'})});
 assert.equal(crossOrigin.status,403);
 await authCall('/request-password-reset',{email:outsider.user.email,redirectTo:origin+'/'});
 const resetMail=messages.filter(m=>m.to===outsider.user.email&&m.subject==='Reset your Restless password').at(-1);assert(resetMail);
 const callback=await fetch(resetMail.text.match(/https?:\/\/\S+/)[0],{redirect:'manual'});assert.equal(callback.status,302);
 const resetToken=new URL(callback.headers.get('location')).searchParams.get('token');assert(resetToken);
 await authCall('/reset-password',{token:resetToken,newPassword:'NewRestlessLaunchTest!123'});
 await authCall('/reset-password',{token:resetToken,newPassword:'CannotReuseLaunchToken!123'},'',400);
 await pause(11000);
 await authCall('/sign-in/email',{email:outsider.user.email,password:'RestlessLaunchTest!123'},'',401);
 await authCall('/sign-in/email',{email:outsider.user.email,password:'NewRestlessLaunchTest!123'});
 console.log('PASS cancelled invitation, cross-origin refusal and single-use password-reset email journey');


 await writeFile(process.env.RESTLESS_TEST_RESULT,JSON.stringify({mode:'self-hosted identity service with authenticated local Core',real_authentication:true,independent_principals:true,invitation_email_matching:true,entry_replay_rejected:true,private_document_access:true,comment_by_invited_human:true,existing_session_revoked:true,removal_outage_restart_retry:true,product_identity_service:true,published_user_journey:false,browser_coediting:false,live_protocol_coediting:true,reconnect_preserves_edits:true,open_document_connection_revoked:true},null,2));
}finally{
 for(const client of clients){client.provider.destroy();client.socket.destroy();client.document.destroy();}
 await stopDaemon();
 for(const socket of traceSockets)socket.destroy();
 if(traceProxy)await new Promise(resolve=>traceProxy.close(resolve));
 closeSync(logFd);if(server)await new Promise(resolve=>server.close(resolve));if(cellPool)await cellPool.end();if(service)await service.close();
 console.log('PASS identity service, Core and document clients stopped');
}
