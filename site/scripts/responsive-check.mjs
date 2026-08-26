import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { createServer } from 'node:http';
import { extname, join, relative, sep } from 'node:path';
import process from 'node:process';
import puppeteer from 'puppeteer-core';

const siteRoot = new URL('..', import.meta.url).pathname;
const distRoot = join(siteRoot, 'dist');
const requestedBaseUrl = process.env.RESPONSIVE_CHECK_BASE_URL;
const baseUrl = requestedBaseUrl ?? 'http://127.0.0.1:4322';
const viewports = [
  { width: 390, height: 844 },
  { width: 1440, height: 1000 },
];

function publicRoutes(directory = distRoot) {
  const routes = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) routes.push(...publicRoutes(path));
    if (!entry.isFile() || entry.name !== 'index.html') continue;
    const directoryName = relative(distRoot, directory).split(sep).join('/');
    routes.push(directoryName ? `/${directoryName}/` : '/');
  }
  return routes.filter((route) => route !== '/404/').sort();
}

function startStaticServer() {
  const contentTypes = {
    '.css': 'text/css; charset=utf-8',
    '.html': 'text/html; charset=utf-8',
    '.js': 'text/javascript; charset=utf-8',
    '.svg': 'image/svg+xml',
    '.woff2': 'font/woff2',
  };
  const server = createServer((request, response) => {
    const pathname = decodeURIComponent(new URL(request.url ?? '/', baseUrl).pathname);
    const requestedPath = pathname.endsWith('/') ? `${pathname}index.html` : pathname;
    const filePath = join(distRoot, requestedPath);
    if (!filePath.startsWith(distRoot) || !existsSync(filePath)) {
      response.writeHead(404).end('Not found');
      return;
    }
    response.writeHead(200, { 'Content-Type': contentTypes[extname(filePath)] ?? 'application/octet-stream' });
    response.end(readFileSync(filePath));
  });
  return new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(4322, '127.0.0.1', () => resolve(server));
  });
}

async function probeTarget(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`responsive target returned ${response.status} at ${url}`);
}

function chromiumExecutable() {
  const candidates = [
    process.env.CHROMIUM_PATH,
    '/usr/bin/chromium',
    '/usr/bin/chromium-browser',
    '/usr/bin/google-chrome',
  ].filter(Boolean);
  const executable = candidates.find(existsSync);
  if (!executable) throw new Error('Chromium not found; set CHROMIUM_PATH');
  return executable;
}

let staticServer;
let browser;
let failed = false;

try {
  if (!requestedBaseUrl) staticServer = await startStaticServer();
  await probeTarget(baseUrl);

  browser = await puppeteer.launch({
    executablePath: chromiumExecutable(),
    headless: true,
    args: ['--no-sandbox', '--disable-dev-shm-usage'],
  });

  for (const viewport of viewports) {
    for (const route of publicRoutes()) {
      const page = await browser.newPage();
      await page.setViewport(viewport);
      let response;
      let error;
      try {
        response = await page.goto(new URL(route, baseUrl).href, { waitUntil: 'networkidle0' });
        await page.evaluate(() => document.fonts.ready);
      } catch (caught) {
        error = caught instanceof Error ? caught.message : String(caught);
      }

      const measurement = error
        ? { route, ...viewport, error }
        : await page.evaluate(({ route, width, height, status }) => {
            const root = document.documentElement;
            const overflowing = [...document.querySelectorAll('body *')]
              .filter((element) => {
                const rect = element.getBoundingClientRect();
                return rect.right > root.clientWidth + 0.5 || rect.left < -0.5;
              })
              .slice(0, 8)
              .map((element) => {
                const rect = element.getBoundingClientRect();
                return {
                  element: element.tagName.toLowerCase(),
                  className: typeof element.className === 'string' ? element.className : '',
                  left: Math.round(rect.left * 100) / 100,
                  right: Math.round(rect.right * 100) / 100,
                  text: (element.textContent ?? '').trim().replace(/\s+/g, ' ').slice(0, 80),
                };
              });
            return {
              route,
              width,
              height,
              status,
              clientWidth: root.clientWidth,
              scrollWidth: root.scrollWidth,
              overflowing,
            };
          }, { route, ...viewport, status: response?.status() ?? 0 });

      console.log(JSON.stringify(measurement));
      if (measurement.error || measurement.status >= 400 || measurement.status === 0 || measurement.scrollWidth > measurement.clientWidth) {
        failed = true;
      }
      await page.close();
    }
  }
} catch (error) {
  failed = true;
  console.error(error instanceof Error ? error.stack : error);
} finally {
  await browser?.close();
  await new Promise((resolve) => staticServer ? staticServer.close(resolve) : resolve());
}

if (failed) process.exitCode = 1;
