// Supervisor sends TERM to this process. Chromium's TERM path can exit before
// newly accepted cookies reach disk; Browser.close uses its normal close path.
// Core owns this browser lifecycle in both local and hosted installations.
import { spawn } from 'node:child_process';
import { pathToFileURL } from 'node:url';

export async function closeBrowser({ fetchImpl = fetch, Socket = WebSocket, timeoutMs = 3000 } = {}) {
  const response = await fetchImpl('http://127.0.0.1:9222/json/version', {
    signal: AbortSignal.timeout(timeoutMs), redirect: 'error',
  });
  if (!response.ok) throw new Error('browser discovery unavailable');
  const url = new URL((await response.json()).webSocketDebuggerUrl);
  if (url.protocol !== 'ws:' || url.hostname !== '127.0.0.1' || url.port !== '9222'
      || url.username || url.password || !url.pathname.startsWith('/devtools/browser/')) {
    throw new Error('browser discovery returned an unexpected endpoint');
  }
  await new Promise((resolve, reject) => {
    const socket = new Socket(url.href);
    let sent = false, settled = false;
    const finish = error => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      socket.close();
      error ? reject(error) : resolve();
    };
    const timer = setTimeout(() => finish(new Error('browser close request timed out')), timeoutMs);
    socket.addEventListener('open', () => {
      socket.send(JSON.stringify({ id: 1, method: 'Browser.close' }));
      sent = true;
    }, { once: true });
    socket.addEventListener('message', event => {
      try {
        const message = JSON.parse(event.data);
        if (message.id === 1) finish(message.error ? new Error('browser refused close') : undefined);
      } catch { finish(new Error('invalid browser close response')); }
    });
    // Chrome can terminate the transport without a WebSocket close frame after
    // accepting Browser.close. The caller must still observe the child exiting.
    const disconnected = () => finish(sent ? undefined : new Error('browser disconnected before close request'));
    socket.addEventListener('close', disconnected, { once: true });
    socket.addEventListener('error', disconnected, { once: true });
  });
}

export function runBrowser(command, args, {
  spawnImpl = spawn, close = closeBrowser, signals = process,
  termAfterMs = 8000, killAfterMs = 15000, log = message => console.error(message),
} = {}) {
  if (!command) throw new Error('browser command required');
  const child = spawnImpl(command, args, { stdio: 'inherit' });
  let stopping = false, timers = [];
  const stop = () => {
    if (stopping) return;
    stopping = true;
    timers = [
      setTimeout(() => { log('browser close deadline: sending TERM'); child.kill('SIGTERM'); }, termAfterMs),
      setTimeout(() => { log('browser stop deadline: sending KILL'); child.kill('SIGKILL'); }, killAfterMs),
    ];
    Promise.resolve().then(close).catch(() => log('browser close request unavailable; waiting for bounded fallback'));
  };
  signals.on('SIGTERM', stop);
  signals.on('SIGINT', stop);
  const cleanup = () => {
    timers.forEach(clearTimeout);
    signals.removeListener('SIGTERM', stop);
    signals.removeListener('SIGINT', stop);
  };
  return new Promise((resolve, reject) => {
    child.once('error', error => { cleanup(); reject(error); });
    child.once('exit', (code, signal) => { cleanup(); resolve({ code, signal, stopping }); });
  });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const result = await runBrowser(process.argv[2], process.argv.slice(3));
    process.exitCode = result.code ?? 1;
  } catch {
    console.error('browser launcher failed');
    process.exitCode = 1;
  }
}
