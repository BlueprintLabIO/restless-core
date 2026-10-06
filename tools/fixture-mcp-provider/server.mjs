// A real MCP provider for Restless's connections smoke (Sprint 61 T2).
//
// Streamable HTTP MCP behind real MCP authorization: protected-resource
// metadata, dynamic client registration, PKCE, exact redirect matching and
// refresh, all from the official SDK's auth router. Restless must connect it
// exactly as it would connect Linear, so nothing here is special to Restless.
//
// A separate loopback control port lets the runner change the world under
// the connection: mutate a tool's schema, expire or revoke tokens, drop one
// response after executing it, and read back what actually happened.
//
// Never a product dependency. Usage:
//   FIXTURE_CONTROL_TOKEN=… node server.mjs --port 9100 --control-port 9101 \
//     [--static-https-port 9102 --static-dir DIR --tls-cert C --tls-key K]
import { randomBytes, randomUUID } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { spawn } from 'node:child_process';
import { createServer as createHttpsServer } from 'node:https';
import express from 'express';
import { z } from 'zod';
import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StreamableHTTPServerTransport } from '@modelcontextprotocol/sdk/server/streamableHttp.js';
import { mcpAuthRouter, getOAuthProtectedResourceMetadataUrl } from '@modelcontextprotocol/sdk/server/auth/router.js';
import { requireBearerAuth } from '@modelcontextprotocol/sdk/server/auth/middleware/bearerAuth.js';
import { InvalidGrantError, InvalidTokenError } from '@modelcontextprotocol/sdk/server/auth/errors.js';

function option(name, fallback) {
  const at = process.argv.indexOf(`--${name}`);
  return at >= 0 ? process.argv[at + 1] : fallback;
}
const port = Number(option('port', '9100'));
const controlPort = Number(option('control-port', '9101'));
const controlToken = process.env.FIXTURE_CONTROL_TOKEN;
let accessSeconds = Number(option('access-seconds', '3600'));
if (!controlToken || controlToken.length < 16) throw Error('FIXTURE_CONTROL_TOKEN (16+ chars) is required');

const ACCOUNT = 'fixture-owner';
const base = new URL(`http://127.0.0.1:${port}`);
const mcpUrl = new URL('/mcp', base);

/** Everything the runner may inspect. Tokens are recorded so the runner can
 * grep a Runtime for their exact values. */
const world = {
  clients: new Map(),
  pending: new Map(), // consent id -> { client, params }
  codes: new Map(), // code -> { clientId, params }
  access: new Map(), // token -> { clientId, scopes, expiresAt }
  refresh: new Map(), // token -> { clientId, scopes }
  issued: { access: [], refresh: [] },
  revoked: [],
  notes: [
    { id: 'n1', to: 'ada@example.test', body: 'Kickoff notes' },
    { id: 'n2', to: 'lin@example.test', body: 'Pricing follow-up' },
  ],
  sends: [], // every send_note that executed, including dropped ones
  deletes: [],
  sendSchema: 1,
  dropNextSend: false,
};

function issueTokens(clientId, scopes) {
  const access = `fx_at_${randomBytes(24).toString('hex')}`;
  const refresh = `fx_rt_${randomBytes(24).toString('hex')}`;
  world.access.set(access, { clientId, scopes, expiresAt: Math.floor(Date.now() / 1000) + accessSeconds });
  world.refresh.set(refresh, { clientId, scopes });
  world.issued.access.push(access);
  world.issued.refresh.push(refresh);
  return { access_token: access, token_type: 'bearer', expires_in: accessSeconds, refresh_token: refresh, scope: scopes.join(' ') };
}

const provider = {
  clientsStore: {
    async getClient(id) { return world.clients.get(id); },
    async registerClient(client) {
      const full = { ...client, client_id: randomUUID(), client_id_issued_at: Math.floor(Date.now() / 1000) };
      world.clients.set(full.client_id, full);
      return full;
    },
  },
  // The SDK has already checked the client and that redirect_uri is one it
  // registered. Show one consent page; approving it issues a code.
  async authorize(client, params, res) {
    const id = randomUUID();
    world.pending.set(id, { client, params });
    res.type('html').send(`<!doctype html><html lang="en"><head><meta charset="utf-8"><title>Fixture sign-in</title></head>
<body><main><h1>Allow ${escapeHtml(client.client_name ?? 'this app')} to use Fixture Notes?</h1>
<p>Signed in as ${ACCOUNT}.</p>
<form method="post" action="/consent"><input type="hidden" name="id" value="${id}">
<button type="submit" name="decision" value="approve">Approve</button>
<button type="submit" name="decision" value="deny">Deny</button></form></main></body></html>`);
  },
  async challengeForAuthorizationCode(client, code) {
    const grant = world.codes.get(code);
    if (!grant || grant.clientId !== client.client_id) throw new InvalidGrantError('unknown authorization code');
    return grant.params.codeChallenge;
  },
  async exchangeAuthorizationCode(client, code, _verifier, redirectUri) {
    const grant = world.codes.get(code);
    if (!grant || grant.clientId !== client.client_id) throw new InvalidGrantError('unknown authorization code');
    if (redirectUri && redirectUri !== grant.params.redirectUri) throw new InvalidGrantError('redirect_uri does not match');
    world.codes.delete(code);
    return issueTokens(client.client_id, grant.params.scopes ?? []);
  },
  async exchangeRefreshToken(client, refreshToken) {
    const held = world.refresh.get(refreshToken);
    if (!held || held.clientId !== client.client_id) throw new InvalidGrantError('refresh token is not valid');
    world.refresh.delete(refreshToken);
    return issueTokens(client.client_id, held.scopes);
  },
  async verifyAccessToken(token) {
    const held = world.access.get(token);
    if (!held || held.expiresAt <= Math.floor(Date.now() / 1000)) throw new InvalidTokenError('access token is not valid');
    return { token, clientId: held.clientId, scopes: held.scopes, expiresAt: held.expiresAt };
  },
  async revokeToken(_client, request) {
    world.revoked.push(request.token);
    world.access.delete(request.token);
    world.refresh.delete(request.token);
  },
};

function escapeHtml(text) {
  return String(text).replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);
}

/** One MCP server per request, so a schema change is visible immediately. */
function notesServer(res) {
  const server = new McpServer({ name: 'fixture-notes', version: '1.0.0' });
  server.registerTool('whoami', {
    description: 'The signed-in Fixture Notes account.',
    inputSchema: {},
    annotations: { readOnlyHint: true },
  }, async () => ({ content: [{ type: 'text', text: JSON.stringify({ account: ACCOUNT }) }] }));
  server.registerTool('list_notes', {
    description: 'List notes in the account.',
    inputSchema: {},
    annotations: { readOnlyHint: true },
  }, async () => ({ content: [{ type: 'text', text: JSON.stringify({ notes: world.notes, sent: world.sends.map(({ id, to }) => ({ id, to })) }) }] }));
  const sendSchema = world.sendSchema === 1
    ? { to: z.string().describe('Recipient address'), body: z.string() }
    : { to: z.string().describe('Recipient address'), body: z.string(), subject: z.string().optional() };
  server.registerTool('send_note', {
    description: 'Send a note to someone. It reaches them.',
    inputSchema: sendSchema,
  }, async ({ to, body }) => {
    const id = `m_${randomBytes(6).toString('hex')}`;
    world.sends.push({ id, to, body, at: new Date().toISOString() });
    if (world.dropNextSend) {
      world.dropNextSend = false;
      // Executed upstream, but the caller never hears back.
      res.socket?.destroy();
      return await new Promise(() => {});
    }
    return { content: [{ type: 'text', text: JSON.stringify({ message_id: id, to }) }] };
  });
  server.registerTool('delete_note', {
    description: 'Permanently delete a note.',
    inputSchema: { id: z.string() },
    annotations: { destructiveHint: true },
  }, async ({ id }) => {
    world.deletes.push(id);
    world.notes = world.notes.filter((note) => note.id !== id);
    return { content: [{ type: 'text', text: JSON.stringify({ deleted: id }) }] };
  });
  return server;
}

const app = express();
app.use(mcpAuthRouter({
  provider,
  issuerUrl: base,
  resourceServerUrl: mcpUrl,
  scopesSupported: ['notes'],
  resourceName: 'Fixture Notes',
  authorizationOptions: { rateLimit: false },
  clientRegistrationOptions: { rateLimit: false },
  tokenOptions: { rateLimit: false },
  revocationOptions: { rateLimit: false },
}));
app.post('/consent', express.urlencoded({ extended: false }), (req, res) => {
  const pending = world.pending.get(req.body.id);
  if (!pending) return res.status(400).send('This sign-in has expired.');
  world.pending.delete(req.body.id);
  const target = new URL(pending.params.redirectUri);
  if (pending.params.state) target.searchParams.set('state', pending.params.state);
  if (req.body.decision !== 'approve') {
    target.searchParams.set('error', 'access_denied');
    return res.redirect(302, target.href);
  }
  const code = randomBytes(16).toString('hex');
  world.codes.set(code, { clientId: pending.client.client_id, params: pending.params });
  target.searchParams.set('code', code);
  res.redirect(302, target.href);
});
const bearer = requireBearerAuth({ verifier: provider, resourceMetadataUrl: getOAuthProtectedResourceMetadataUrl(mcpUrl) });
app.post('/mcp', bearer, express.json(), async (req, res) => {
  const server = notesServer(res);
  const transport = new StreamableHTTPServerTransport({ sessionIdGenerator: undefined, enableJsonResponse: true });
  res.on('close', () => { transport.close(); server.close(); });
  await server.connect(transport);
  await transport.handleRequest(req, res, req.body);
});
app.all('/mcp', bearer, (_req, res) => res.status(405).set('Allow', 'POST').end());
app.listen(port, '127.0.0.1', () => console.log(`fixture MCP on ${mcpUrl.href}`));

// Control: loopback only, and only with the runner's token.
const control = express();
control.use(express.json());
control.use((req, res, next) => (req.get('x-fixture-control') === controlToken ? next() : res.status(401).end()));
control.get('/state', (_req, res) => res.json({
  sends: world.sends,
  deletes: world.deletes,
  revoked: world.revoked,
  issued: world.issued,
  live_access: world.access.size,
  live_refresh: world.refresh.size,
  clients: world.clients.size,
  send_schema: world.sendSchema,
}));
control.post('/mutate-send-schema', (_req, res) => { world.sendSchema = world.sendSchema === 1 ? 2 : 1; res.json({ send_schema: world.sendSchema }); });
control.post('/expire-access', (_req, res) => {
  for (const held of world.access.values()) held.expiresAt = 0;
  res.json({ expired: world.access.size });
});
control.post('/revoke-refresh', (_req, res) => {
  const revoked = world.refresh.size + world.access.size;
  world.refresh.clear();
  world.access.clear();
  res.json({ revoked });
});
// Tokens issued from now on live this long, as a provider with short-lived
// access tokens would issue them.
control.post('/set-access-seconds', (req, res) => {
  accessSeconds = Number(req.body.seconds);
  res.json({ access_seconds: accessSeconds });
});
control.post('/drop-next-send', (_req, res) => { world.dropNextSend = true; res.json({ armed: true }); });
control.listen(controlPort, '127.0.0.1', () => console.log(`fixture control on 127.0.0.1:${controlPort}`));

// Optional https Git host for the plugin bundle: smart HTTP through
// `git http-backend`, so shallow clones work as they do against GitHub. The
// runner supplies the certificate and trusts it only in the processes under
// test.
const gitPort = option('static-https-port');
if (gitPort) {
  const projectRoot = option('static-dir');
  const gitServer = (req, res) => {
    const url = new URL(req.url, 'https://fixture');
    const backend = spawn('git', ['http-backend'], {
      env: {
        ...process.env,
        GIT_PROJECT_ROOT: projectRoot,
        GIT_HTTP_EXPORT_ALL: '1',
        PATH_INFO: decodeURIComponent(url.pathname),
        REQUEST_METHOD: req.method,
        QUERY_STRING: url.search.slice(1),
        CONTENT_TYPE: req.headers['content-type'] ?? '',
        REMOTE_ADDR: req.socket.remoteAddress ?? '',
        GIT_PROTOCOL: req.headers['git-protocol'] ?? '',
      },
    });
    req.pipe(backend.stdin);
    let head = Buffer.alloc(0);
    let headersDone = false;
    backend.stdout.on('data', (chunk) => {
      if (headersDone) return res.write(chunk);
      head = Buffer.concat([head, chunk]);
      const end = head.indexOf('\r\n\r\n');
      if (end < 0) return;
      headersDone = true;
      let status = 200;
      for (const line of head.subarray(0, end).toString().split('\r\n')) {
        const at = line.indexOf(':');
        const name = line.slice(0, at).trim();
        const value = line.slice(at + 1).trim();
        if (name.toLowerCase() === 'status') status = Number(value.split(' ')[0]);
        else res.setHeader(name, value);
      }
      res.statusCode = status;
      res.write(head.subarray(end + 4));
    });
    backend.on('close', () => res.end());
  };
  const tls = { cert: readFileSync(option('tls-cert')), key: readFileSync(option('tls-key')) };
  // Loopback for the plane; the default Docker bridge for the company computer.
  for (const host of ['127.0.0.1', '172.17.0.1']) {
    createHttpsServer(tls, gitServer)
      .on('error', (error) => console.log(`fixture git https on ${host} unavailable: ${error.code}`))
      .listen(Number(gitPort), host, () => console.log(`fixture git https on ${host}:${gitPort}`));
  }
}
