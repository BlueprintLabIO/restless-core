// Types for Core's PostgreSQL membership Store (ADR 0012).
import type { MembershipStore, Placement, ViewerMembership } from './issuer';

/** Store tables. A host runs these once, after Better Auth's migrations. */
export declare const MEMBERSHIP_SCHEMA: string;

export interface CoreResponse {
  ok: boolean;
  json(): Promise<any>;
}
export type FetchCore = (
  url: string,
  options: { method: string; headers: Record<string, string>; body: string; redirect?: string; signal?: AbortSignal }
) => Promise<CoreResponse>;

export interface Ed25519PrivateJwk {
  kty: 'OKP';
  crv: 'Ed25519';
  x: string;
  d: string;
  kid: string;
}

export interface EntryHandoff {
  /** The Core plane's entry URL; the browser posts `assertion` there. */
  action: string;
  assertion: string;
}

export interface SqlMembershipStore extends MembershipStore {
  membership(
    companyId: string,
    userId: string
  ): Promise<(ViewerMembership & { user_id: string; version: number }) | null>;
  /** A single-use entry handoff; refuses unless the member is active and no terminal control is pending. */
  enter(user: { id: string; name?: string | null }, companyId: string): Promise<EntryHandoff>;
  jwks: { keys: Array<Record<string, string>> };
  deliverDue(): Promise<void>;
  close(): Promise<void>;
}

export declare function createSqlMembershipStore(options: {
  /** A pg Pool on the database that holds Better Auth's tables. */
  pool: any;
  /** The issuer origin, the `iss` of every signed token. */
  origin: string;
  signingKey: Ed25519PrivateJwk;
  placement: Placement;
  fetchCore?: FetchCore;
  deliveryIntervalMs?: number;
}): Promise<SqlMembershipStore>;
