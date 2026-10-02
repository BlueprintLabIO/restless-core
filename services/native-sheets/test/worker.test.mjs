import test from 'node:test';
import assert from 'node:assert/strict';
import { execute, ENGINE_VERSION } from '../src/worker.mjs';
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
test('invalid final cell yields no accepted partial result; structural limits reject', async () => {
  await assert.rejects(execute({...blank,operation:{action:'set_cells',values:[['valid',{content:{bad:true}}]]}}),/Cell value/);
  await assert.rejects(execute({...blank,operation:{action:'insert_rows',before:1,count:1000000}}),/Invalid row/);
});
