import type { CoreResponse } from './membership-sql';
/** Node HTTP(S) request to a Core plane, resolving `.localhost` to loopback. */
export declare function coreRequest(
  url: string,
  options: { method: string; headers: Record<string, string>; body?: string }
): Promise<CoreResponse>;
