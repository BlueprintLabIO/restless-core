import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import {
  probe,
  install,
  installationStatus,
  environment,
  harnessRoot,
} from "./manage.mjs";

function workspace(t) {
  const root = fs.mkdtempSync(
    path.join(os.tmpdir(), "restless-custom-harness_test-"),
  );
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  return root;
}
function fixture(root, mode) {
  const filename = path.join(root, "acp-fixture.mjs");
  fs.writeFileSync(
    filename,
    `
import readline from 'node:readline';
import fs from 'node:fs';
const mode = ${JSON.stringify(mode)};
const model = 'custom:example:vendor/model';
const option = value => ({id:'choose-model',category:'model',currentValue:value,options:[{group:'Provider',options:[{value:model,name:'模型'}]}]});
const send = value => {
 const data = Buffer.from(JSON.stringify(value)+'\\n');
 if(mode==='chunked') {let i=0;const write=()=>{if(i<data.length){process.stdout.write(data.subarray(i,i+1));i++;setImmediate(write);}};write();}
 else process.stdout.write(data);
};
for await (const line of readline.createInterface({input:process.stdin})) {
 const m=JSON.parse(line);
 if(m.error && m.id===999) {fs.writeFileSync('tool-denied',m.error.message); continue;}
 if(mode==='hang') continue;
 if(mode==='exit') process.exit(1);
 if(mode==='invalid') {process.stdout.write('not JSON\\n');continue;}
 if(mode==='secret') {send({id:m.id,error:{message:'oops '+process.env.FIXTURE_SECRET}});continue;}
 let result={};
 if(m.method==='initialize') result={protocolVersion:1,agentInfo:{name:'fixture'},agentCapabilities:{},authMethods:[]};
 if(m.method==='session/new') {
   if(mode==='tool') send({jsonrpc:'2.0',id:999,method:'fs/write_text_file',params:{path:'forbidden',content:'bad'}});
   if(mode==='frames') process.stdout.write(JSON.stringify({method:'notice',params:{padding:'x'.repeat(400)}})+'\\n');
   result={sessionId:'fixture-session',configOptions:mode==='empty'?[]:[option('before')]};
   if(mode==='legacy') {delete result.configOptions;result.models={currentModelId:'before',availableModels:[{modelId:model,name:'Legacy'}]};}
 }
 if(m.method==='session/set_config_option') result={configOptions:[option(mode==='ignores-selection'?'before':m.params.value)]};
 send({jsonrpc:'2.0',id:m.id,result});
}
`,
  );
  return { command: [process.execPath, filename] };
}
const selected = "custom:example:vendor/model";

test("ACP discovery preserves chunked UTF-8 and exact model IDs, then confirms selection", async (t) => {
  const root = workspace(t);
  const result = await probe(root, {
    config: fixture(root, "chunked"),
    model: selected,
  });
  assert.equal(result.state, "compatible");
  assert.equal(result.models[0].name, "模型");
  assert.equal(result.selected_model, selected);
  assert.equal(result.model_selection, "config_option");
});
test("installation/protocol success alone does not claim selectable models", async (t) => {
  const root = workspace(t);
  const result = await probe(root, { config: fixture(root, "empty") });
  assert.equal(result.state, "setup_required");
  assert.equal(result.model_selection, "unavailable");
});
test("legacy ACP selection is explicitly identified", async (t) => {
  const root = workspace(t);
  const result = await probe(root, {
    config: fixture(root, "legacy"),
    model: selected,
  });
  assert.equal(result.state, "compatible");
  assert.equal(result.model_selection, "legacy");
  assert.equal(result.selected_model, selected);
});
test("refuses an ignored exact-model change", async (t) => {
  const root = workspace(t);
  const result = await probe(root, {
    config: fixture(root, "ignores-selection"),
    model: selected,
  });
  assert.equal(result.state, "setup_required");
  assert.match(result.message, /did not confirm/);
});
test("probe refuses harness requests to change files", async (t) => {
  const root = workspace(t);
  const result = await probe(root, {
    config: fixture(root, "tool"),
    model: selected,
  });
  assert.equal(result.state, "compatible");
  assert.equal(fs.existsSync(path.join(root, "forbidden")), false);
  assert.match(
    fs.readFileSync(path.join(root, "tool-denied"), "utf8"),
    /does not authorize/,
  );
});
test("frame size limit applies per line rather than aggregated reads", async (t) => {
  const root = workspace(t);
  const result = await probe(
    root,
    { config: fixture(root, "frames") },
    { maxFrameBytes: 512 },
  );
  assert.equal(result.state, "compatible");
});
test("terminated, invalid and silent harnesses return bounded failures", async (t) => {
  for (const mode of ["exit", "invalid", "hang"]) {
    const root = workspace(t);
    const result = await probe(
      root,
      { config: fixture(root, mode) },
      { timeoutMs: 150 },
    );
    assert.equal(result.state, "setup_required");
    assert.match(result.message, /exited|JSON|timed out/);
  }
});
test("redacts supplied credentials from protocol failures", async (t) => {
  const root = workspace(t);
  const result = await probe(root, {
    config: fixture(root, "secret"),
    env: { FIXTURE_SECRET: "private-example-value" },
    secret_names: ["FIXTURE_SECRET"],
  });
  assert.equal(result.message.includes("private-example-value"), false);
  assert.match(result.message, /redacted/);
});
test("preserves model-setting values while redacting only declared credentials", async (t) => {
  const root = workspace(t);
  const result = await probe(root, {
    config: fixture(root, "legacy"),
    env: {
      TEST_MODEL: "custom:example:vendor/model",
      FIXTURE_SECRET: "private-example-value",
    },
    secret_names: ["FIXTURE_SECRET"],
    model: "custom:example:vendor/model",
  });
  assert.equal(result.models[0].id, "custom:example:vendor/model");
  assert.equal(result.selected_model, "custom:example:vendor/model");
});
test("isolates profiles and does not inherit actor or model credentials", (t) => {
  const root = workspace(t);
  process.env.RESTLESS_SESSION_CAPABILITY = "example-capability";
  process.env.OPENAI_API_KEY = "unrelated";
  t.after(() => {
    delete process.env.RESTLESS_SESSION_CAPABILITY;
    delete process.env.OPENAI_API_KEY;
  });
  const env = environment(root, {
    HOME: "/bad",
    OPENCLAW_STATE_DIR: "/bad",
    CUSTOM_KEY: "specific",
  });
  assert.equal(env.HOME, path.join(root, "home"));
  assert.equal(env.OPENCLAW_STATE_DIR, path.join(root, "home/.openclaw"));
  assert.equal(env.RESTLESS_SESSION_CAPABILITY, undefined);
  assert.equal(env.OPENAI_API_KEY, undefined);
  assert.equal(env.CUSTOM_KEY, "specific");
  assert.throws(() => harnessRoot("../escape"));
});
test("flock serializes concurrent installs and reports real child failure", async (t) => {
  const root = workspace(t);
  const first = install(root, {
    install: "sleep 0.5; printf installed > proof",
  });
  for (
    let i = 0;
    i < 100 && installationStatus(root).state !== "installing";
    i++
  )
    await new Promise((r) => setTimeout(r, 10));
  assert.equal(installationStatus(root).state, "installing");
  await assert.rejects(
    install(root, { install: "printf duplicate > bad" }),
    /already running/,
  );
  assert.equal((await first).state, "installed");
  assert.equal(fs.existsSync(path.join(root, "bad")), false);
  assert.equal(fs.readFileSync(path.join(root, "proof"), "utf8"), "installed");
  await assert.rejects(
    install(root, {
      install: 'printf "$CUSTOM_SECRET"; exit 9',
      env: { CUSTOM_SECRET: "private-example-value" },
    }),
  );
  const failed = installationStatus(root);
  assert.equal(failed.state, "failed");
  assert.match(failed.message, /exit 9/);
  assert.equal(failed.log, "[redacted]");
});
test("status recognizes an interrupted installer without modifying its record", (t) => {
  const root = workspace(t);
  const state = {
    state: "installing",
    pid: process.pid,
    process_start: "wrong-instance",
  };
  fs.writeFileSync(path.join(root, "install.json"), JSON.stringify(state));
  assert.equal(installationStatus(root).state, "interrupted");
  assert.deepEqual(
    JSON.parse(fs.readFileSync(path.join(root, "install.json"))),
    state,
  );
});
