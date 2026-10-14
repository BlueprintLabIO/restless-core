// A guest (UXG-070, issuer 1.3.0) is someone outside the company given one document or sheet.
// Owners and admins invite them; a guest sees nothing of the membership API but leaving, cannot
// invite or change anyone, and is never made a member or the owner by a role change.
import test from "node:test";
import assert from "node:assert/strict";
import { authorizeChange, authorizeOwnershipTransfer, createIssuer, Refusal } from "../src/issuer.mjs";
import { MEMBERSHIP_SCHEMA } from "../src/membership-sql.mjs";

const COMPANY = "0b8f1f8e-5d0a-4a52-9d61-2c3c8c4c9a10";
const CORE = "https://work.example.test";

function issuerFor(role, { guests = true } = {}) {
  const invited = [];
  const changed = [];
  const auth = {
    api: {
      getSession: async () => ({ user: { id: "u1", emailVerified: true } }),
      listInvitations: async () => [],
      createInvitation: async ({ body }) => {
        invited.push(body);
        return { id: "inv1", email: body.email, role: body.role, expiresAt: new Date(Date.now() + 1e6) };
      },
    },
  };
  const store = {
    membership: async () => ({ membership_id: "m1", organization_id: "org1", role, status: "active", ending: false }),
    members: async () => [{ membership_id: "m1", role, status: "active" }],
    change: async (companyId, membershipId, change, { authorize }) => {
      authorize({ membership_id: membershipId, role: "member", status: "active" });
      changed.push([membershipId, change]);
      return { ending: false };
    },
  };
  const issuer = createIssuer({
    auth,
    origin: "https://accounts.example.test",
    placement: { resolve: async () => ({ coreOrigin: CORE, organizationId: "org1" }) },
    store,
    invitationLink: (id) => `https://accounts.example.test/?invite=${id}`,
    guests,
  });
  const call = async (method, rest, body) => {
    const response = await issuer.handle(
      new Request(`https://accounts.example.test/api/admin/v1/companies/${COMPANY}/${rest}`, {
        method,
        headers: { origin: CORE, "content-type": "application/json" },
        body: body ? JSON.stringify(body) : undefined,
      }),
    );
    return { status: response.status, body: await response.json() };
  };
  return { issuer, call, invited, changed };
}

test("owners and admins invite a guest; members and guests cannot invite anyone", async () => {
  for (const role of ["owner", "admin"]) {
    const { call, invited } = issuerFor(role);
    const { status, body } = await call("POST", "invitations", { email: "accountant@example.test", role: "guest" });
    assert.equal(status, 200, `${role} invites a guest`);
    assert.equal(body.invitation.role, "guest");
    assert.equal(invited[0].role, "guest");
  }
  for (const role of ["member", "guest"]) {
    const { call, invited } = issuerFor(role);
    for (const invite of ["guest", "member"]) {
      const { status } = await call("POST", "invitations", { email: "x@example.test", role: invite });
      assert.equal(status, 403, `${role} cannot invite a ${invite}`);
    }
    assert.equal(invited.length, 0);
  }
});

test("a host without the guest role neither offers nor accepts one", async () => {
  const { issuer, call, invited } = issuerFor("owner", { guests: false });
  assert.deepEqual(issuer.metadata.roles, ["owner", "admin", "member"]);
  const { status } = await call("POST", "invitations", { email: "x@example.test", role: "guest" });
  assert.equal(status, 400);
  assert.equal(invited.length, 0);
  assert.ok(issuerFor("owner").issuer.metadata.roles.includes("guest"));
});

test("a guest sees nothing of the membership API but leaving", async () => {
  const { call, changed } = issuerFor("guest");
  assert.equal((await call("GET", "members")).status, 403, "not who is in the company");
  for (const rest of ["members/m2/role", "members/m2/remove", "members/m2/suspend", "members/m2/make-owner", "invitations/inv1/cancel"])
    assert.equal((await call("POST", rest, { role: "member" })).status, 403, rest);
  assert.equal(changed.length, 0);
  assert.equal((await call("POST", "members/self/leave", {})).status, 200, "a guest may leave");
});

test("no role change makes a guest a member, an admin or the owner", () => {
  const guest = { membership_id: "g1", role: "guest", status: "active" };
  const refusal = (run) => assert.throws(run, (error) => error instanceof Refusal && error.status >= 400);
  for (const role of ["member", "admin", "owner", "guest"])
    refusal(() => authorizeChange({ membership_id: "o1", role: "owner" }, { verb: "role", role }, guest));
  refusal(() => authorizeChange({ membership_id: "o1", role: "owner" }, { verb: "role", role: "guest" }, { ...guest, role: "member" }));
  refusal(() =>
    authorizeOwnershipTransfer(
      { membership_id: "o1", role: "owner" },
      { membership_id: "o1", role: "owner", status: "active" },
      guest,
    ),
  );
  // An admin removes or suspends a guest as they would a member, and still cannot touch an admin.
  assert.equal(authorizeChange({ membership_id: "a1", role: "admin" }, { verb: "remove" }, guest), true);
  assert.equal(authorizeChange({ membership_id: "a1", role: "admin" }, { verb: "suspend" }, guest), true);
  refusal(() => authorizeChange({ membership_id: "a1", role: "admin" }, { verb: "remove" }, { ...guest, role: "admin" }));
});

test("the Store's role checks hold a guest, and existing installs are widened", () => {
  assert.equal((MEMBERSHIP_SCHEMA.match(/role IN \('owner','admin','member','guest'\)/g) ?? []).length, 4);
  assert.ok(!/role IN \('owner','admin','member'\)\)/.test(MEMBERSHIP_SCHEMA));
});
