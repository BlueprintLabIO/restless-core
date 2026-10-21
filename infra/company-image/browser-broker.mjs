// Chrome's DevTools protocol is already the mature automation surface. This
// broker only gates and forwards it; it does not add browser actions. The
// /json discovery response is rewritten so clients keep their long-lived
// WebSocket through this gate instead of following Chrome's private 9222 URL.
import fs from 'node:fs';
import http from 'node:http';
import { createRequire } from 'node:module';
import { execFileSync } from 'node:child_process';

const { WebSocket, WebSocketServer } = createRequire(import.meta.url)('ws');

const leasePath = '/company/run/browser-control.json';
const listenPort = 9223;
const chromePort = 9222;
const active = new Set();
const websocketServer = new WebSocketServer({ noServer: true, perMessageDeflate: false });
const tabsPath = '/company/browser-profile/restless-tabs.json';
const sessionPath = '/company/run/browser-agent-session.json';
const desktopEnv = { ...process.env, DISPLAY: ':1' };

// The browser sleeps on its own (S104 F6). Chrome is most of an awake company's memory, and on a
// 4 GB plane four awake companies thrashed it (22 October). After this long with no agent socket,
// no request, no owner control and nobody watching the desktop, Chrome stops; any agent request or
// desktop viewer starts it again, and start-company-chromium reopens its tabs. A health probe
// (x-restless-probe) is answered without waking it.
const idleMs = Number(process.env.RESTLESS_BROWSER_IDLE_MS ?? 15 * 60 * 1000);
const asleepPath = '/company/run/browser-asleep';
const supervisor = ['-c', '/etc/supervisor/conf.d/company.conf'];
let lastUse = Date.now();
let waking = null;
// Supervisor starts Chrome with the container, so a marker left from before is stale.
try { fs.rmSync(asleepPath, { force: true }); } catch { /* nothing to clear */ }

function supervisorctl(...args) {
  return execFileSync('/usr/bin/supervisorctl', [...supervisor, ...args], { encoding: 'utf8', timeout: 40000 });
}

function chromeAsleep() {
  return fs.existsSync(asleepPath);
}

// An established connection to a local port: the desktop's web transport (websockify, 6080) means
// someone is watching the computer; Chrome's own DevTools port (9222) means Core or an app is
// driving it directly.
function connected(port) {
  const hex = `:${port.toString(16).toUpperCase().padStart(4, '0')}`;
  for (const table of ['/proc/net/tcp', '/proc/net/tcp6']) {
    let rows = '';
    try { rows = fs.readFileSync(table, 'utf8'); } catch { continue; }
    for (const row of rows.split('\n').slice(1)) {
      const fields = row.trim().split(/\s+/);
      if (fields.length > 3 && fields[1].endsWith(hex) && fields[3] === '01') return true;
    }
  }
  return false;
}
const desktopWatched = () => connected(6080);

// One request on its own connection, closed after: a pooled keep-alive socket of the broker's own
// would count as Chrome being in use and keep it awake for ever.
function chromeGet(path, timeout = 1500) {
  return new Promise((resolve) => {
    const request = http.get(
      { host: '127.0.0.1', port: chromePort, path, agent: false, headers: { connection: 'close' }, timeout },
      (response) => {
        const chunks = [];
        response.on('data', (chunk) => chunks.push(chunk));
        response.on('end', () => resolve({ ok: response.statusCode === 200, body: Buffer.concat(chunks).toString('utf8') }));
      }
    );
    request.on('timeout', () => request.destroy());
    request.on('error', () => resolve({ ok: false, body: '' }));
  });
}

async function chromeAnswers() {
  return (await chromeGet('/json/version')).ok;
}

function wakeChrome() {
  lastUse = Date.now();
  if (!chromeAsleep()) return Promise.resolve();
  waking ??= (async () => {
    try {
      supervisorctl('start', 'chromium');
    } catch { /* already starting, or supervisor answers late: the probe below decides */ }
    for (let i = 0; i < 60 && !(await chromeAnswers()); i += 1) await new Promise((resolve) => setTimeout(resolve, 500));
    try { fs.rmSync(asleepPath, { force: true }); } catch { /* the next wake retries */ }
    console.log(JSON.stringify({ event: 'browser_woke' }));
  })().finally(() => { waking = null; });
  return waking;
}

setInterval(() => {
  if (idleMs <= 0) return;
  if (desktopWatched()) {
    void wakeChrome();
    return;
  }
  if (active.size > 0 || ownerControls() || chromeAsleep() || waking || connected(chromePort)) {
    if (active.size > 0 || ownerControls() || connected(chromePort)) lastUse = Date.now();
    return;
  }
  if (Date.now() - lastUse < idleMs) return;
  try {
    fs.writeFileSync(asleepPath, `${new Date().toISOString()}\n`, { mode: 0o600 });
    supervisorctl('stop', 'chromium');
    console.log(JSON.stringify({ event: 'browser_slept', idle_ms: Date.now() - lastUse }));
  } catch (error) {
    try { fs.rmSync(asleepPath, { force: true }); } catch { /* left for the next look */ }
    console.log(JSON.stringify({ event: 'browser_sleep_failed', message: String(error?.message ?? error) }));
  }
}, 5000);

function ownerControls() {
  try {
    const lease = JSON.parse(fs.readFileSync(leasePath, 'utf8'));
    return lease.controller === 'owner' && Date.parse(lease.expires_at) > Date.now();
  } catch {
    return false;
  }
}

function activeAgentSession(ticket) {
  try {
    const session = JSON.parse(fs.readFileSync(sessionPath, 'utf8'));
    return session.ticket === ticket && Date.parse(session.expires_at) > Date.now()
      ? session
      : null;
  } catch {
    return null;
  }
}

function refuse(response) {
  response.writeHead(423, { 'content-type': 'application/json', connection: 'close' });
  response.end(
    JSON.stringify({
      error: 'owner_controls',
      message: 'browser automation is paused for owner handover'
    })
  );
}

function listWindows() {
  const activeOutput = execFileSync('/usr/bin/xprop', ['-root', '_NET_ACTIVE_WINDOW'], {
    encoding: 'utf8', timeout: 1000, env: desktopEnv
  });
  const activeMatch = activeOutput.match(/# (0x[0-9a-f]+)/i)?.[1];
  const active = activeMatch ? BigInt(activeMatch).toString(16) : null;
  const rows = execFileSync('/usr/bin/wmctrl', ['-lpGx'], {
    encoding: 'utf8', timeout: 1000, env: desktopEnv
  }).split('\n');
  const windows = [];
  for (const row of rows) {
    const match = row.match(/^\s*(0x[0-9a-f]+)\s+\S+\s+\S+\s+(-?\d+)\s+(-?\d+)\s+(\d+)\s+(\d+)\s+(\S+)\s+\S+\s*(.*)$/i);
    if (!match) continue;
    const [, id, x, y, width, height, app, title] = match;
    let type = '';
    try {
      type = execFileSync('/usr/bin/xprop', ['-id', id, '_NET_WM_WINDOW_TYPE'], {
        encoding: 'utf8', timeout: 1000, env: desktopEnv
      });
    } catch {
      continue;
    }
    if (type.includes('_NET_WM_WINDOW_TYPE_DOCK') || type.includes('_NET_WM_WINDOW_TYPE_DESKTOP')) continue;
    windows.push({
      id: id.toLowerCase(),
      title: title.trim(),
      app,
      geometry: { x: Number(x), y: Number(y), width: Number(width), height: Number(height) },
      active: BigInt(id).toString(16) === active
    });
  }
  return windows;
}

function sendJson(response, status, value) {
  response.writeHead(status, { 'content-type': 'application/json' });
  response.end(JSON.stringify(value));
}

function desktopRequest(request, response) {
  const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
  if (pathname === '/restless/desktop/windows' && request.method === 'GET') {
    try {
      sendJson(response, 200, { windows: listWindows() });
    } catch (error) {
      sendJson(response, 503, { error: 'desktop_unavailable', message: error.message });
    }
    return true;
  }
  const match = pathname.match(/^\/restless\/desktop\/windows\/(0x[0-9a-f]+)\/focus$/i);
  if (match && request.method === 'POST') {
    if (!ownerControls()) {
      sendJson(response, 423, { error: 'owner_lease_required', message: 'take control of the company computer before focusing a window' });
      return true;
    }
    let body = '';
    request.setEncoding('utf8');
    request.on('data', (chunk) => {
      body += chunk;
      if (body.length > 1024) request.destroy();
    });
    request.on('end', () => {
      try {
        const input = JSON.parse(body);
        const lease = JSON.parse(fs.readFileSync(leasePath, 'utf8'));
        if (typeof input.client_id !== 'string' || typeof input.lease_id !== 'string' ||
            input.client_id.length > 128 || input.lease_id.length > 128 ||
            lease.controller !== 'owner' || !(Date.parse(lease.expires_at) > Date.now()) ||
            input.client_id !== lease.client_id || input.lease_id !== lease.lease_id) {
          sendJson(response, 423, { error: 'owner_lease_required', message: 'this tab does not hold the live company computer lease' });
          return;
        }
        const id = match[1].toLowerCase();
        if (!listWindows().some((window) => window.id === id)) {
          sendJson(response, 404, { error: 'window_unavailable', message: 'the selected window is no longer open' });
          return;
        }
        const latest = JSON.parse(fs.readFileSync(leasePath, 'utf8'));
        if (latest.controller !== 'owner' || !(Date.parse(latest.expires_at) > Date.now()) ||
            latest.client_id !== input.client_id || latest.lease_id !== input.lease_id) {
          sendJson(response, 423, { error: 'owner_lease_required', message: 'the company computer lease expired before focusing the window' });
          return;
        }
        execFileSync('/usr/bin/wmctrl', ['-ia', id], {
          stdio: 'ignore', timeout: 1000, env: desktopEnv
        });
        sendJson(response, 200, { focused: true });
      } catch (error) {
        sendJson(response, 400, { error: 'invalid_focus_request', message: error.message });
      }
    });
    return true;
  }
  return false;
}

const server = http.createServer(async (request, response) => {
  if (desktopRequest(request, response)) return;
  if (ownerControls()) {
    refuse(response);
    return;
  }
  if (request.headers['x-restless-probe'] && chromeAsleep()) {
    sendJson(response, 200, { Browser: 'asleep', asleep: true });
    return;
  }
  if (!request.headers['x-restless-probe']) await wakeChrome();
  const upstream = http.request(
    {
      host: '127.0.0.1',
      port: chromePort,
      method: request.method,
      path: request.url,
      headers: { ...request.headers, host: `127.0.0.1:${chromePort}` }
    },
    (incoming) => {
      const chunks = [];
      incoming.on('data', (chunk) => chunks.push(chunk));
      incoming.on('end', () => {
        let body = Buffer.concat(chunks);
        const contentType = String(incoming.headers['content-type'] ?? '');
        if (contentType.includes('json')) {
          body = Buffer.from(
            body
              .toString('utf8')
              .replaceAll(`ws://127.0.0.1:${chromePort}`, `ws://127.0.0.1:${listenPort}`)
              .replaceAll(`ws://localhost:${chromePort}`, `ws://127.0.0.1:${listenPort}`)
          );
        }
        const headers = { ...incoming.headers, 'content-length': String(body.length) };
        response.writeHead(incoming.statusCode ?? 502, headers);
        response.end(body);
      });
    }
  );
  upstream.on('error', (error) => {
    if (!response.headersSent) response.writeHead(502, { 'content-type': 'application/json' });
    response.end(JSON.stringify({ error: 'chromium_unavailable', message: error.message }));
  });
  request.pipe(upstream);
});

server.on('upgrade', async (request, socket, head) => {
  const pathname = new URL(request.url, 'http://127.0.0.1').pathname;
  const match = pathname.match(/^\/session\/([a-f0-9]{32})(\/devtools\/browser\/[a-zA-Z0-9_-]+)$/);
  if (!match || !activeAgentSession(match[1])) {
    socket.end('HTTP/1.1 401 Unauthorized\r\nConnection: close\r\n\r\n');
    return;
  }
  if (ownerControls()) {
    socket.end('HTTP/1.1 423 Locked\r\nConnection: close\r\n\r\n');
    return;
  }
  await wakeChrome();

  websocketServer.handleUpgrade(request, socket, head, (agent) => {
    const upstream = new WebSocket(`ws://127.0.0.1:${chromePort}${match[2]}`, {
      perMessageDeflate: false,
      handshakeTimeout: 5000
    });
    const pair = {
      agent, upstream, ticket: match[1], pending: new Map(), ownerErrored: new Set(),
      openingQueue: [], ownerPaused: false, closed: false
    };
    active.add(pair);
    const close = () => {
      if (pair.closed) return;
      pair.closed = true;
      active.delete(pair);
      if (agent.readyState !== WebSocket.CLOSED) agent.close();
      if (upstream.readyState === WebSocket.OPEN || upstream.readyState === WebSocket.CONNECTING) upstream.close();
    };
    agent.on('message', (data, binary) => {
      if (pair.closed) return;
      lastUse = Date.now();
      if (ownerControls()) { rejectOwnerCommand(pair, data, binary); return; }
      if (!activeAgentSession(pair.ticket)) { close(); return; }
      if (upstream.readyState === WebSocket.CONNECTING) {
        if (pair.openingQueue.length >= 256) {
          rejectUnavailableCommand(pair, data, binary, 'browser connection is still starting; command was not forwarded');
          return;
        }
        pair.openingQueue.push({ data, binary });
        return;
      }
      forwardAgentCommand(pair, data, binary);
    });
    upstream.on('open', () => {
      const queued = pair.openingQueue.splice(0);
      if (pair.closed) return;
      if (ownerControls() || !activeAgentSession(pair.ticket)) {
        for (const item of queued) rejectUnavailableCommand(pair, item.data, item.binary,
          'owner_controls: browser connection changed before command forwarding');
        return;
      }
      for (const item of queued) forwardAgentCommand(pair, item.data, item.binary);
    });
    upstream.on('message', (data, binary) => {
      if (pair.closed || agent.readyState !== WebSocket.OPEN) return;
      if (!binary) {
        try {
          const message = JSON.parse(data.toString());
          if (Number.isInteger(message.id)) {
            if (pair.ownerErrored.delete(message.id)) return;
            if (ownerControls()) {
              const pending = pair.pending.get(message.id);
              if (pending) {
                pair.pending.delete(message.id);
                agent.send(JSON.stringify({
                  id: message.id,
                  ...(pending.sessionId ? { sessionId: pending.sessionId } : {}),
                  error: { code: -32000, message: 'owner_controls: the result may be uncertain; inspect current browser state before continuing' }
                }));
              }
              return;
            }
            pair.pending.delete(message.id);
          }
        } catch { /* forward non-JSON payloads unchanged */ }
      }
      agent.send(data, { binary });
    });
    agent.on('error', close);
    upstream.on('error', close);
    agent.on('close', close);
    upstream.on('close', close);
  });
});

function rejectOwnerCommand(pair, data, binary) {
  if (binary || pair.agent.readyState !== WebSocket.OPEN) return;
  let message;
  try { message = JSON.parse(data.toString()); } catch { return; }
  if (!Number.isInteger(message.id)) return;
  pair.agent.send(JSON.stringify({
    id: message.id,
    ...(message.sessionId ? { sessionId: message.sessionId } : {}),
    error: { code: -32000, message: 'owner_controls: inspect current browser state after control returns; this command was not forwarded' }
  }));
}

function rejectUnavailableCommand(pair, data, binary, reason) {
  if (binary || pair.agent.readyState !== WebSocket.OPEN) return;
  let message;
  try { message = JSON.parse(data.toString()); } catch { return; }
  if (!Number.isInteger(message.id)) return;
  pair.agent.send(JSON.stringify({
    id: message.id,
    ...(message.sessionId ? { sessionId: message.sessionId } : {}),
    error: { code: -32000, message: reason }
  }));
}

function forwardAgentCommand(pair, data, binary) {
  if (pair.closed || pair.upstream.readyState !== WebSocket.OPEN) {
    rejectUnavailableCommand(pair, data, binary, 'browser connection closed before command forwarding');
    return;
  }
  if (!binary) {
    try {
      const message = JSON.parse(data.toString());
      if (Number.isInteger(message.id)) {
        pair.pending.set(message.id, { method: message.method ?? '', sessionId: message.sessionId });
      }
    } catch { return; }
  }
  pair.upstream.send(data, { binary });
}

// Keep each authenticated CDP socket alive through control changes. On
// takeover, fail pending commands and stop forwarding new ones; never queue
// or replay browser actions after the owner returns.
setInterval(() => {
  // No live authenticated sockets can need a lease transition while idle.
  // New requests check both the session and lease before connecting.
  if (active.size === 0) return;
  for (const pair of [...active]) {
    if (!activeAgentSession(pair.ticket)) {
      pair.agent.close();
      pair.upstream.close();
      active.delete(pair);
      continue;
    }
    const controlled = ownerControls();
    if (controlled && !pair.ownerPaused) {
      pair.ownerPaused = true;
      for (const [id, pending] of pair.pending) {
        pair.ownerErrored.add(id);
        if (pair.agent.readyState === WebSocket.OPEN) {
          pair.agent.send(JSON.stringify({
            id,
            ...(pending.sessionId ? { sessionId: pending.sessionId } : {}),
            error: { code: -32000, message: 'owner_controls: the result may be uncertain; inspect current browser state before continuing' }
          }));
        }
      }
      pair.pending.clear();
    } else if (!controlled && pair.ownerPaused) {
      pair.ownerPaused = false;
    }
  }
}, 100);

async function checkpointTabs() {
  if (ownerControls() || chromeAsleep()) return;
  try {
    const listed = await chromeGet('/json/list', 3000);
    if (!listed.ok) return;
    const targets = JSON.parse(listed.body);
    const urls = [...new Set(
      targets
        .filter((target) => target.type === 'page')
        .map((target) => String(target.url ?? ''))
        .filter((url) => /^(https?|file):\/\//.test(url))
    )];
    // A just-started new-tab page must not erase the last useful checkpoint
    // before start-company-chromium has had a chance to reopen it.
    if (urls.length === 0) return;
    const snapshot = `${JSON.stringify(urls, null, 2)}\n`;
    try {
      if (fs.readFileSync(tabsPath, 'utf8') === snapshot) return;
    } catch {
      // A missing checkpoint needs recreating for browser restart recovery.
    }
    const temporary = `${tabsPath}.tmp`;
    fs.writeFileSync(temporary, snapshot, { mode: 0o600 });
    fs.renameSync(temporary, tabsPath);
  } catch {
    // Health reports Chrome separately; a transient checkpoint miss does not
    // turn a URL snapshot into authority or wedge the browser.
  }
}

setInterval(checkpointTabs, 2000);

server.listen(listenPort, '127.0.0.1');
