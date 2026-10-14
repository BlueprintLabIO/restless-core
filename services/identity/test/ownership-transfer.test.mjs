// Handing the company on (UXG-100): the owner makes an active admin or member the owner, and becomes
// an admin, in one transaction. Both changes reach Core as active controls; a refusal changes nothing.
import test from "node:test";
import assert from "node:assert/strict";
import { exportJWK, generateKeyPair } from "jose";
import { createSqlMembershipStore } from "../src/membership-sql.mjs";
import { authorizeOwnershipTransfer, Refusal } from "../src/issuer.mjs";

const COMPANY = "0b8f1f8e-5d0a-4a52-9d61-2c3c8c4c9a10";

async function storeWith(people) {
  const queries = [];
  const client = {
    async query(sql, params = []) {
      queries.push({ sql, params });
      if (/FROM member WHERE "organizationId" = \$1 AND id = \$2 FOR UPDATE/.test(sql)) {
        const person = people[params[1]];
        return {
          rows: person
            ? [{ membership_id: params[1], user_id: `u-${params[1]}`, organization_id: "org1", role: person.role }]
            : [],
        };
      }
      if (/SELECT status, version FROM restless_membership_state/.test(sql)) {
        const person = people[params[0]];
        return { rows: [{ status: person.status, version: person.version }] };
      }
      return { rows: [] };
    },
    release() {},
  };
  const pool = { connect: async () => client, query: async (sql, params) => client.query(sql, params) };
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
  queries.filter((query) => /INSERT INTO restless_membership_controls/.test(query.sql)).map((query) => query.params);
const roleWrites = (queries) =>
  queries.filter((query) => /^UPDATE member SET role=/.test(query.sql)).map((query) => [query.sql.match(/'(\w+)'/)[1], query.params[0]]);
const viewer = { membership_id: "owner1", role: "owner" };

test("the owner makes an admin the owner and becomes an admin, each with its next version", async () => {
  const { store, queries } = await storeWith({
    owner1: { role: "owner", status: "active", version: 4 },
    admin1: { role: "admin", status: "active", version: 2 },
  });
  const result = await store.transferOwnership(COMPANY, "owner1", "admin1", {
    authorize: (current, target) => authorizeOwnershipTransfer(viewer, current, target),
  });
  await store.close();
  assert.deepEqual(result, { ending: false });
  assert.deepEqual(roleWrites(queries), [["owner", "admin1"], ["admin", "owner1"]]);
  assert.deepEqual(
    controls(queries).map(([, membership, , , role, status, version]) => [membership, role, status, version]),
    [["admin1", "owner", "active", 3], ["owner1", "admin", "active", 5]],
    "the new owner is told first; neither control ends access",
  );
  assert.ok(queries.some((query) => query.sql === "COMMIT"), "both change in one transaction");
});

test("a refused transfer writes nothing and rolls back", async () => {
  for (const [people, status] of [
    // Only the owner hands the company on.
    [{ owner1: { role: "admin", status: "active", version: 1 }, admin1: { role: "member", status: "active", version: 1 } }, 403],
    // Only to someone active.
    [{ owner1: { role: "owner", status: "active", version: 1 }, admin1: { role: "member", status: "suspended", version: 1 } }, 409],
  ]) {
    const { store, queries } = await storeWith(people);
    await assert.rejects(
      store.transferOwnership(COMPANY, "owner1", "admin1", {
        authorize: (current, target) => authorizeOwnershipTransfer(viewer, current, target),
      }),
      (error) => error instanceof Refusal && error.status === status,
    );
    await store.close();
    assert.deepEqual(roleWrites(queries), []);
    assert.deepEqual(controls(queries), []);
    assert.ok(queries.some((query) => query.sql === "ROLLBACK"));
  }
});

test("someone outside the company cannot be made the owner", async () => {
  const { store, queries } = await storeWith({ owner1: { role: "owner", status: "active", version: 1 } });
  await assert.rejects(
    store.transferOwnership(COMPANY, "owner1", "stranger", {
      authorize: (current, target) => authorizeOwnershipTransfer(viewer, current, target),
    }),
    (error) => error instanceof Refusal && error.status === 404,
  );
  await store.close();
  assert.deepEqual(controls(queries), []);
});

test("a repeat after the handover happened succeeds and writes nothing", async () => {
  // Finish pressed again, or a retry after a lost response: the former owner is an admin now.
  const { store, queries } = await storeWith({
    owner1: { role: "admin", status: "active", version: 5 },
    admin1: { role: "owner", status: "active", version: 3 },
  });
  const formerOwner = { membership_id: "owner1", role: "admin" };
  const result = await store.transferOwnership(COMPANY, "owner1", "admin1", {
    authorize: (current, target) => authorizeOwnershipTransfer(formerOwner, current, target),
  });
  await store.close();
  assert.deepEqual(result, { ending: false });
  assert.deepEqual(roleWrites(queries), []);
  assert.deepEqual(controls(queries), []);
});

test("the repeat path admits nobody else", async () => {
  for (const [people, caller] of [
    // A member cannot use it to learn or touch anything.
    [{ owner1: { role: "member", status: "active", version: 1 }, admin1: { role: "owner", status: "active", version: 1 } }, "member"],
    // An admin cannot make another admin the owner.
    [{ owner1: { role: "admin", status: "active", version: 1 }, admin1: { role: "admin", status: "active", version: 1 } }, "admin"],
    // A suspended former owner gets nothing.
    [{ owner1: { role: "admin", status: "suspended", version: 1 }, admin1: { role: "owner", status: "active", version: 1 } }, "admin"],
  ]) {
    const { store, queries } = await storeWith(people);
    await assert.rejects(
      store.transferOwnership(COMPANY, "owner1", "admin1", {
        authorize: (current, target) =>
          authorizeOwnershipTransfer({ membership_id: "owner1", role: caller }, current, target),
      }),
      (error) => error instanceof Refusal && error.status === 403,
    );
    await store.close();
    assert.deepEqual(roleWrites(queries), []);
    assert.deepEqual(controls(queries), []);
  }
});
