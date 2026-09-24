// Chrome's DevTools protocol is already the mature automation surface. This
// broker only gates and forwards it; it does not add browser actions. The
// /json discovery response is rewritten so clients keep their long-lived
// WebSocket through this gate instead of following Chrome's private 9222 URL.
import fs from 'node:fs';
import http from 'node:http';
import net from 'node:net';
import { execFileSync } from 'node:child_process';

const leasePath = '/company/run/browser-control.json';
const listenPort = 9223;
const chromePort = 9222;
const active = new Set();
const tabsPath = '/company/browser-profile/restless-tabs.json';
const desktopEnv = { ...process.env, DISPLAY: ':1' };

function ownerControls() {
  try {
    const lease = JSON.parse(fs.readFileSync(leasePath, 'utf8'));
    return lease.controller === 'owner' && Date.parse(lease.expires_at) > Date.now();
  } catch {
    return false;
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

const server = http.createServer((request, response) => {
  if (desktopRequest(request, response)) return;
  if (ownerControls()) {
    refuse(response);
    return;
  }
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

server.on('upgrade', (request, client, head) => {
  if (ownerControls()) {
    client.end('HTTP/1.1 423 Locked\r\nConnection: close\r\n\r\n');
    return;
  }
  const chrome = net.connect({ host: '127.0.0.1', port: chromePort }, () => {
    const headers = Object.entries(request.headers)
      .map(([name, value]) => `${name}: ${value}`)
      .join('\r\n');
    chrome.write(`${request.method} ${request.url} HTTP/${request.httpVersion}\r\n${headers}\r\n\r\n`);
    if (head.length) chrome.write(head);
    client.pipe(chrome);
    chrome.pipe(client);
  });
  const pair = { client, chrome };
  active.add(pair);
  const close = () => {
    active.delete(pair);
    client.destroy();
    chrome.destroy();
  };
  client.on('error', close);
  chrome.on('error', close);
  client.on('close', close);
  chrome.on('close', close);
});

// Existing automation WebSockets cannot race the owner's keyboard. Sever
// them at acquisition; clients see a transport loss and resume only after the
// source-owned hand-back wake.
setInterval(() => {
  // Handover only needs to sever live automation sockets. New requests check
  // the lease before connecting, so an idle broker need not read it at 10 Hz.
  if (active.size === 0) return;
  if (!ownerControls()) return;
  for (const pair of [...active]) {
    pair.client.destroy(new Error('owner took browser control'));
    pair.chrome.destroy();
    active.delete(pair);
  }
}, 100);

async function checkpointTabs() {
  if (ownerControls()) return;
  try {
    const response = await fetch(`http://127.0.0.1:${chromePort}/json/list`);
    if (!response.ok) return;
    const targets = await response.json();
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
