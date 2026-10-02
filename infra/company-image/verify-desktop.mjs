import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { access, readFile, writeFile } from 'node:fs/promises';
import { chromium } from '/usr/local/lib/node_modules/playwright/index.mjs';

const children=[];
const start=(command,args)=>{
  const child=spawn(command,args,{stdio:['ignore','pipe','pipe']});
  let output='';
  child.stdout.on('data',data=>output+=data);
  child.stderr.on('data',data=>output+=data);
  child.on('error',error=>output+=error.stack);
  children.push({child,output:()=>output});
  return child;
};
async function until(check,label){
  for(let i=0;i<100;i++){
    try{if(await check())return;}catch{}
    await new Promise(resolve=>setTimeout(resolve,100));
  }
  throw new Error(`Timeout: ${label}`);
}
let browser;
try{
  assert.equal(process.getuid(),2000);
  assert.match(process.version,/^v24\./);
  await assert.rejects(access('/usr/bin/node'));
  for(const name of ['nodejs','libnode115','node-undici','node-brace-expansion','node-minimatch']){
    assert.ok(!(await readFile('/var/lib/dpkg/status','utf8')).includes(`Package: ${name}\n`),name);
  }
  assert.equal(JSON.parse(await readFile('/usr/share/novnc/package.json')).version,'1.6.0');
  console.log('PASS desktop uses Node 24; Debian Node 20 and its vulnerable dependencies are absent');
  start('/usr/bin/Xtigervnc',[':1','-geometry','1280x800','-depth','24','-SecurityTypes','None','-localhost','yes','-rfbport','5901']);
  await until(async()=>{await access('/tmp/.X11-unix/X1');return true;},'TigerVNC display');
  start('/usr/bin/xterm',['-display',':1','-geometry','80x15+20+20','-title','Restless desktop probe','-e','/bin/sh','-c','printf "Restless desktop: real VNC connection\\n"; sleep 120']);
  start('/usr/bin/websockify',['--web=/usr/share/novnc','127.0.0.1:6080','127.0.0.1:5901']);
  await until(async()=>{const r=await fetch('http://127.0.0.1:6080/vnc.html');await r.arrayBuffer();return r.ok;},'noVNC HTTP');
  const errors=[];
  browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox','--disable-dev-shm-usage']});
  const page=await browser.newPage({viewport:{width:1440,height:1000}});
  page.on('pageerror',error=>errors.push(error.message));
  const url='http://127.0.0.1:6080/vnc.html?autoconnect=1&reconnect=1&shared=1&show_dot=1&resize=remote&view_only=1&path=websockify';
  await page.goto(url);
  await page.waitForFunction(()=>document.documentElement.classList.contains('noVNC_connected'),null,{timeout:15000});
  await page.locator('#noVNC_transition').waitFor({state:'hidden'});
  const observed=await page.evaluate(async()=>{
    const {default:UI}=await import('/app/ui.js');
    const canvas=document.querySelector('canvas');
    return {connected:UI.connected,state:UI.rfb._rfbConnectionState,viewOnly:UI.rfb.viewOnly,width:canvas.width,height:canvas.height};
  });
  assert.equal(observed.connected,true);
  assert.equal(observed.state,'connected');
  assert.equal(observed.viewOnly,true);
  assert.ok(observed.width>0&&observed.height>0);
  assert.deepEqual(errors,[]);
  await page.screenshot({path:'/tmp/desktop-proof.png'});
  await writeFile('/tmp/desktop-proof.json',JSON.stringify({url,uid:process.getuid(),node:process.version,...observed,errors},null,2)+'\n');
  console.log('PASS preserved noVNC URL: real TigerVNC/WebSocket connection, rendered framebuffer, view-only mode, no browser errors');
}catch(error){
  for(const p of children) console.error(p.output());
  throw error;
}finally{
  await browser?.close();
  for(const p of children.reverse()) p.child.kill('SIGTERM');
  await Promise.all(children.map(({child})=>new Promise(resolve=>{
    if(child.exitCode!==null||child.signalCode!==null)return resolve();
    const timer=setTimeout(()=>{child.kill('SIGKILL');resolve();},2000);
    child.once('exit',()=>{clearTimeout(timer);resolve();});
  })));
}
