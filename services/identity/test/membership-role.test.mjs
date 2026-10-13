// A role change reaches Core at once (issuer 1.1.1): it enqueues an active membership control with
// the new role and the next version, as suspension and removal enqueue theirs. Before, the role
// waited for the person's next entry, so a demoted admin kept admin until then.
import test from "node:test";
import assert from "node:assert/strict";
import { exportJWK, generateKeyPair } from "jose";
import { createSqlMembershipStore } from "../src/membership-sql.mjs";

const COMPANY = "0b8f1f8e-5d0a-4a52-9d61-2c3c8c4c9a10";

async function storeWith(state) {
  const queries = [];
  const client = {
    async query(sql, params = []) {
      queries.push({ sql, params });
      if (/FROM member WHERE "organizationId" = \$1 AND id = \$2 FOR UPDATE/.test(sql))
        return { rows: [{ membership_id: "m1", user_id: "u1", organization_id: "org1", role: state.role }] };
      if (/SELECT status, version FROM restless_membership_state/.test(sql))
        return { rows: [{ status: state.status, version: state.version }] };
      return { rows: [] };
    },
    release() {},
  };
  const pool = {
    connect: async () => client,
    // Delivery claims nothing here: the control stays in the outbox, as when Core is away.
    query: async (sql, params) => client.query(sql, params),
  };
  const { privateKey } = await generateKeyPair("EdDSA", { extractable: true });
  const signingKey = { ...(await exportJWK(privateKey)), kid: "test" };
  const store = await createSqlMembershipStore({
    pool,
    origin: "http://issuer.test",
    signingKey,
    placement: { resolve: async () => ({ organizationId: "org1" }) },
    fetchCore: async () => {
      throw new Error("Core is not called in this test");
    },
    deliveryIntervalMs: 60_000,
  });
  return { store, queries };
}

const controls = (queries) =>
  queries.filter((query) => /INSERT INTO restless_membership_controls/.test(query.sql));

test("a role change enqueues an active control with the new role and the next version", async () => {
  const { store, queries } = await storeWith({ role: "member", status: "active", version: 3 });
  const result = await store.change(COMPANY, "m1", { verb: "role", role: "admin" }, { authorize: () => true });
  await store.close();
  const [control] = controls(queries);
  assert.ok(control, "a control is enqueued");
  const [, membership, company, user, role, status, version] = control.params;
  assert.deepEqual([membership, company, user, role, status, version], ["m1", COMPANY, "u1", "admin", "active", 4]);
  assert.ok(
    queries.some((query) => /UPDATE member SET role=\$2/.test(query.sql) && query.params[1] === "admin"),
    "the member row takes the new role",
  );
  assert.deepEqual(result, { ending: false }, "a role change in flight ends nothing");
});

test("taking a role back is delivered the same way", async () => {
  const { store, queries } = await storeWith({ role: "admin", status: "active", version: 7 });
  await store.change(COMPANY, "m1", { verb: "role", role: "member" }, { authorize: () => true });
  await store.close();
  const [control] = controls(queries);
  assert.deepEqual(control.params.slice(4), ["member", "active", 8]);
});

test("an unauthorised change enqueues nothing", async () => {
  const { store, queries } = await storeWith({ role: "member", status: "active", version: 1 });
  await store.change(COMPANY, "m1", { verb: "role", role: "admin" }, { authorize: () => false });
  await store.close();
  assert.equal(controls(queries).length, 0);
});

test("a role change never reinstates a suspended member", async () => {
  const { store, queries } = await storeWith({ role: "member", status: "suspended", version: 5 });
  await store.change(COMPANY, "m1", { verb: "role", role: "admin" }, { authorize: () => true });
  await store.close();
  const [control] = controls(queries);
  assert.deepEqual(control.params.slice(4), ["admin", "suspended", 6]);
});
