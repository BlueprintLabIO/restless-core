import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawn } from 'node:child_process';
import { chromium } from 'playwright-core';

const runtimeDir=join(tmpdir(),'restless-greenfield-review');
await mkdir(runtimeDir,{recursive:true});
const pidPath=join(runtimeDir,'server.pid');
let oldPid;
try { oldPid=Number(await readFile(pidPath,'utf8')); process.kill(oldPid,'SIGTERM'); } catch {}
const server=spawn(process.execPath,['scripts/server.js','dist'],{detached:true,stdio:['ignore','ignore','ignore'],env:{...process.env,PORT:'8080'}});
server.unref();
await writeFile(pidPath,String(server.pid));
let ready=false;
for(let i=0;i<30;i++){try{const r=await fetch('http://127.0.0.1:8080/');if(r.ok){ready=true;break}}catch{} await new Promise(r=>setTimeout(r,100));}
if(!ready) throw new Error('Review target did not become live');
const browser=await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox']});
const observations=[];
try{
 for(const viewport of [{width:390,height:844,name:'mobile'},{width:1440,height:1000,name:'desktop'}]){
  const page=await browser.newPage({viewport});
  const response=await page.goto('http://127.0.0.1:8080/',{waitUntil:'networkidle'});
  const metrics=await page.evaluate(()=>({title:document.title,h1:document.querySelector('h1')?.innerText,scrollWidth:document.documentElement.scrollWidth,clientWidth:document.documentElement.clientWidth}));
  if(!response?.ok()||metrics.scrollWidth>metrics.clientWidth+1) throw new Error(`Probe failed: ${viewport.name}`);
  await page.screenshot({path:join(runtimeDir,`home-${viewport.name}.png`),fullPage:true});
  observations.push({viewport:`${viewport.width}x${viewport.height}`,status:response.status,...metrics});
 }
}finally{await browser.close()}
const report={probedAt:new Date().toISOString(),url:'http://127.0.0.1:8080/',serverPid:server.pid,observations};
await writeFile(join(runtimeDir,'probe.json'),JSON.stringify(report,null,2)+'\n');
console.log(JSON.stringify(report,null,2));
