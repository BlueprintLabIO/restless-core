// The one Restless identity issuer (ADR 0012), as a library. A host owns its
// Better Auth instance (with the organization plugin), its account pages and
// three ports:
//
// - Placement: where a company lives (owner, plane, company and cell ids, Core
//   origin and host, readiness, and optionally its organisation id).
// - Mail: delivered through the host's Better Auth configuration.
// - Store: the membership state behind versioned entry and durable controls.
//   `membership-sql.mjs` is Core's store; Restless Cloud supplies its Fleet one.
//
// The library owns what must mean the same everywhere: who may change whom,
// the membership admin API the company cockpit calls, CORS to that cockpit,
// and the issuer metadata document. Requests and responses are Web-standard,
// so any host can mount `issuer.handle(request)` before its own routes.

export const ISSUER_CONTRACT_VERSION = 1;
export const ADMIN_PREFIX = "/api/admin/v1/companies/";
const UUID =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const MAX_BODY_BYTES = 16384;

export class Refusal extends Error {
  constructor(status, message) {
    super(message);
    this.status = status;
  }
}
export const refuse = (status, message) => {
  throw new Refusal(status, message);
};

export function json(status, data, headers = {}) {
  return new Response(JSON.stringify(data), {
    status,
    headers: {
      "Content-Type": "application/json; charset=utf-8",
      "Cache-Control": "no-store",
      ...headers,
    },
  });
}
/** Bounded JSON body; an empty body is `{}`. */
export async function readJson(request) {
  const text = await request.text();
  if (Buffer.byteLength(text) > MAX_BODY_BYTES) refuse(413, "Request is too large.");
  try {
    return JSON.parse(text || "{}");
  } catch {
    refuse(400, "Request must contain valid JSON.");
  }
}
/** Map a thrown refusal or Better Auth API error to a bounded response. */
export function failure(error, headers = {}) {
  const status =
    error instanceof Refusal ? error.status : Number(error?.statusCode) || 500;
  const message =
    error instanceof Refusal
      ? error.message
      : (status < 500 ? error?.body?.message : null) ||
        "The account service could not complete that request. Please try again.";
  return json(status, { message }, headers);
}

/**
 * The permission rules for one membership change, run by the Store under its
 * own lock against the target's current state. Returns false when the change
 * is already true, so the Store does nothing.
 */
export function authorizeChange(viewer, change, target) {
  if (target.role === "owner" || target.membership_id === viewer.membership_id)
    refuse(403, "You cannot change the company owner or yourself here.");
  if (viewer.role === "admin" && target.role !== "member")
    refuse(403, "Only the owner can change an administrator.");
  if (change.verb === "role") {
    if (viewer.role !== "owner") refuse(403, "Only the owner can change roles.");
    if (!["member", "admin"].includes(change.role))
      refuse(400, "Choose member or administrator access.");
    if (target.status !== "active")
      refuse(409, "Reinstate this person before changing their access.");
    return change.role !== target.role;
  }
  if (change.verb === "suspend") return target.status === "active";
  if (change.verb === "reinstate") {
    if (target.status !== "suspended") refuse(409, "This person is not suspended.");
    return true;
  }
  // Removal always proceeds: a Store may hold a removal whose final step was
  // interrupted (the tombstone recorded, the account row not yet deleted), and
  // repeating the request must complete it rather than report nothing to do.
  if (change.verb === "remove") return true;
  refuse(404, "Membership action not found.");
}

/**
 * @param options.auth           Better Auth instance with the organization plugin
 * @param options.origin         issuer origin, e.g. https://accounts.example.com
 * @param options.placement      { resolve(companyId) -> coordinates | null }
 * @param options.store          {
 *   membership(companyId, userId) -> { membership_id, organization_id, role, status, ending } | null,
 *   members(companyId, organizationId) -> [{ membership_id, name, email, role, status, version, ending }],
 *   change(companyId, membershipId, change, { actorUserId, headers, authorize }) -> { ending },
 * }
 * @param options.invitationLink (invitationId) -> absolute URL the invitee opens to accept
 */
export function createIssuer({ auth, origin, placement, store, invitationLink }) {
  if (typeof auth?.api?.getSession !== "function")
    throw Error("A Better Auth instance is required");
  if (typeof placement?.resolve !== "function")
    throw Error("A Placement port is required");
  for (const method of ["membership", "members", "change"])
    if (typeof store?.[method] !== "function")
      throw Error(`The membership Store must implement ${method}()`);
  if (typeof invitationLink !== "function")
    throw Error("An invitation link builder is required");
  const metadata = {
    contract_version: ISSUER_CONTRACT_VERSION,
    issuer: origin,
    admin_api: origin + "/api/admin/v1",
    account_url: origin + "/",
    roles: ["owner", "admin", "member"],
    terminal_statuses: ["suspended", "removed"],
    invitation_delivery: ["email", "link"],
    role_changes: "owner",
  };

  async function coordinates(companyId) {
    if (!UUID.test(companyId ?? "")) refuse(404, "Company not found.");
    const placed = await placement.resolve(companyId);
    if (!placed) refuse(404, "Company not found.");
    return placed;
  }
  const sessionHeaders = (request) =>
    new Headers({ cookie: request.headers.get("cookie") ?? "" });
  async function account(request) {
    const session = await auth.api.getSession({
      headers: sessionHeaders(request),
      query: { disableCookieCache: true },
    });
    if (!session?.user?.emailVerified)
      refuse(401, "Sign in with a verified email to continue.");
    return session.user;
  }

  async function adminView(request, companyId, viewer) {
    const [members, invitations] = await Promise.all([
      store.members(companyId, viewer.organization_id),
      auth.api.listInvitations({
        headers: sessionHeaders(request),
        query: { organizationId: viewer.organization_id },
      }),
    ]);
    const now = Date.now();
    return {
      company_id: companyId,
      viewer: { membership_id: viewer.membership_id, role: viewer.role },
      members,
      invitations: invitations
        .filter(
          (invitation) =>
            invitation.status === "pending" &&
            new Date(invitation.expiresAt).getTime() > now,
        )
        .slice(0, 100)
        .map((invitation) => ({
          id: invitation.id,
          email: invitation.email,
          role: invitation.role,
          expires_at: invitation.expiresAt,
          link: invitationLink(invitation.id),
        })),
    };
  }

  async function adminAction(request, companyId, rest) {
    const user = await account(request);
    const viewer = await store.membership(companyId, user.id);
    if (!viewer || viewer.status !== "active")
      refuse(403, "You do not currently have access to this company.");
    if (!["owner", "admin"].includes(viewer.role))
      refuse(403, "Only the company owner or an administrator can manage members.");
    if (request.method === "GET" && rest === "members")
      return adminView(request, companyId, viewer);
    if (request.method !== "POST") refuse(404, "Membership action not found.");
    if (!/^application\/json\b/i.test(request.headers.get("content-type") ?? ""))
      refuse(415, "Membership changes must be JSON.");
    const input = await readJson(request);
    const organizationId = viewer.organization_id;

    if (rest === "invitations") {
      if (!["member", "admin"].includes(input.role))
        refuse(400, "Choose member or administrator access.");
      if (input.role === "admin" && viewer.role !== "owner")
        refuse(403, "Only the owner can invite an administrator.");
      const invitation = await auth.api.createInvitation({
        headers: sessionHeaders(request),
        body: {
          email: input.email,
          role: input.role,
          organizationId,
          resend: input.resend === true,
        },
      });
      return {
        invitation: {
          id: invitation.id,
          email: invitation.email,
          role: invitation.role,
          expires_at: invitation.expiresAt,
          link: invitationLink(invitation.id),
        },
      };
    }
    let match = rest.match(/^invitations\/([^/]{1,128})\/cancel$/);
    if (match) {
      const invitation = (
        await auth.api.listInvitations({
          headers: sessionHeaders(request),
          query: { organizationId },
        })
      ).find((candidate) => candidate.id === match[1]);
      if (!invitation) refuse(404, "Invitation not found.");
      if (invitation.role !== "member" && viewer.role !== "owner")
        refuse(403, "Only the owner can cancel an administrator invitation.");
      await auth.api.cancelInvitation({
        headers: sessionHeaders(request),
        body: { invitationId: match[1] },
      });
      return { cancelled: true };
    }
    match = rest.match(/^members\/([^/]{1,128})\/(role|suspend|reinstate|remove)$/);
    if (!match) refuse(404, "Membership action not found.");
    const [, membershipId, verb] = match;
    const change = verb === "role" ? { verb, role: input.role } : { verb };
    return store.change(companyId, membershipId, change, {
      actorUserId: user.id,
      headers: sessionHeaders(request),
      authorize: (target) => authorizeChange(viewer, change, target),
    });
  }

  function cors(request, allowed) {
    const requestOrigin = request.headers.get("origin");
    if (!requestOrigin) return {};
    if (requestOrigin !== allowed) return null;
    return {
      "Access-Control-Allow-Origin": allowed,
      "Access-Control-Allow-Credentials": "true",
      Vary: "Origin",
    };
  }

  /** Answer an issuer route, or return null so the host serves its own. */
  async function handle(request) {
    const path = new URL(request.url).pathname;
    if (request.method === "GET" && path === "/.well-known/restless-issuer")
      return json(200, metadata, { "Cache-Control": "public, max-age=60, must-revalidate" });
    if (!path.startsWith(ADMIN_PREFIX)) return null;
    let headers = {};
    try {
      const [companyId, ...parts] = path.slice(ADMIN_PREFIX.length).split("/");
      const placed = await coordinates(companyId);
      // Only the company's own cockpit may call with the account session.
      const allowed = cors(request, placed.coreOrigin);
      if (!allowed) refuse(403, "This origin cannot manage company members.");
      headers = allowed;
      if (request.method === "OPTIONS")
        return new Response(null, {
          status: 204,
          headers: {
            ...headers,
            "Access-Control-Allow-Methods": "GET, POST",
            "Access-Control-Allow-Headers": "Content-Type",
            "Access-Control-Max-Age": "600",
          },
        });
      if (request.method === "POST" && request.headers.get("origin") !== placed.coreOrigin)
        refuse(403, "Membership changes must come from the company.");
      return json(200, await adminAction(request, companyId, parts.join("/")), headers);
    } catch (error) {
      return failure(error, headers);
    }
  }

  return { handle, metadata };
}
