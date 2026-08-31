#!/usr/bin/env node

import { mkdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
import process from 'node:process';
import { chromium } from 'playwright';

const PROFILES = [
  { name: 'desktop', viewport: { width: 1440, height: 1000 }, reducedMotion: 'no-preference' },
  { name: 'mobile', viewport: { width: 390, height: 844 }, reducedMotion: 'no-preference' },
  { name: 'desktop-reduced-motion', viewport: { width: 1440, height: 1000 }, reducedMotion: 'reduce' },
];

function usage(message) {
  if (message) process.stderr.write(`${message}\n\n`);
  process.stderr.write(
    'Usage: restless-web-review --url <base-url> --output <directory> ' +
      '[--route </path>]... [--reference-url <url>] [--reference-route </path>]...\n',
  );
  process.exit(message ? 2 : 0);
}

function parseArgs(argv) {
  const args = { routes: [], referenceRoutes: [] };
  for (let index = 0; index < argv.length; index += 1) {
    const flag = argv[index];
    if (flag === '--help' || flag === '-h') usage();
    if (!['--url', '--output', '--route', '--reference-url', '--reference-route'].includes(flag)) {
      usage(`Unknown argument ${flag}`);
    }
    const value = argv[index + 1];
    if (!value || value.startsWith('--')) usage(`${flag} needs a value`);
    index += 1;
    if (flag === '--route') args.routes.push(value);
    else if (flag === '--reference-route') args.referenceRoutes.push(value);
    else if (flag === '--url') args.url = value;
    else if (flag === '--output') args.output = value;
    else args.referenceUrl = value;
  }
  if (!args.url || !args.output) usage('--url and --output are required');
  if (args.routes.length === 0) args.routes.push('/');
  args.url = new URL(args.url).toString();
  if (args.referenceUrl) args.referenceUrl = new URL(args.referenceUrl).toString();
  args.routes = [...new Set(args.routes.map((route) => new URL(route, args.url).pathname))];
  args.referenceRoutes = args.referenceUrl
    ? [
        ...new Set(
          (args.referenceRoutes.length ? args.referenceRoutes : ['/']).map(
            (route) => new URL(route, args.referenceUrl).pathname,
          ),
        ),
      ]
    : [];
  args.output = path.resolve(args.output);
  return args;
}

function fileSlug(route) {
  if (route === '/') return 'index';
  return route.replace(/^\/+|\/+$/g, '').replace(/[^a-zA-Z0-9._-]+/g, '-');
}

async function settle(page) {
  await page.evaluate(async () => {
    if (document.fonts?.ready) await document.fonts.ready;
  });
  await page.waitForTimeout(250);
  await page
    .waitForLoadState('networkidle', { timeout: 4_000 })
    .catch(() => undefined);
}

async function exerciseWholePage(page) {
  const height = await page.evaluate(() => document.documentElement.scrollHeight);
  const viewportHeight = page.viewportSize()?.height ?? 800;
  for (let top = 0; top < height; top += Math.max(320, Math.floor(viewportHeight * 0.72))) {
    await page.evaluate((nextTop) => window.scrollTo({ top: nextTop, behavior: 'instant' }), top);
    await page.waitForTimeout(140);
  }
  await page.evaluate(() => window.scrollTo({ top: 0, behavior: 'instant' }));
  await page.waitForTimeout(220);
}

async function inspectPage(page, response, consoleErrors, pageErrors) {
  return page.evaluate(
    ({ status, consoleErrors: capturedConsoleErrors, pageErrors: capturedPageErrors }) => {
      const selectorFor = (element) => {
        if (element.id) return `#${CSS.escape(element.id)}`;
        const parts = [];
        let current = element;
        while (current && current !== document.body && parts.length < 4) {
          let part = current.tagName.toLowerCase();
          const classes = [...current.classList].slice(0, 2);
          if (classes.length) part += `.${classes.map((value) => CSS.escape(value)).join('.')}`;
          parts.unshift(part);
          current = current.parentElement;
        }
        return parts.join(' > ');
      };
      const textFor = (element) => (element.textContent ?? '').replace(/\s+/g, ' ').trim().slice(0, 120);
      const visible = (element) => {
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return (
          style.display !== 'none' &&
          style.visibility !== 'hidden' &&
          Number.parseFloat(style.opacity || '1') > 0.02 &&
          rect.width > 0 &&
          rect.height > 0
        );
      };
      const authored = [...document.querySelectorAll('main h1, main h2, main h3, main p, main li, main article, main section, main [class*="reveal"]')];
      const invisibleAuthoredContent = authored
        .filter((element) => textFor(element) && !visible(element))
        .slice(0, 80)
        .map((element) => ({ selector: selectorFor(element), text: textFor(element) }));
      const interactive = [...document.querySelectorAll('a[href], button, input, select, textarea, [tabindex]:not([tabindex="-1"])')];
      const offViewportInteractive = interactive
        .filter(visible)
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          return rect.left < -1 || rect.right > document.documentElement.clientWidth + 1;
        })
        .slice(0, 80)
        .map((element) => {
          const rect = element.getBoundingClientRect();
          return {
            selector: selectorFor(element),
            text: textFor(element),
            left: Math.round(rect.left),
            right: Math.round(rect.right),
          };
        });
      const overflowElements = [...document.querySelectorAll('body *')]
        .filter(visible)
        .filter((element) => {
          const rect = element.getBoundingClientRect();
          return rect.left < -2 || rect.right > document.documentElement.clientWidth + 2;
        })
        .slice(0, 80)
        .map((element) => {
          const rect = element.getBoundingClientRect();
          return {
            selector: selectorFor(element),
            text: textFor(element),
            left: Math.round(rect.left),
            right: Math.round(rect.right),
          };
        });
      const links = [...document.querySelectorAll('a[href]')].map((anchor) => ({
        text: textFor(anchor),
        href: anchor.href,
        visible: visible(anchor),
      }));
      const headings = [...document.querySelectorAll('h1, h2, h3')]
        .filter(visible)
        .map((heading) => ({ level: heading.tagName.toLowerCase(), text: textFor(heading) }));
      const footer = document.querySelector('footer');
      const visibleElements = [...document.querySelectorAll('body *')].filter(visible);
      const textElements = visibleElements.filter((element) => textFor(element));
      const frequency = (values, limit = 16) =>
        [...values.reduce((counts, value) => counts.set(value, (counts.get(value) ?? 0) + 1), new Map())]
          .sort((left, right) => right[1] - left[1] || left[0].localeCompare(right[0]))
          .slice(0, limit)
          .map(([value, count]) => ({ value, count }));
      const typography = frequency(
        textElements.map((element) => {
          const style = getComputedStyle(element);
          return [
            style.fontFamily,
            style.fontSize,
            style.fontWeight,
            style.lineHeight,
            style.letterSpacing,
          ].join(' | ');
        }),
      );
      const palette = {
        text: frequency(textElements.map((element) => getComputedStyle(element).color)),
        background: frequency(
          visibleElements
            .map((element) => getComputedStyle(element).backgroundColor)
            .filter((value) => value !== 'rgba(0, 0, 0, 0)'),
        ),
        border: frequency(
          visibleElements
            .map((element) => getComputedStyle(element).borderTopColor)
            .filter((value) => value !== 'rgba(0, 0, 0, 0)'),
        ),
      };
      const geometry = {
        borderRadii: frequency(
          visibleElements
            .map((element) => getComputedStyle(element).borderRadius)
            .filter((value) => value !== '0px'),
        ),
        backdropFilteredElements: visibleElements.filter((element) => {
          const style = getComputedStyle(element);
          return style.backdropFilter !== 'none' || style.webkitBackdropFilter !== 'none';
        }).length,
      };
      const motion = {
        animations: frequency(
          visibleElements
            .flatMap((element) =>
              [null, '::before', '::after'].map((pseudo) => {
                const style = getComputedStyle(element, pseudo);
                return style.animationName === 'none'
                  ? null
                  : `${pseudo ?? 'element'} | ${style.animationName} | ${style.animationDuration} | ${style.animationIterationCount}`;
              }),
            )
            .filter(Boolean),
        ),
        transitions: frequency(
          visibleElements
            .map((element) => getComputedStyle(element).transitionDuration)
            .filter((value) => value !== '0s'),
        ),
      };
      const readingSurfaces = [
        ...document.querySelectorAll(
          '[data-reading-surface], .article-body, .prose, article [class*="body"]',
        ),
      ]
        .filter(visible)
        .slice(0, 20)
        .map((element) => {
          const style = getComputedStyle(element);
          const rect = element.getBoundingClientRect();
          return {
            selector: selectorFor(element),
            width: Math.round(rect.width),
            maxWidth: style.maxWidth,
            marginLeft: style.marginLeft,
            fontFamily: style.fontFamily,
            fontSize: style.fontSize,
            lineHeight: style.lineHeight,
            textCharacters: (element.textContent ?? '').replace(/\s+/g, ' ').trim().length,
          };
        });
      const graphics = {
        svg: visibleElements.filter((element) => element.tagName === 'svg').length,
        canvas: visibleElements.filter((element) => element.tagName === 'CANVAS').length,
        images: visibleElements.filter((element) => element.tagName === 'IMG').length,
        video: visibleElements.filter((element) => element.tagName === 'VIDEO').length,
        authoredVisuals: visibleElements.filter((element) => element.hasAttribute('data-visual')).length,
      };
      return {
        status,
        title: document.title,
        url: location.href,
        viewport: {
          width: document.documentElement.clientWidth,
          height: window.innerHeight,
        },
        document: {
          width: document.documentElement.scrollWidth,
          height: document.documentElement.scrollHeight,
          bodyTextCharacters: (document.body.innerText ?? '').trim().length,
        },
        horizontalOverflow:
          document.documentElement.scrollWidth > document.documentElement.clientWidth + 1,
        overflowElements,
        offViewportInteractive,
        invisibleAuthoredContent,
        headings,
        links,
        footer: footer
          ? { present: true, visible: visible(footer), textCharacters: (footer.innerText ?? '').trim().length }
          : { present: false, visible: false, textCharacters: 0 },
        design: {
          root: {
            fontFamily: getComputedStyle(document.body).fontFamily,
            fontSize: getComputedStyle(document.body).fontSize,
            lineHeight: getComputedStyle(document.body).lineHeight,
            color: getComputedStyle(document.body).color,
            backgroundColor: getComputedStyle(document.body).backgroundColor,
          },
          typography,
          palette,
          geometry,
          motion,
          readingSurfaces,
          graphics,
        },
        consoleErrors: capturedConsoleErrors,
        pageErrors: capturedPageErrors,
      };
    },
    {
      status: response?.status() ?? null,
      consoleErrors,
      pageErrors,
    },
  );
}

async function probeInternalLinks(context, pages, candidateOrigin) {
  const urls = [
    ...new Set(
      pages
        .flatMap((page) => page.observation.links)
        .map((link) => link.href)
        .filter((href) => {
          const url = new URL(href);
          return url.origin === candidateOrigin && ['http:', 'https:'].includes(url.protocol);
        })
        .map((href) => {
          const url = new URL(href);
          url.hash = '';
          return url.toString();
        }),
    ),
  ];
  const results = [];
  for (const url of urls) {
    try {
      const response = await context.request.get(url, { timeout: 10_000, failOnStatusCode: false });
      results.push({ url, status: response.status(), ok: response.ok() });
    } catch (error) {
      results.push({ url, status: null, ok: false, error: String(error) });
    }
  }
  return results;
}

async function captureTarget(browser, target, output, routes, profiles) {
  const pages = [];
  for (const profile of profiles) {
    const context = await browser.newContext({
      viewport: profile.viewport,
      reducedMotion: profile.reducedMotion,
      colorScheme: 'dark',
      deviceScaleFactor: 1,
    });
    for (const route of routes) {
      const page = await context.newPage();
      const consoleErrors = [];
      const pageErrors = [];
      page.on('console', (message) => {
        if (message.type() === 'error') consoleErrors.push(message.text());
      });
      page.on('pageerror', (error) => pageErrors.push(String(error)));
      const url = new URL(route, target.url).toString();
      let response = null;
      let navigationError = null;
      try {
        response = await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 30_000 });
        await settle(page);
        await exerciseWholePage(page);
      } catch (error) {
        navigationError = String(error);
      }
      const screenshot = `${target.name}-${profile.name}-${fileSlug(route)}.png`;
      await page.screenshot({ path: path.join(output, screenshot), fullPage: true });
      const observation = await inspectPage(page, response, consoleErrors, pageErrors);
      pages.push({
        target: target.name,
        profile: profile.name,
        route,
        screenshot,
        navigationError,
        observation,
      });
      await page.close();
    }
    await context.close();
  }
  return pages;
}

const args = parseArgs(process.argv.slice(2));
await mkdir(args.output, { recursive: true });
const browser = await chromium.launch({ executablePath: '/usr/bin/chromium', headless: true });

try {
  const candidate = { name: 'candidate', url: args.url };
  const candidatePages = await captureTarget(browser, candidate, args.output, args.routes, PROFILES);
  const linkContext = await browser.newContext();
  const internalLinks = await probeInternalLinks(
    linkContext,
    candidatePages.filter((page) => page.profile === 'desktop'),
    new URL(args.url).origin,
  );
  await linkContext.close();

  let referencePages = [];
  if (args.referenceUrl) {
    referencePages = await captureTarget(
      browser,
      { name: 'reference', url: args.referenceUrl },
      args.output,
      args.referenceRoutes,
      PROFILES.filter((profile) => profile.name !== 'desktop-reduced-motion'),
    );
  }

  const manifest = {
    schema: 'restless.web-review/v2',
    generatedAt: new Date().toISOString(),
    candidate: { baseUrl: args.url, routes: args.routes },
    reference: args.referenceUrl
      ? { baseUrl: args.referenceUrl, routes: args.referenceRoutes }
      : null,
    method: {
      browser: 'system Chromium through Playwright',
      scrollExercise: true,
      fontsAwaited: true,
      computedDesignEvidence: true,
      acceptanceVerdict: 'none; deterministic observations only',
      profiles: PROFILES,
    },
    pages: [...candidatePages, ...referencePages],
    internalLinks,
  };
  await writeFile(path.join(args.output, 'manifest.json'), `${JSON.stringify(manifest, null, 2)}\n`);

  const failedPages = candidatePages.filter(
    (page) =>
      page.navigationError ||
      page.observation.status === null ||
      page.observation.status >= 400,
  );
  const failedLinks = internalLinks.filter((link) => !link.ok);
  process.stdout.write(
    `${JSON.stringify({
      output: args.output,
      candidatePages: candidatePages.length,
      referencePages: referencePages.length,
      failedPages: failedPages.length,
      failedInternalLinks: failedLinks.length,
      manifest: path.join(args.output, 'manifest.json'),
    })}\n`,
  );
  if (failedPages.length || failedLinks.length) process.exitCode = 1;
} finally {
  await browser.close();
}
