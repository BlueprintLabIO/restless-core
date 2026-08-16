// Chrome's DevTools protocol is already the mature automation surface. This
// broker only gates and forwards it; it does not add browser actions. The
// /json discovery response is rewritten so clients keep their long-lived
// WebSocket through this gate instead of following Chrome's private 9222 URL.
import fs from 'node:fs';
import http from 'node:http';
import net from 'node:net';

const leasePath = '/company/run/browser-control.json';
const listenPort = 9223;
const chromePort = 9222;
const active = new Set();
const tabsPath = '/company/browser-profile/restless-tabs.json';

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

const server = http.createServer((request, response) => {
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
    const temporary = `${tabsPath}.tmp`;
    fs.writeFileSync(temporary, `${JSON.stringify(urls, null, 2)}\n`, { mode: 0o600 });
    fs.renameSync(temporary, tabsPath);
  } catch {
    // Health reports Chrome separately; a transient checkpoint miss does not
    // turn a URL snapshot into authority or wedge the browser.
  }
}

setInterval(checkpointTabs, 2000);

server.listen(listenPort, '127.0.0.1');
