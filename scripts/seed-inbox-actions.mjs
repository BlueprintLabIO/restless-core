#!/usr/bin/env node
/* Isolated fixtures for US-246. Documents are made through HTTP; only actor-authored review
 * and handoff fixtures use the company's own tables, as seed-reno-scale does for finished Work.
 * No production company, owner profile, model provider or external service is touched. */
import { createConnection } from "node:net";
import { createRequire } from "node:module";
import { readFileSync } from "node:fs";
import { randomUUID } from "node:crypto";
import { join } from "node:path";

const company = process.argv[2];
const home = process.env.RESTLESS_HOME;
const origin = process.env.RESTLESS_SMOKE_ORIGIN;
if (
  !company?.endsWith("_test") ||
  !home?.includes("/.restless-dev/") ||
  !origin ||
  new URL(origin).hostname !== "127.0.0.1"
)
  throw new Error(
    "set RESTLESS_HOME to an isolated dev profile, RESTLESS_SMOKE_ORIGIN to localhost, and pass a _test company",
  );

const base = origin + "/api/companies/" + encodeURIComponent(company);
async function json(path, body, method = "POST") {
  const response = await fetch(
    base + path,
    body
      ? {
          method,
          headers: {
            "Content-Type": "application/json",
            "Idempotency-Key": randomUUID(),
            Origin: origin,
          },
          body: JSON.stringify(body),
        }
      : {},
  );
  const result = await response.json();
  if (!response.ok)
    throw new Error(response.status + ": " + JSON.stringify(result));
  return result;
}
const pg = createRequire(
  new URL("../services/identity/package.json", import.meta.url),
)("pg");
const adminUrl = readFileSync(join(home, "orgintel.toml"), "utf8").match(
  /database_url\s*=\s*"([^"]+)"/,
)[1];
const admin = new pg.Client({ connectionString: adminUrl });
await admin.connect();
const { rows: databases } = await admin.query(
  "SELECT datname FROM pg_database WHERE datname LIKE '%\\_' || $1 || '\\_%' ESCAPE '\\'",
  [company],
);
await admin.end();
if (databases.length !== 1)
  throw new Error("Expected one isolated company database");
const url = new URL(adminUrl);
url.pathname = "/" + databases[0].datname;
const db = new pg.Client({ connectionString: url.toString() });
await db.connect();
await db.query('SET search_path TO "' + company + '"');
try {
  for (const kind of ["review", "collaboration"]) {
    const title = "Inbox footer " + kind + " test";
    const documents = await json("/documents?limit=100");
    let document = documents.items.find(
      (item) => item.document.title === title,
    )?.document;
    if (!document) {
      const created = await json("/documents", {
        title,
        kind: "review",
        visibility: "company",
        linked_room_id: null,
        inherit_room_visibility: false,
        reason: "Inbox footer verification",
        content_json: {
          type: "doc",
          content: Array.from({ length: 40 }, (_, index) => ({
            type: "paragraph",
            attrs: { block_id: randomUUID() },
            content: [
              {
                type: "text",
                text:
                  "Review paragraph " +
                  (index + 1) +
                  ": decision controls stay available while the full context scrolls.",
              },
            ],
          })),
        },
      });
      document = { id: created.document_id };
    }
    const view = await json("/documents/" + document.id);
    if (kind === "review") {
      await db.query(
        "INSERT INTO native_document_reviews (id,document_id,requested_version_id,requested_by_actor_id,reviewer_actor_id,summary) SELECT $1,$2,$3,'exec','owner',$4 WHERE NOT EXISTS (SELECT 1 FROM native_document_reviews WHERE document_id=$2 AND status='requested')",
        [
          randomUUID(),
          document.id,
          view.current_version.version.id,
          "Check the long document before accepting this exact version.",
        ],
      );
    } else {
      await db.query(
        "INSERT INTO native_document_collaboration_requests (id,document_id,requested_by_actor_id,requested_owner_actor_id,summary) SELECT $1,$2,'exec','owner',$3 WHERE NOT EXISTS (SELECT 1 FROM native_document_collaboration_requests WHERE document_id=$2 AND resolved_at IS NULL)",
        [
          randomUUID(),
          document.id,
          "Edit the shared document, then finish from the pane footer.",
        ],
      );
    }
  }
  await db.query(
    "INSERT INTO work (id,owner_id,title,outcome,status,resolution,priority,commissioned_by) SELECT $1,'exec',$2,'The owner can read context and confirm a prepared step','blocked','Waiting for the isolated human step',1,'exec' WHERE NOT EXISTS (SELECT 1 FROM work WHERE title=$2)",
    [randomUUID(), "Inbox footer handoff verification"],
  );
  await db.query(
    "INSERT INTO owner_handoffs (id,work_id,requested_by,assigned_to,category,requested_action,prepared_state,resume_condition) SELECT $1,w.id,'exec',NULL,'mfa',$2,$3,'The owner confirmed the step' FROM work w WHERE w.title='Inbox footer handoff verification' AND NOT EXISTS (SELECT 1 FROM owner_handoffs WHERE work_id=w.id AND state='pending')",
    [
      randomUUID(),
      "Confirm the isolated Inbox footer test step",
      Array.from(
        { length: 30 },
        (_, index) =>
          "Context paragraph " +
          (index + 1) +
          ": read the evidence without losing the decision controls.",
      ).join("\n\n"),
    ],
  );
  await db.query(
    "UPDATE owner_handoffs SET assigned_to=NULL WHERE requested_action='Confirm the isolated Inbox footer test step'",
  );
  const daemon = (request) =>
    new Promise((resolve, reject) => {
      const socket = createConnection(join(home, "restlessd.sock"));
      let buffer = "";
      socket.setEncoding("utf8");
      socket.setTimeout(10_000, () => {
        socket.destroy();
        reject(new Error("Fixture command timed out"));
      });
      socket.on("connect", () =>
        socket.write(
          JSON.stringify({ principal: "owner", company, ...request }) + "\n",
        ),
      );
      socket.on("data", (chunk) => {
        buffer += chunk;
        if (buffer.includes("\n")) {
          socket.end();
          const response = JSON.parse(buffer.split("\n")[0]);
          response.ok
            ? resolve(response.data)
            : reject(new Error(JSON.stringify(response.error)));
        }
      });
      socket.on("error", reject);
    });
  const goalTitle = "Completed goal for owner reliability test";
  let listed = await daemon({ cmd: "goals" });
  let goals = Array.isArray(listed) ? listed : listed.goals;
  if (
    !goals.some((goal) =>
      [goalTitle, "Edited goal reliability fixture"].includes(goal.title),
    )
  ) {
    await daemon({
      cmd: "goal-add",
      title: goalTitle,
      body: "All fixture tasks are done; archiving remains an owner choice.",
      actor: "exec",
    });
    listed = await daemon({ cmd: "goals" });
    goals = Array.isArray(listed) ? listed : listed.goals;
  }
  const completedGoal = goals.find((goal) =>
    [goalTitle, "Edited goal reliability fixture"].includes(goal.title),
  );
  await db.query(
    "INSERT INTO work (id,owner_id,title,outcome,status,resolution,priority,commissioned_by,goal_id) SELECT $1,'exec',$2,'The completed fixture can be archived with its goal','completed','accepted',1,'exec',$3 WHERE NOT EXISTS (SELECT 1 FROM work WHERE title=$2)",
    [randomUUID(), "Completed owner reliability fixture", completedGoal.id],
  );
  let brand = await json("/company/identity");
  const rationale = "Stale brand proposal reliability test";
  if (
    !brand.pending_proposals.some(
      (proposal) => proposal.rationale === rationale,
    )
  ) {
    if (!brand.current_release) {
      await json(
        "/company/identity",
        {
          expected_release: null,
          change_account: "Set the isolated brand fixture",
          statements: [
            {
              id: null,
              pillar: "voice",
              text: "Fixture baseline: quiet and clear.",
              visual: null,
            },
          ],
        },
        "PUT",
      );
      brand = await json("/company/identity");
    }
    await daemon({
      cmd: "identity-propose",
      actor: "exec",
      reason: rationale,
      evidence_ids: brand.release_evidence
        .filter((row) => row.release_id === brand.current_release.id)
        .map((row) => row.evidence_id),
    });
    await json(
      "/company/identity",
      {
        expected_release: brand.current_release.id,
        change_account: "Owner changed the brand after the fixture proposal",
        statements: [
          {
            id: null,
            pillar: "voice",
            text: "The newer owner wording must survive draft recovery.",
            visual: null,
          },
        ],
      },
      "PUT",
    );
  }
  const charter = await json("/company/charter");
  if (!charter.proposal) {
    const request = {
      principal: "owner",
      company,
      cmd: "direction-operation",
      direction_operation: {
        operation: "charter_propose",
        requester: "exec",
        reason: "Inbox footer verification",
        markdown:
          "# Inbox footer verification\n\n" +
          Array.from(
            { length: 45 },
            (_, index) =>
              "Charter rule " +
              (index + 1) +
              ": owner decisions remain available while reading the full context.",
          ).join("\n\n"),
      },
    };
    await daemon(request);
  }
  console.log(
    "Seeded Inbox handoff, charter, review and collaboration fixtures in " +
      company,
  );
} finally {
  await db.end();
}
