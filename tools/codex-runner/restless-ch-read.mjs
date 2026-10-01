#!/usr/bin/env node

// Restless installs this fixed, read-only compatibility client for one Staff
// Attempt. The private adjacent config contains only an expiring Restless MCP
// grant; the Clapping Hands upstream token and browser profile stay on Core.
import { readFileSync } from 'node:fs';

function fail(kind) {
  process.stdout.write(JSON.stringify({ error: kind }) + '\n');
  process.exit(2);
}

function listingUrl(raw, site) {
  try {
    const url = new URL(raw);
    if (url.protocol !== 'https:' || url.username || url.password || url.port || url.search || url.hash) return false;
    if (site === 'facebook') {
      return url.hostname === 'www.facebook.com' && /^\/marketplace\/item\/[0-9]{1,24}\/?$/.test(url.pathname);
    }
    return url.hostname === 'www.gumtree.com.au' && /^\/web\/listing\/[a-z0-9-]{1,64}\/[0-9]{1,16}$/.test(url.pathname);
  } catch {
    return false;
  }
}

const [verb, ...args] = process.argv.slice(2);
let request;
if ((verb === 'search' || verb === 'search-fast') && (args.length === 1 || args.length === 2)) {
  const query = args[0]?.trim();
  const limit = args.length === 2 ? Number(args[1]) : 12;
  if (!query || query.length > 100 || /[\x00-\x1f\x7f]/.test(query) || !Number.isInteger(limit) || limit < 1 || limit > 24) fail('invalid-search');
  request = { operation: 'marketplace-search', query, limit };
  if (verb === 'search-fast') request.fastSearch = true;
} else if (verb === 'details' && args.length >= 1 && args.length <= 8) {
  if (args.some((url) => !listingUrl(url, 'facebook')) || new Set(args).size !== args.length) fail('invalid-marketplace-url');
  request = { operation: 'marketplace-details', urls: args };
} else if (verb === 'gumtree' && args.length === 1) {
  if (!listingUrl(args[0], 'gumtree')) fail('invalid-gumtree-url');
  request = { operation: 'gumtree-listing', url: args[0] };
} else {
  fail('usage: ch-read.mjs search QUERY [LIMIT] | search-fast QUERY [LIMIT] | details FB_ITEM_URL [FB_ITEM_URL...] | gumtree GUMTREE_LISTING_URL');
}

let config;
try {
  config = JSON.parse(readFileSync(new URL('./ch-read.json', import.meta.url), 'utf8'));
  const endpoint = new URL(config.endpoint);
  if (endpoint.protocol !== 'http:' || endpoint.hostname !== 'host.docker.internal' ||
      !/^\/mcp-read\/[a-z0-9_-]+\/clapping-hands$/.test(endpoint.pathname) ||
      !/^Bearer [A-Za-z0-9._~-]+$/.test(config.authorization)) fail('broker-config-invalid');
} catch {
  fail('broker-config-unavailable');
}

try {
  const startedAt = performance.now();
  const response = await fetch(config.endpoint, {
    method: 'POST',
    redirect: 'error',
    headers: { Authorization: config.authorization, 'Content-Type': 'application/json' },
    body: JSON.stringify(request),
    signal: AbortSignal.timeout(105_000),
  });
  if (!response.ok) fail(`broker-http-${response.status}`);
  const result = await response.json();
  process.stdout.write(JSON.stringify({
    isError: result.isError === true,
    brokerWallMs: Math.round(performance.now() - startedAt),
    result: result.structuredContent ?? result,
  }) + '\n');
  if (result.isError === true) process.exitCode = 2;
} catch {
  fail('broker-transport-failed');
}
