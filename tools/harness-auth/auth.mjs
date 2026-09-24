#!/usr/bin/env node
// Native CLI authentication only. Never return tokens or raw CLI output to the owner API.
import fs from 'node:fs';
import {spawn, execFileSync} from 'node:child_process';
import {createInterface} from 'node:readline';
process.umask(0o077);
const [harness, action]=process.argv.slice(2);
if(!['codex','claude-agent'].includes(harness)||!['login','status','logout','cancel'].includes(action))process.exit(2);
const dir=`/company/home/.restless/harness-auth/${harness}`;
fs.mkdirSync(dir,{recursive:true,mode:0o700});fs.chmodSync(dir,0o700);
const file=`${dir}/login-state.json`, pidFile=`${dir}/login.pid`;
const claude='/usr/local/lib/node_modules/@agentclientprotocol/claude-agent-acp/node_modules/@anthropic-ai/claude-agent-sdk-linux-x64/claude';
const env={...process.env,CODEX_HOME:dir,CLAUDE_CONFIG_DIR:dir,DISPLAY:':1',HOME:'/company/home'};
for(const key of ['OPENAI_API_KEY','OPENAI_BASE_URL','ANTHROPIC_API_KEY','ANTHROPIC_AUTH_TOKEN','ANTHROPIC_BASE_URL','CLAUDE_CODE_OAUTH_TOKEN'])delete env[key];
const write=value=>{fs.writeFileSync(`${file}.tmp`,JSON.stringify(value),{mode:0o600});fs.renameSync(`${file}.tmp`,file);};
const read=()=>{try{return JSON.parse(fs.readFileSync(file,'utf8'));}catch{return {state:'disconnected'};}};
const out=value=>process.stdout.write(JSON.stringify(value)+'\n');
let child;
function stop(){if(child){try{child.kill('SIGTERM');}catch{}}}
process.on('SIGTERM',()=>{stop();process.exit(0);});
function cancel(){try{const pid=Number(fs.readFileSync(pidFile,'utf8'));const cmd=fs.readFileSync(`/proc/${pid}/cmdline`,'utf8');if(pid>1&&cmd.includes('restless-harness-auth')&&cmd.includes('login'))process.kill(pid,'SIGTERM');}catch{}}
async function codex(){
 child=spawn('codex',['app-server','--stdio'],{env,stdio:['pipe','pipe','ignore']});
 const pending=new Map();let id=0;let completed;
 const completion=new Promise(resolve=>completed=resolve);
 createInterface({input:child.stdout}).on('line',line=>{try{const x=JSON.parse(line);if(x.id!=null){const p=pending.get(x.id);if(p){pending.delete(x.id);x.error?p.reject(new Error('Native Codex request failed')):p.resolve(x.result);}}if(x.method==='account/login/completed')completed(x.params);}catch{}});
 child.on('exit',()=>{for(const p of pending.values())p.reject(new Error('Codex exited'));pending.clear();});
 const request=(method,params={})=>new Promise((resolve,reject)=>{const n=++id;pending.set(n,{resolve,reject});child.stdin.write(JSON.stringify({id:n,method,params})+'\n');});
 await request('initialize',{clientInfo:{name:'restless_harness_auth',version:'1.0.0'},capabilities:{experimentalApi:true}});
 child.stdin.write(JSON.stringify({method:'initialized',params:{}})+'\n');
 return {request,completion};
}
const timer=setTimeout(()=>{if(action==='login')write({state:'expired',message:'Sign-in expired. Start again.'});stop();process.exit(1);},action==='login'?900000:action==='status'?45000:20000);
try{
 if(action==='cancel'){cancel();write({state:'cancelled'});out({state:'cancelled'});}
 else if(action==='logout'){
  cancel();if(harness==='codex'){const rpc=await codex();await rpc.request('account/logout');}else execFileSync(claude,['auth','logout'],{env,timeout:15000,stdio:'ignore'});
  write({state:'disconnected'});out({state:'disconnected'});
 }else if(action==='status'){
  let account=null;let models;
  if(harness==='codex'){const rpc=await codex();const result=await rpc.request('account/read',{refreshToken:false});if(result.account){account={type:result.account.type,email:result.account.email??null};try { models=[];let cursor=null;do {const list=await rpc.request('model/list',{includeHidden:false,...(cursor?{cursor}:{})});models.push(...list.data.filter(m=>!m.hidden).map(m=>({id:m.model??m.id,name:m.displayName??m.model??m.id,default:!!m.isDefault})));cursor=list.nextCursor;}while(cursor&&models.length<1000);}catch{models=undefined;}}}
  else {try{const result=JSON.parse(execFileSync(claude,['auth','status','--json'],{env,timeout:15000,encoding:'utf8',stdio:['ignore','pipe','ignore']}));if(result.loggedIn)account={type:result.authMethod,email:result.email??null};}catch{}}
  const state=read();if(account)out({state:'connected',account,models});else if(state.expires_at && Date.now()>state.expires_at)out({state:'expired'});else out(state.state==='connected'?{state:'disconnected'}:state);
 }else{
  cancel();fs.writeFileSync(pidFile,String(process.pid),{mode:0o600});
  const expires_at=Date.now()+900000;write({state:'starting',expires_at});
  if(harness==='codex'){
   const rpc=await codex();const login=await rpc.request('account/login/start',{type:'chatgptDeviceCode'});
   const url=new URL(login.verificationUrl);if(url.protocol!=='https:'||!['auth.openai.com','chatgpt.com','auth0.openai.com'].includes(url.hostname))throw new Error('Unexpected verification host');
   write({state:'waiting',verification_url:url.href,user_code:login.userCode,expires_at});
   const done=await rpc.completion;write({state:done.success?'connected':'failed',message:done.success?'Signed in.':'Native sign-in failed. Please try again.'});
  }else{
   for(let attempt=0;attempt<60&&!fs.existsSync('/tmp/.X11-unix/X1');attempt++)await new Promise(resolve=>setTimeout(resolve,1000));
   if(!fs.existsSync('/tmp/.X11-unix/X1'))throw new Error('Company display did not start');
   const browser=`${dir}/open-browser`;
   fs.writeFileSync(browser,'#!/bin/sh\nexec /usr/bin/chromium --display=:1 --remote-debugging-address=127.0.0.1 --remote-debugging-port=9222 --user-data-dir=/company/browser-profile --no-first-run --no-default-browser-check --no-sandbox "$@" >/dev/null 2>&1\n',{mode:0o700});
   child=spawn(claude,['auth','login','--claudeai'],{env:{...env,BROWSER:browser},stdio:['ignore','ignore','ignore']});
   write({state:'waiting',desktop:true,expires_at});
   const code=await new Promise(resolve=>child.on('exit',resolve));write({state:code===0?'connected':'failed',message:code===0?'Signed in.':'Native sign-in failed. Please try again.'});
  }
 }
}catch{if(action==='login')write({state:'failed',message:'Native login could not complete. Check internet access and try again.'});else out({state:'unavailable',message:'Native authentication could not be checked.'});process.exitCode=1;}
finally{clearTimeout(timer);stop();}
