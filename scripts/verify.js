import { readFile, readdir } from 'node:fs/promises';
import { spawn } from 'node:child_process';
import { chromium } from 'playwright-core';

const required = ['/', '/product/', '/how-it-works/', '/research/', '/compare/', '/journal/', '/journal/one-worker-default/', '/journal/parallel-thresholds/', '/journal/supervision-correctness/'];
const run = (cmd,args) => new Promise((resolve,reject)=>{const p=spawn(cmd,args,{stdio:'inherit'});p.on('exit',code=>code===0?resolve():reject(new Error(`${cmd} exited ${code}`)));});
await run(process.execPath,['scripts/build.js']);
const files = await readdir('dist',{recursive:true});
const publicText = (await Promise.all(files.filter(f=>f.endsWith('.html')).map(f=>readFile(`dist/${f}`,'utf8')))).join('\n');
if (publicText.includes('—')) throw new Error('Public HTML contains an em dash');
for (const route of required) {
  const file = route === '/' ? 'dist/index.html' : `dist${route}index.html`;
  await readFile(file);
}
for (const token of ['<main id="main">','<nav id="nav" aria-label="Primary">','prefers-reduced-motion','og:image']) {
  const haystack = token === 'prefers-reduced-motion' ? await readFile('dist/assets/site.css','utf8') : publicText;
  if (!haystack.includes(token)) throw new Error(`Missing requirement: ${token}`);
}
const server = spawn(process.execPath,['scripts/server.js','dist'],{env:{...process.env,PORT:'4173'},stdio:['ignore','pipe','pipe']});
await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(new Error('server timeout')),5000);server.stdout.on('data',()=>{clearTimeout(timer);resolve();});});
const browser = await chromium.launch({executablePath:'/usr/bin/chromium',headless:true,args:['--no-sandbox']});
try {
  for (const viewport of [{width:390,height:844},{width:1440,height:1000}]) {
    const page = await browser.newPage({viewport});
    for (const route of required) {
      const response = await page.goto(`http://127.0.0.1:4173${route}`,{waitUntil:'networkidle'});
      if (!response?.ok()) throw new Error(`${route} returned ${response?.status()}`);
      const measurements = await page.evaluate(() => ({scrollWidth:document.documentElement.scrollWidth, clientWidth:document.documentElement.clientWidth, h1:document.querySelectorAll('h1').length, main:!!document.querySelector('main')}));
      if (measurements.scrollWidth > measurements.clientWidth + 1) throw new Error(`${route} overflows at ${viewport.width}: ${JSON.stringify(measurements)}`);
      if (measurements.h1 !== 1 || !measurements.main) throw new Error(`${route} lacks expected document structure`);
    }
    await page.close();
  }
  const page = await browser.newPage({viewport:{width:390,height:844}});
  await page.goto('http://127.0.0.1:4173/');
  await page.keyboard.press('Tab');
  const focused = await page.evaluate(() => document.activeElement?.className);
  if (focused !== 'skip') throw new Error(`Skip link not first focus target: ${focused}`);
  await page.locator('[data-example="research"]').click();
  if (!(await page.locator('[data-mission]').textContent()).includes('market thesis')) throw new Error('Product demo interaction failed');
  await page.close();
} finally { await browser.close(); server.kill('SIGTERM'); }
console.log(`VERIFY PASS: ${required.length} routes, 2 viewports, no overflow, structure, keyboard skip and product interaction.`);
