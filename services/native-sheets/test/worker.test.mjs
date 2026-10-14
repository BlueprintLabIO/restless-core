import test from 'node:test';
import assert from 'node:assert/strict';
import { randomUUID } from 'node:crypto';
import { execute, ENGINE_VERSION } from '../src/worker.mjs';
import { Model } from '../src/upstream.mjs';
const blank = { engine_version: ENGINE_VERSION };
const seed = [ ['record_id','Item','Buy','Sell','Fees','Profit'], ['gfx','GFX 50S II',2100,3200,100,'=D2-C2-E2'], ['gpu','RTX 3090',800,1150,50,'=D3-C3-E3'] ];

test('agent batches replay formulas and update stable records after sorting', async () => {
  const base = (await execute(blank)).workbook;
  const write = await execute({ ...blank, snapshot: base, operation: { action:'set_cells', values:seed } });
  let messages = write.messages;
  const read = await execute({ ...blank, snapshot:base, messages, operation:{action:'get_range',range:'F2:F3'} });
  assert.deepEqual(read.result.rows.map(r=>r[0].value),[1000,300]);
  const sort = await execute({ ...blank,snapshot:base,messages,operation:{action:'sort',range:'A2:F3',by:'C2',direction:'asc'} });
  messages = [...messages,...sort.messages];
  const edit = await execute({ ...blank,snapshot:base,messages,operation:{action:'update_record',range:'A1:F3',record_id:'gfx',values:{Sell:3100}} });
  messages = [...messages,...edit.messages];
  const result = await execute({ ...blank,snapshot:base,messages,operation:{action:'get_range',range:'A1:F3'} });
  const gfx = result.result.rows.find(r=>r[0].value==='gfx');
  assert.equal(gfx[3].value,3100);assert.equal(gfx[5].value,900);
});
test('CSV uses all populated rows, quoted fields and auto-expands worksheet', async () => {
  const base=(await execute(blank)).workbook;
  const write=await execute({...blank,snapshot:base,operation:{action:'import_csv',start:'A101',csv:'Item,Note\r\nGFX,"line one\nline two, quoted"\r\n'}});
  const read=await execute({...blank,snapshot:base,messages:write.messages,operation:{action:'export_csv'}});
  assert.match(read.result.csv,/GFX,"line one\nline two, quoted"/);
  assert.equal(read.workbook.sheets[0].rowNumber,102);
});
test('importing a CSV again changes only what differs, and an identical import changes nothing', async () => {
  const base=(await execute(blank)).workbook;
  const first=await execute({...blank,snapshot:base,operation:{action:'import_csv',csv:'Item,Q1\nRent,40\nPay,100'}});
  const again=await execute({...blank,snapshot:base,messages:first.messages,operation:{action:'import_csv',csv:'Item,Q1\nRent,45\nPay,100\nAds,12'}});
  const read=await execute({...blank,snapshot:base,messages:[...first.messages,...again.messages],operation:{action:'export_csv'}});
  assert.equal(read.result.csv.trimEnd(),'Item,Q1\r\nRent,45\r\nPay,100\r\nAds,12');
  const same=await execute({...blank,snapshot:base,messages:[...first.messages,...again.messages],operation:{action:'import_csv',csv:'Item,Q1\nRent,45'}});
  assert.equal(same.messages.length,0,'nothing differs, so nothing is sent');
});
test('a large write is a few revisions that replay to the same workbook and export whole', async () => {
  const base=(await execute(blank)).workbook;
  const values=Array.from({length:500},(_,r)=>Array.from({length:50},(_,c)=>r*50+c));
  const write=await execute({...blank,snapshot:base,operation:{action:'set_cells',values}});
  assert.ok(write.messages.length<=4,'25,002 commands in revisions of at most 10,000, not one per cell');
  assert.equal(write.messages[0].serverRevisionId,base.revisionId);
  assert.equal(write.messages.at(-1).nextRevisionId,write.workbook.revisionId);
  const replay=await execute({...blank,snapshot:base,messages:write.messages,operation:{action:'export_csv'}});
  assert.deepEqual(replay.workbook,write.workbook);
  const lines=replay.result.csv.trimEnd().split('\r\n');
  assert.equal(lines.length,500);assert.equal(lines.at(-1).split(',').at(-1),'24999');
});
test('a browser paste larger than 10,000 cells is accepted as one revision', async () => {
  const base=(await execute(blank)).workbook;
  const commands=[];
  for(let row=0;row<100;row++)for(let col=0;col<26;col++)for(let n=0;n<6;n++)commands.push({type:'UPDATE_CELL',sheetId:'Sheet1',col,row,content:String(row*26+col+n)});
  const revision={type:'REMOTE_REVISION',version:1,serverRevisionId:base.revisionId,nextRevisionId:randomUUID(),clientId:randomUUID(),commands};
  const accepted=await execute({...blank,snapshot:base,revision});
  assert.equal(accepted.workbook.revisionId,revision.nextRevisionId);
  const read=await execute({...blank,snapshot:base,messages:[revision],operation:{action:'get_range',range:'Z100'}});
  assert.equal(read.result.rows[0][0].value,2604);
});
test('a new workbook imports from CSV or converted XLSX and is the engine\'s own export', async () => {
  const fromCsv=await execute({...blank,import:{csv:'Item,Total\r\nGFX,=1+1\r\n'}});
  const csvRead=await execute({...blank,snapshot:fromCsv.workbook,operation:{action:'get_range',range:'A2:B2'}});
  assert.deepEqual(csvRead.result.rows[0].map(c=>c.value),['GFX',2]);
  const base=(await execute(blank)).workbook;
  const write=await execute({...blank,snapshot:base,operation:{action:'set_cells',values:seed}});
  const source=new Model(write.workbook);
  source.dispatch('CREATE_SHEET',{sheetId:'Sales',name:'Sales',position:1});
  source.dispatch('UPDATE_CELL',{sheetId:'Sales',col:0,row:0,content:'=Sheet1!F2'});
  const files=Object.fromEntries(source.exportXLSX().files.filter(f=>'content' in f).map(f=>[f.path,f.content]));
  const converted={...new Model(files,{mode:'readonly'}).exportData(),revisionId:'chosen-by-client'};
  const imported=await execute({...blank,import:{workbook:converted}});
  assert.notEqual(imported.workbook.revisionId,'chosen-by-client');
  assert.deepEqual(imported.workbook.sheets.map(s=>s.name),['Sheet1','Sales']);
  const read=await execute({...blank,snapshot:imported.workbook,operation:{action:'get_range',worksheet:imported.workbook.sheets[1].id,range:'A1'}});
  assert.equal(read.result.rows[0][0].value,1000);
  await assert.rejects(execute({...blank,import:{workbook:{'[Content_Types].xml':'<Types/>'}}}),/workbook data/);
});
test('invalid final cell yields no accepted partial result; structural limits reject', async () => {
  await assert.rejects(execute({...blank,operation:{action:'set_cells',values:[['valid',{content:{bad:true}}]]}}),/Cell value/);
  await assert.rejects(execute({...blank,operation:{action:'insert_rows',before:1,count:1000000}}),/Invalid row/);
  // A browser revision is bounded before each insertion, so it cannot exhaust the worker first.
  const base=(await execute(blank)).workbook;
  const commands=Array.from({length:200},()=>({type:'ADD_COLUMNS_ROWS',sheetId:'Sheet1',dimension:'ROW',position:'after',base:0,quantity:50000}));
  await assert.rejects(execute({...blank,snapshot:base,revision:{type:'REMOTE_REVISION',version:1,serverRevisionId:base.revisionId,nextRevisionId:randomUUID(),clientId:randomUUID(),commands}}),/dimensions/);
});
test('restoring a saved version in place replays to its cells and worksheets', async () => {
  const base=(await execute(blank)).workbook;
  const first=await execute({...blank,snapshot:base,operation:{action:'set_cells',values:[['Item','Q1'],['Rent',{content:40,format:'0.00'}],['Total','=B2']]}});
  const saved=first.workbook;
  let messages=first.messages;
  const later=[{action:'set_cells',values:[['Changed','Q1'],['Rent',99],[''],['Extra','x']]},{action:'create_worksheet',name:'Scratch'}];
  for(const operation of later){const next=await execute({...blank,snapshot:base,messages,operation});messages=[...messages,...next.messages];}
  const restore=await execute({...blank,snapshot:base,messages,operation:{action:'restore_workbook',workbook:saved}});
  assert.equal(restore.result.worksheets,1);
  const replayed=await execute({...blank,snapshot:base,messages:[...messages,...restore.messages],operation:{action:'snapshot'}});
  const sheet=replayed.result.workbook.sheets;
  assert.deepEqual(sheet.map(s=>s.id),saved.sheets.map(s=>s.id),'the worksheet added later is gone');
  assert.deepEqual(sheet[0].cells,saved.sheets[0].cells);
  assert.deepEqual(sheet[0].formats,saved.sheets[0].formats);
  const again=await execute({...blank,snapshot:base,messages:[...messages,...restore.messages],operation:{action:'restore_workbook',workbook:saved}});
  assert.equal(again.result.changed_cells,0,'restoring the same version again changes nothing');
});
