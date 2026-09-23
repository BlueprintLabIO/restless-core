// UI fixture: no company writes. Real Hocuspocus transport joins two browser editors.
import assert from 'node:assert/strict';
import { chromium } from 'playwright';
import { Server } from '@hocuspocus/server';
import * as Y from 'yjs';
import { createServer } from 'node:http';
import { connect } from 'node:net';
import { readFile } from 'node:fs/promises';
import { resolve, extname } from 'node:path';

const source = process.env.RESTLESS_TEST_SOURCE_COMPANY;
assert(source);
const company='attention_document_test', document='11111111-1111-4111-8111-111111111111', version='22222222-2222-4222-8222-222222222222', request='33333333-3333-4333-8333-333333333333';
const now=new Date().toISOString(), content={type:'doc',content:[{type:'paragraph',attrs:{block_id:'opening'},content:[{type:'text',text:'Shared opening'}]}]};
const seedDoc=new Y.Doc(),paragraph=new Y.XmlElement('paragraph'),text=new Y.XmlText('Shared opening');paragraph.setAttribute('block_id','opening');paragraph.insert(0,[text]);seedDoc.getXmlFragment('default').insert(0,[paragraph]);const seed=Y.encodeStateAsUpdate(seedDoc);seedDoc.destroy();
let peers=0,persisted=seed;
const collab=new Server({port:0,address:'127.0.0.1',quiet:true,stopOnSignals:false,onConnect:async()=>{peers+=1;},onAuthenticate:async()=>({}),onStoreDocument:async({document:doc})=>{persisted=Y.encodeStateAsUpdate(doc);},onLoadDocument:async()=>{const d=new Y.Doc();Y.applyUpdate(d,persisted);return d;}});
let preview,browser,closing=false,mode='collaboration',resolved=false;
const errors=[],bridges=new Set();
try {
 await collab.listen();
 const root=resolve('build');
 preview=createServer(async(req,res)=>{
  const path=resolve(root,`.${new URL(req.url,'http://fixture').pathname}`);
  if(!path.startsWith(root+'/') && path!==root){res.writeHead(404);res.end();return;}
  let file=path,body;try{body=await readFile(file);}catch{file=resolve(root,'index.html');body=await readFile(file);}
  res.setHeader('content-type',({'.html':'text/html','.js':'application/javascript','.css':'text/css','.svg':'image/svg+xml','.json':'application/json','.woff2':'font/woff2'})[extname(file)]??'application/octet-stream');res.end(body);
 });
 preview.on('upgrade',(req,socket,head)=>{
  const upstream=connect(collab.address.port,'127.0.0.1');bridges.add(socket);bridges.add(upstream);
  upstream.on('connect',()=>{upstream.write(`${req.method} ${req.url} HTTP/${req.httpVersion}\r\n${Object.entries(req.headers).map(([key,value])=>`${key}: ${value}`).join('\r\n')}\r\n\r\n`);if(head.length)upstream.write(head);socket.pipe(upstream).pipe(socket);});
  upstream.on('error',()=>socket.destroy());socket.on('error',()=>upstream.destroy());
  socket.on('close',()=>{upstream.destroy();bridges.delete(socket);bridges.delete(upstream);});
 });
 await new Promise(r=>preview.listen(0,'127.0.0.1',r));const base=`http://127.0.0.1:${preview.address().port}`;
 browser=await chromium.launch({executablePath:process.env.PLAYWRIGHT_CHROMIUM_EXECUTABLE ?? '/home/black/.cache/ms-playwright/chromium-1234/chrome-linux64/chrome'});
 const context=await browser.newContext({viewport:{width:1680,height:1050}});
 async function page() {
  const p=await context.newPage();p.on('pageerror',e=>errors.push(e.message));p.on('websocket',ws=>{ws.on('socketerror',error=>errors.push(String(error)));});
  await p.route('**/api/**',async route=>{try {
   const r=route.request(),u=new URL(r.url()),path=u.pathname.replace(`/api/companies/${company}`,'');
   if(path==='/attention') return route.fulfill({json:{company:{id:company,name:'Collaboration fixture',mission:'Co-edit a document',model:'fixture'},source_health:{orgintel:'available',authority:'available',runtime:'available',browser:'available'},items:resolved?[]:[{id:`document:${mode}:${request}`,source:{plane:'orgintel',kind:`document_${mode}`,reference:request},category:mode==='review'?'review':'collaboration',title:'Shared brief',what_happened:'Work on this together',why_it_matters:'A shared draft',recommendation:'Edit together',requested_action:'Work together',if_no_action:'Stays open',brief_status:'source-authored',evidence:[],responsible_actor:{id:"exec",display:"Exec",role:"executive"},actions:[{id:"chat-document",label:"Discuss document",role:"conversation",consequence:"Discuss the draft",next_state:"Request stays open"}],can_continue:false,created_at:now,native_document:{id:request,document_id:document,title:'Shared brief',summary:'Help refine the opening together.',requested_by_actor_id:'exec',created_at:now,kind:mode,named_version_id:mode==='review'?version:null}}],continuations:[],refreshed_at:now}});
   if(path.startsWith(`/documents/${document}`)) {
    const versionView={version:{id:version,document_id:document,version_number:1,schema_version:1,content_json:content,plain_text:'Shared opening',content_hash:'a'.repeat(64),document_status:'draft',restored_from_version_id:null,created_by_actor_id:'exec',reason:'Initial brief',created_at:now},rendered_html:'<p>Shared opening</p>',markdown:'Shared opening'};
    if(path.endsWith('/resolve')) {assert.equal(r.method(),'POST');resolved=true;return route.fulfill({json:{id:request,resolved_at:now}});}
    if(path.endsWith('/collaboration/token'))return route.fulfill({json:{token:'fixture-token',token_type:'Bearer',access:'write',expires_at:new Date(Date.now()+60000).toISOString()}});
    assert.equal(r.method(),'GET',`Unexpected document write ${path}`);
    if(path===`/documents/${document}`)return route.fulfill({json:{document:{id:document,title:'Shared brief',kind:'brief',status:'draft',visibility:'participants',linked_room_id:null,inherit_room_visibility:false,owner_actor_id:'exec',current_named_version_id:version,created_by_actor_id:'exec',created_at:now,updated_at:now,version:1},access:'edit',current_version:versionView}});
    if(path===`/documents/${document}/versions/${version}`)return route.fulfill({json:versionView});
    if(path.endsWith('/reviews'))return route.fulfill({json:{items:mode==='review'?[{review:{id:request,document_id:document,requested_version_id:version,requested_by_actor_id:'exec',summary:'Review the exact version',status:'requested',created_at:now,version:1},work_dependency:null}]:[],next_cursor:null}});
    return route.fulfill({json:{items:[],next_cursor:null}});
   }
   assert.equal(r.method(),'GET',`Unexpected write ${path}`);
   if(path.includes('/activity'))return route.fulfill({body:': fixture\n\n',contentType:'text/event-stream'});
   const target=r.url().replace(base,'http://127.0.0.1:8788').replaceAll(company,source);
   const response=await route.fetch({url:target});await route.fulfill({response,body:(await response.text()).replaceAll(source,company)});
  }catch(error){if(!closing){errors.push(error.message);await route.abort();}}});
  await p.goto(`${base}/${company}`);return p;
 }
 const first=await page(),second=await page();
 await first.locator('.tiptap[contenteditable="true"]').waitFor();await second.locator('.tiptap[contenteditable="true"]').waitFor();
 await first.locator('.tiptap').getByText('Shared opening',{exact:true}).waitFor();
 await second.locator('.tiptap').getByText('Shared opening',{exact:true}).waitFor();
 await first.locator('.tiptap').press('Control+End');await first.locator('.tiptap').pressSequentially(' with a human edit');
 await second.getByText('Shared opening with a human edit',{exact:true}).waitFor();
 await first.getByRole('link',{name:'Discuss',exact:true}).click();
 await first.getByRole('textbox',{name:'Message Exec',exact:true}).first().waitFor();
 await first.locator('.desktop-stage .tiptap[contenteditable="true"]').waitFor();
 assert((await first.locator('.desktop-stage .tiptap').innerText()).includes('human edit'));
 await first.goto(`${base}/${company}`);
 await first.locator('.tiptap[contenteditable="true"]').waitFor();
 await first.getByRole('button',{name:'Comments and review',exact:true}).click();
 await first.getByRole('complementary',{name:'Document review and discussion'}).waitFor();
 for(const width of [1680,1024,390]) {await first.setViewportSize({width,height:950});await first.waitForTimeout(150);if(width===390 && await first.locator('.tb-exec[aria-expanded="true"]').count())await first.locator('.tb-exec[aria-expanded="true"]').click();assert(await first.locator('.attention-document').evaluate(e=>e.scrollWidth<=e.clientWidth+1),`document overflow at ${width}`);const frame=await first.locator('.document-editor').boundingBox(),save=await first.getByRole('button',{name:'Document actions',exact:true}).boundingBox();assert(save.x>=frame.x && save.x+save.width<=frame.x+frame.width+1,`save action clipped at ${width}`);assert(await first.evaluate(()=>document.documentElement.scrollWidth<=innerWidth+1),`page overflow at ${width}`);}
 await first.screenshot({path:'../../work/browser/attention-document-mobile.png'});
 await first.getByRole('button',{name:'Document actions',exact:true}).click();
 await first.getByRole('button',{name:'Finish collaboration',exact:true}).click();assert(resolved);
 mode='review';resolved=false;await second.reload();await second.getByRole('article',{name:'Requested document version'}).waitFor();
 await second.getByText('Version 1 under review',{exact:true}).waitFor();
 assert.equal((await second.locator('.requested-version').innerText()).replace(/\s+/g,' '),'Version 1 under review Shared opening');
 await second.getByRole('button',{name:'Document actions',exact:true}).click();
 await second.getByRole('button',{name:'Edit live document',exact:true}).click();
 await second.getByText('Shared opening with a human edit',{exact:true}).waitFor();
 await second.screenshot({path:'../../work/browser/attention-document-desktop.png'});
 assert(peers<20,`connection setup loop: ${peers} peers`);
 assert.deepEqual(errors,[]);
 console.log('PASS two browser editors synchronize in Attention; exact requested snapshot remains distinct; desktop/mobile fit; no company writes');
} catch(error) {
 console.error('Browser errors:',errors);
 if(browser) for(const context of browser.contexts())for(const p of context.pages())console.error('Visible fixture:',await p.locator('.attention-document').innerText().catch(()=> 'No document workspace'));
 throw error;
} finally {
 closing=true;if(browser)await browser.close();for(const socket of bridges)socket.destroy();await collab.destroy();
 if(preview){preview.closeAllConnections();await new Promise(r=>preview.close(r));}
}
