import http from 'node:http';
import { readFile, stat } from 'node:fs/promises';
import path from 'node:path';

const root = path.resolve(process.argv[2] || 'dist');
const port = Number(process.env.PORT || 8080);
const types = {'.html':'text/html; charset=utf-8','.css':'text/css; charset=utf-8','.js':'text/javascript; charset=utf-8','.svg':'image/svg+xml','.txt':'text/plain; charset=utf-8'};
const server = http.createServer(async (req,res) => {
  try {
    const pathname = decodeURIComponent(new URL(req.url, 'http://localhost').pathname);
    let target = path.join(root, pathname);
    if (!target.startsWith(root)) throw new Error('invalid path');
    const info = await stat(target).catch(() => null);
    if (info?.isDirectory() || pathname.endsWith('/')) target = path.join(target,'index.html');
    const data = await readFile(target);
    res.writeHead(200, {'content-type':types[path.extname(target)] || 'application/octet-stream','cache-control':'no-cache'});
    res.end(data);
  } catch {
    res.writeHead(404, {'content-type':'text/plain; charset=utf-8'}); res.end('Not found');
  }
});
server.listen(port, '0.0.0.0', () => console.log(`Review server listening on http://127.0.0.1:${port}/`));
