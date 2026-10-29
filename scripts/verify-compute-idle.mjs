// Exercise actual isolated company computers. Use a short browser idle timeout
// (RESTLESS_BROWSER_IDLE_MS=1500) and a local mock with --log; no paid model.
// RESTLESS_RESOURCE_NAMESPACE=... RESTLESS_VERIFY_OUTPUT=... \
// RESTLESS_RESOURCE_MOCK_LOG=... RESTLESS_RESOURCE_MOCK_URL=http://127.0.0.1:... \
// node scripts/verify-compute-idle.mjs resource1_test resource2_test ...
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, writeFile, readFile } from 'node:fs/promises';

const namespace = process.env.RESTLESS_RESOURCE_NAMESPACE;
const output = process.env.RESTLESS_VERIFY_OUTPUT;
const mockLog = process.env.RESTLESS_RESOURCE_MOCK_LOG;
const mockUrl = new URL(process.env.RESTLESS_RESOURCE_MOCK_URL);
const companies = process.argv.slice(2);
const wakeAll = process.env.RESTLESS_RESOURCE_WAKE_ALL === '1';
assert(/^[a-z0-9][a-z0-9-]*$/.test(namespace ?? ''), 'An isolated dev namespace is required');
assert(output && mockLog && companies.length >= 2, 'Output, mock log and at least two companies are required');
assert(companies.every(name => /^[a-z0-9_]+_test$/.test(name)), 'Only disposable _test companies are allowed');
assert(['127.0.0.1', 'localhost', '[::1]'].includes(mockUrl.hostname) && mockUrl.protocol === 'http:', 'Model control must use a local mock');
await mkdir(output, { recursive: true });

async function command(args, input) {
  return await new Promise((resolve, reject) => {
    const child = spawn('docker', args, { stdio: ['pipe', 'pipe', 'pipe'] });
    let stdout = '', stderr = '';
    const timer = setTimeout(() => child.kill('SIGKILL'), 90_000);
    child.stdout.on('data', data => stdout += data);
    child.stderr.on('data', data => stderr += data);
    child.on('error', error => { clearTimeout(timer); reject(error); });
    child.on('exit', code => {
      clearTimeout(timer);
      code === 0 ? resolve(stdout) : reject(Error(`Isolated resource probe failed (${code}): ${stderr.slice(0, 400)}`));
    });
    child.stdin.on('error', () => {});
    child.stdin.end(input);
  });
}
async function requestCount() {
  return (await readFile(mockLog, 'utf8')).split('\n').filter(Boolean).length;
}
// Prove the log is connected before treating a flat request count as evidence.
let initial = 0;
try { initial = await requestCount(); } catch (error) { if (error.code !== 'ENOENT') throw error; }
const control = await fetch(new URL('/v1/messages', mockUrl), {
  method: 'POST', headers: { 'content-type': 'application/json', 'x-api-key': 'synthetic-resource-control' },
  body: JSON.stringify({ model: 'claude-opus-5-5', max_tokens: 1, messages: [{ role: 'user', content: 'Resource logging control' }] }),
  signal: AbortSignal.timeout(10_000)
});
assert.equal(control.status, 200);
await control.text();
const requestsBefore = await requestCount();
assert.equal(requestsBefore, initial + 1, 'The local model log must record the control request');

const probe = `import http from 'node:http'; import {execFileSync} from 'node:child_process';
async function get(probe) {
  return await new Promise((resolve,reject) => {
    const request=http.get({host:'127.0.0.1',port:9223,path:'/json/version',agent:false,
      headers:probe?{'x-restless-probe':'1'}:{},timeout:20000},response=>{
      let body=''; response.on('data',x=>body+=x);
      response.on('end',()=>{try{resolve(JSON.parse(body))}catch{reject(Error(body.slice(0,150)))}});
    });
    request.on('timeout',()=>request.destroy(Error('Browser readiness timeout')));
    request.on('error',reject);
  });
}
const first=await get(true);
if(!first.asleep)throw Error('An untouched idle company started Chrome');
if(process.argv[2]==='wake') {
  const awake=await get(false);
  if(awake.asleep || !awake.webSocketDebuggerUrl)throw Error('A real browser request failed to wake Chrome');
}
let asleep=Boolean(first.asleep);
for(let i=0;i<80;i++) {
  asleep=Boolean((await get(true)).asleep);
  await new Promise(resolve=>setTimeout(resolve,150));
}
let state='';
try{state=execFileSync('/usr/bin/supervisorctl',['-c','/etc/supervisor/conf.d/computer.conf','status','chromium'],{encoding:'utf8',timeout:5000});}
catch(error){state=String(error.stdout??'');}
console.log(JSON.stringify({initiallyAsleep:Boolean(first.asleep),finallyAsleep:asleep,supervisor:state.trim()}));`;
const report = await Promise.all(companies.map(async (company, index) => {
  const result = JSON.parse((await command(['exec', '-i', '-u', 'company', `restless-${namespace}-co-${company}`,
    'node', '--input-type=module', '-', wakeAll || index === 0 ? 'wake' : 'idle'], probe)).trim());
  assert(result.finallyAsleep, 'Read-only probes kept an idle browser awake');
  assert(result.supervisor.includes('STOPPED'), 'Chrome must stop after becoming idle');
  return { company, ...result };
}));
const names = companies.flatMap(company => [`restless-${namespace}-co-${company}`, `restless-${namespace}-pc-${company}`]);
const stats = (await command(['stats', '--no-stream', '--format', '{{json .}}', ...names]))
  .split('\n').filter(Boolean).map(line => JSON.parse(line));
const requestsAfter = await requestCount();
assert.equal(requestsAfter, requestsBefore, 'Quiet companies must not call the model');
const result = { wokenCompanies: wakeAll ? companies.length : 1, report, stats, mockLoggingControl: 'pass', modelCallsDuringIdle: requestsAfter - requestsBefore };
await writeFile(`${output}/report.json`, JSON.stringify(result, null, 2));
console.log(JSON.stringify(result));
