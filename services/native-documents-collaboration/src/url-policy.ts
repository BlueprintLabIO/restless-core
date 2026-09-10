import { isIP } from 'node:net';

import { JWKS_PATH } from './constants.js';

const PRIVATE_SERVICE_NAME = /^[a-z0-9](?:[a-z0-9-]{0,61}[a-z0-9])?$/;

function unwrappedHostname(hostname: string): string {
  return hostname.startsWith('[') && hostname.endsWith(']') ? hostname.slice(1, -1) : hostname;
}

export function isLoopbackHostname(hostname: string): boolean {
  if (hostname === 'localhost') return true;
  const unwrapped = unwrappedHostname(hostname);
  if (isIP(unwrapped) === 6) return unwrapped === '::1';
  return isIP(unwrapped) === 4 && unwrapped.split('.')[0] === '127';
}

function isPrivateServiceHostname(hostname: string): boolean {
  if (isLoopbackHostname(hostname)) return true;
  const unwrapped = unwrappedHostname(hostname).toLowerCase();
  if (isIP(unwrapped) === 4) {
    const octets = unwrapped.split('.').map(Number);
    const first = octets[0];
    const second = octets[1];
    return first === 10 ||
      (first === 172 && second !== undefined && second >= 16 && second <= 31) ||
      (first === 192 && second === 168);
  }
  if (isIP(unwrapped) === 6) {
    return /^(?:fc|fd)/u.test(unwrapped) || /^fe[89ab]/u.test(unwrapped);
  }
  return PRIVATE_SERVICE_NAME.test(unwrapped);
}

export function normalizedIssuerOrigin(raw: string): string {
  let value: URL;
  try {
    value = new URL(raw);
  } catch {
    throw new Error('native Documents token issuer is invalid');
  }
  const loopback = isLoopbackHostname(value.hostname);
  if (
    (value.protocol !== 'https:' && !(loopback && value.protocol === 'http:')) ||
    value.username ||
    value.password ||
    value.pathname !== '/' ||
    value.search ||
    value.hash ||
    value.origin !== raw
  ) {
    throw new Error('native Documents token issuer is invalid');
  }
  return value.origin;
}

export function normalizedPrivateJwksUrl(raw: string): URL {
  let value: URL;
  try {
    value = new URL(raw);
  } catch {
    throw new Error('native Documents JWKS service URL is invalid');
  }
  if (
    !['http:', 'https:'].includes(value.protocol) ||
    !isPrivateServiceHostname(value.hostname) ||
    value.username ||
    value.password ||
    value.pathname !== JWKS_PATH ||
    value.search ||
    value.hash ||
    value.toString() !== raw
  ) {
    throw new Error('native Documents JWKS service URL is invalid');
  }
  return value;
}
