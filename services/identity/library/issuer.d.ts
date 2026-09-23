// Types for @restless/issuer (ADR 0012). The implementation is src/issuer.mjs.

export declare const ISSUER_CONTRACT_VERSION: 1;
export declare const ADMIN_PREFIX: string;

export type MembershipRole = 'owner' | 'admin' | 'member';
export type MembershipStatus = 'active' | 'suspended' | 'removed';

export declare class Refusal extends Error {
  constructor(status: number, message: string);
  status: number;
}
export declare function refuse(status: number, message: string): never;
export declare function json(status: number, data: unknown, headers?: Record<string, string>): Response;
export declare function readJson(request: Request): Promise<Record<string, unknown>>;
export declare function failure(error: unknown, headers?: Record<string, string>): Response;

/** Where one company lives. */
export interface Coordinates {
  ownerId: string;
  planeId: string;
  companyId: string;
  cellId: string;
  /** The company cockpit origin; the only origin the admin API answers. */
  coreOrigin: string;
  coreHost: string;
  ready?: boolean;
  /** The organisation holding the company's memberships. */
  organizationId?: string;
}
export interface Placement {
  resolve(companyId: string): Promise<Coordinates | null>;
  defaultCompanyId?: string;
}

export interface ViewerMembership {
  membership_id: string;
  organization_id: string;
  role: MembershipRole;
  status: MembershipStatus;
  ending: boolean;
}
export interface MemberView {
  membership_id: string;
  name: string;
  email: string;
  role: MembershipRole;
  status: MembershipStatus;
  version: number;
  /** A suspension or removal Core has not yet confirmed. */
  ending: boolean;
}
export type MembershipChange =
  | { verb: 'role'; role: unknown }
  | { verb: 'suspend' }
  | { verb: 'reinstate' }
  | { verb: 'remove' };
export interface ChangeTarget {
  membership_id: string;
  role: MembershipRole;
  status: MembershipStatus;
}
export interface ChangeOptions {
  actorUserId: string;
  /** The viewer's session, for host Better Auth calls. */
  headers: Headers;
  /** Run under the Store's lock; throws a Refusal, or returns false when nothing changes. */
  authorize(target: ChangeTarget): boolean;
}
export interface MembershipStore {
  membership(companyId: string, userId: string): Promise<ViewerMembership | null>;
  members(companyId: string, organizationId: string): Promise<MemberView[]>;
  change(
    companyId: string,
    membershipId: string,
    change: MembershipChange,
    options: ChangeOptions
  ): Promise<{ ending: boolean }>;
}

export declare function authorizeChange(
  viewer: Pick<ViewerMembership, 'membership_id' | 'role'>,
  change: MembershipChange,
  target: ChangeTarget
): boolean;

export interface IssuerMetadata {
  contract_version: 1;
  issuer: string;
  admin_api: string;
  account_url: string;
  roles: string[];
  terminal_statuses: string[];
  invitation_delivery: string[];
  role_changes: string;
}

export declare function createIssuer(options: {
  /** A Better Auth instance with the organization plugin. */
  auth: any;
  origin: string;
  placement: Placement;
  store: MembershipStore;
  invitationLink(invitationId: string): string;
}): {
  /** Answer an issuer route, or null so the host serves its own. */
  handle(request: Request): Promise<Response | null>;
  metadata: IssuerMetadata;
};
