// Holds a real isolated plane open for a live role sweep (scripts/multiplayer-smoke --hold).
//
// Nothing about sign-in changes here. It is the same harness the multiplayer journeys use: the
// production account service, Core in network entry, verified accounts created over the emailed
// links, and a member and an admin made through Company › Members. What it adds is that the plane
// stays up afterwards, on a scripted model (tools/mock-model), with each person's signed-in browser
// state saved under <evidence>/hold for Playwright scripts to load:
//
//   <evidence>/hold/ready.json     where everything is: core, accounts, company, state files
//   <evidence>/hold/<role>.json    Playwright storage state for owner, admin, member
//   <evidence>/hold/command        write `restart` (Core stops and starts, as an update does) or `stop`
//
// RESTLESS_MP_SCENARIO picks the mock's scenario; RESTLESS_MP_HOLD_SECONDS bounds the hold (3 hours
// by default). run-core.mjs removes the company, containers, databases and roles when it ends.
import { execFileSync, spawn } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { join } from "node:path";
import {
  accounts,
  base,
  boot,
  cellPool,
  company,
  contexts,
  core,
  coreApi,
  daemonCommand,
  direction,
  enterAccounts,
  evidence,
  invite,
  openCompany,
  pass,
  pause,
  people,
  person,
  repo,
  restartCore,
  root,
  run,
  until,
} from "./multiplayer-harness.mjs";

const hold = join(evidence, "hold");
await mkdir(hold, { recursive: true });
const holdSeconds = Number(process.env.RESTLESS_MP_HOLD_SECONDS ?? 10800);

const mockPort = await new Promise((resolve) => {
  const server = createServer();
  server.listen(0, "127.0.0.1", () => {
    const { port } = server.address();
    server.close(() => resolve(port));
  });
});
const scenario = process.env.RESTLESS_MP_SCENARIO ?? join(repo, "tools/mock-model/scenarios/default.json");
const mock = spawn(
  process.execPath,
  [join(repo, "tools/mock-model/server.mjs"), "--port", String(mockPort), "--scenario", scenario, "--log", join(hold, "mock-calls.jsonl")],
  { stdio: "ignore" },
);
// Core's model gateway and its key check read these; the key is a placeholder the mock accepts.
process.env.RESTLESS_ANTHROPIC_BASE_URL = `http://127.0.0.1:${mockPort}/v1`;
process.env.MP_MOCK_MODEL_KEY = "mock-key-not-a-secret";
// The model broker from this checkout, as restless-dev runs it (scripts/lib/dev-tools.sh).
const tools = join(repo, "infra/host-tools");
process.env.RESTLESS_NODE_BIN = process.execPath;
process.env.RESTLESS_OMP_BIN = join(tools, "omp-source");
process.env.RESTLESS_BUN_BIN = execFileSync("bash", ["-c", 'source "$1/resolve-bun.sh"; host_tools_find_bun "$1"', "_", tools], { encoding: "utf8" }).trim();
await writeFile(
  join(root, "companies", `${company}.toml`),
  `name = "${company}"\nmission = "Renovate small cafes."\nmodel = "anthropic/claude-sonnet-5-5"\nreasoning_effort = "high"\nspend_ceiling_usd = 100\n\n[credentials]\n"model.inference" = "env:MP_MOCK_MODEL_KEY"\n`,
  { mode: 0o600 },
);

const paragraph = (text) => ({ type: "paragraph", attrs: { block_id: randomUUID() }, content: [{ type: "text", text }] });

/** A small company a person could recognise: enough in every surface to open something real. */
async function seed(owner, alice) {
  const documents = [];
  for (const [title, visibility] of [
    ["Harbour Street cafe renovation launch plan", "company"],
    ["Brand voice", "company"],
    ["Owner only salary notes", "participants"],
  ])
    documents.push(
      await coreApi(owner, `${base}/documents`, {
        method: "POST",
        expected: 201,
        body: { title, kind: "brief", visibility, content_json: { type: "doc", content: [paragraph(`${title}. First draft.`)] }, reason: "Role sweep fixture" },
      }),
    );
  await coreApi(owner, `${base}/sheets`, { method: "POST", body: { id: randomUUID(), title: "Budget", visibility: "company" } });
  await coreApi(owner, `${base}/goals`, { method: "POST", expected: 201, body: { objective: "Open the Harbour Street cafe by spring" } });
  await daemonCommand({ cmd: "actor-create", company, as_actor: "copy-critic", role: "Copywriter", name: "Wren", actor: "exec", reason: "Role sweep fixture" });
  await daemonCommand({ cmd: "team-create", company, name: "Website", to: "copy-critic", body: "Copy is plain.", actor: "exec" });
  const room = await coreApi(owner, `${base}/rooms`, {
    method: "POST",
    body: { kind: "group", title: "Launch room", command_id: randomUUID(), participant_actor_ids: [alice.principal.actor_id] },
  });
  await coreApi(owner, `${base}/rooms/${room.id}/messages`, { method: "POST", expected: 201, body: { body: "Kickoff at nine.", command_id: randomUUID() } });
  const requester = alice.principal.actor_id;
  await direction({ operation: "request_file", requester, class: null, team: null, summary: "Raise the ads budget to $200", change: "Monthly ads envelope from $100 to $200", reason: "Launch week" });
  await direction({ operation: "request_file", requester, class: null, team: null, summary: "Open a second supplier account", change: "A second joinery supplier on account", reason: "Quotes" });
  // A finished Work with a real file on the company computer, so a Library link has something to open.
  const work = randomUUID();
  await cellPool.query(
    `INSERT INTO "${company}".work (id, owner_id, title, outcome, status, resolution, created_at, updated_at, priority, commissioned_by)
     VALUES ($1, 'exec', 'Harbour Street send pack', 'A pack to send.', 'completed', 'accepted', now(), now(), 1, 'exec')`,
    [work],
  );
  await cellPool.query(
    `INSERT INTO "${company}".artifact_refs (id, kind, uri, created_by, created_at, work_id, label)
     VALUES ($1, 'document', '/company/outputs/role-test/final.md', 'exec', now(), $2, 'final.md')`,
    [randomUUID(), work],
  );
  const computer = `restless-${process.env.RESTLESS_RESOURCE_NAMESPACE}-pc-${company}`;
  await until(() => {
    try {
      execFileSync("docker", ["exec", computer, "sh", "-c", 'mkdir -p /company/outputs/role-test && { echo "# Final pack"; echo; echo "Ready to send."; } > /company/outputs/role-test/final.md && chown -R company:company /company/outputs/role-test'], { stdio: "ignore" });
      return true;
    } catch {
      return false;
    }
  }, "the company computer takes a file", 120);
  // A judgement the owner can hand to a member: blocked Work with a handoff escalated to the Inbox.
  const judgedWork = randomUUID();
  const judgement = randomUUID();
  await cellPool.query(
    `INSERT INTO "${company}".work (id, owner_id, title, outcome, status, resolution, created_at, updated_at, priority, commissioned_by)
     VALUES ($1, 'exec', 'Choose the joinery supplier', 'A supplier is chosen.', 'blocked', 'Waiting for a decision', now(), now(), 2, 'exec')`,
    [judgedWork],
  );
  // A trigger on owner_handoffs reads `actors` unqualified, so this connection names the schema.
  const client = await cellPool.connect();
  try {
    await client.query(`SET search_path TO "${company}"`);
    await client.query(
      `INSERT INTO owner_handoffs (id, work_id, requested_by, assigned_to, category, requested_action, prepared_state, resume_condition)
       VALUES ($1, $2, 'exec', 'exec', 'owner_judgement', 'Choose the joinery supplier', 'Harbour Joinery quotes $18,400 and Bay Timber $21,000.', 'A person chooses a supplier')`,
      [judgement, judgedWork],
    );
  } finally {
    await client.query("RESET search_path");
    client.release();
  }
  await daemonCommand({
    cmd: "work-handoff-prepare-brief", company, id: judgement, as_actor: "exec", owner_kind: "decision",
    headline: "Choose the joinery supplier",
    situation: "Harbour Joinery quotes $18,400 and Bay Timber $21,000.",
    impact: "The fit-out waits for this.",
    recommendation: "Harbour Joinery: cheaper and local.",
    no_action: "The fit-out stays on hold.",
  });
  await daemonCommand({ cmd: "work-handoff-escalate", company, id: judgement, as_actor: "exec", reason: "Needs a person." });
  pass("seeded documents (one restricted), a sheet, a goal, a team, a room, two requests and a judgement for the Inbox");
  return { documents, room, judgedWork, judgement };
}

await run("hold", async () => {
  const owner = await boot();
  const alice = await person("alice");
  const bob = await person("bob");
  await invite(owner, [alice, bob]);
  const seeded = await seed(owner, alice);

  // Bob becomes an admin the way a person does it: from Members, then he re-enters.
  await bob.page.goto("about:blank");
  await owner.page.goto(`${core}/${company}/company/members`);
  const access = owner.page.locator(`select[aria-label="Access for ${people.bob.name}"]`);
  await access.waitFor({ timeout: 30000 });
  await access.selectOption("admin");
  await owner.page.getByText(`${people.bob.name} is now an admin.`).waitFor({ timeout: 15000 });
  await until(
    async () => (await bob.context.request.fetch(`${core}${base}/principal`, { failOnStatusCode: false })).status() === 401,
    "Bob's session from before the change ends",
    30,
  );
  await enterAccounts(bob);
  await bob.page.locator("#company").waitFor({ state: "visible" });
  await openCompany(bob);
  pass(`roles: owner ${owner.principal.membership_role}, admin ${bob.principal.membership_role}, member ${alice.principal.membership_role}`);

  for (const [role, who] of [["owner", owner], ["admin", bob], ["member", alice]]) {
    await who.page.goto("about:blank");
    await who.context.storageState({ path: join(hold, `${role}.json`) });
  }
  await writeFile(
    join(hold, "ready.json"),
    JSON.stringify({
      core,
      accounts,
      company,
      root,
      namespace: process.env.RESTLESS_RESOURCE_NAMESPACE,
      offset: Number(process.env.RESTLESS_PORT_OFFSET),
      actors: { owner: owner.principal.actor_id, admin: bob.principal.actor_id, member: alice.principal.actor_id },
      documents: seeded.documents.map((item) => item.document_id ?? item.document?.id),
      room: seeded.room.id,
      judgement: seeded.judgement,
      judgedWork: seeded.judgedWork,
    }, null, 2),
    { mode: 0o600 },
  );
  pass("the plane is up and each person's browser state is saved; holding");

  const command = join(hold, "command");
  const until_ = Date.now() + holdSeconds * 1000;
  while (Date.now() < until_) {
    const text = (await readFile(command, "utf8").catch(() => "")).trim();
    if (text) await rm(command, { force: true });
    if (text === "stop") break;
    if (text === "restart") {
      await restartCore();
      await writeFile(join(hold, "restarted"), new Date().toISOString());
      console.log("core restarted");
    }
    await pause(500);
  }
  contexts.length = 0;
}).finally(() => mock.kill());
