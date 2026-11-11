import { mkdir, writeFile } from "node:fs/promises";
import { chromium } from "file:///Users/yao/.nvm/versions/node/v24.16.0/lib/node_modules/playwright/index.mjs";

const supplied = process.argv.slice(2).map((argument) => argument.split("=", 2));
if (supplied.length !== 2 || supplied.some(([label, url]) => !label || !url)) {
  throw new Error("usage: node verify-candidates.mjs candidate-a=http://... candidate-b=http://...");
}

const candidates = Object.fromEntries(supplied);
const fixedRoutes = [
  "/",
  "/product/",
  "/how-it-works/",
  "/research/",
  "/compare/",
  "/journal/",
];
const viewports = {
  desktop: { width: 1440, height: 1000 },
  mobile: { width: 390, height: 844 },
};

function normalizePath(pathname) {
  if (pathname === "/") return pathname;
  return pathname.endsWith("/") ? pathname : `${pathname}/`;
}

const browser = await chromium.launch({ headless: true });
const results = {};

for (const [candidate, untrimmedBaseUrl] of Object.entries(candidates)) {
  const baseUrl = untrimmedBaseUrl.replace(/\/$/, "");
  const context = await browser.newContext();
  const page = await context.newPage();
  const consoleErrors = [];
  const pageErrors = [];

  page.on("console", (message) => {
    if (message.type() === "error") {
      consoleErrors.push({ url: page.url(), message: message.text() });
    }
  });
  page.on("pageerror", (error) => pageErrors.push({ url: page.url(), message: error.message }));

  const journalResponse = await page.goto(`${baseUrl}/journal/`, { waitUntil: "networkidle" });
  if (journalResponse?.status() !== 200) {
    throw new Error(`${candidate} journal returned ${journalResponse?.status() ?? "no response"}`);
  }
  const findingRoutes = await page.evaluate(() =>
    [...document.querySelectorAll('a[href^="/journal/"]')]
      .map((anchor) => new URL(anchor.href).pathname)
      .filter((pathname) => pathname !== "/journal/"),
  );
  const uniqueFindingRoutes = [...new Set(findingRoutes.map(normalizePath))].sort();
  if (uniqueFindingRoutes.length < 3) {
    throw new Error(`${candidate} exposes only ${uniqueFindingRoutes.length} journal finding routes`);
  }
  const routes = [...fixedRoutes, ...uniqueFindingRoutes.slice(0, 3)];

  const routeChecks = [];
  for (const [viewport, size] of Object.entries(viewports)) {
    await page.setViewportSize(size);
    for (const route of routes) {
      const response = await page.goto(baseUrl + route, { waitUntil: "networkidle" });
      await page.evaluate(async () => document.fonts.ready);
      const metrics = await page.evaluate(() => ({
        title: document.title,
        h1: document.querySelector("h1")?.innerText.trim() ?? "",
        clientWidth: document.documentElement.clientWidth,
        scrollWidth: document.documentElement.scrollWidth,
        emDashCount: document.body.innerText.split(String.fromCharCode(8212)).length - 1,
        description: document.querySelector('meta[name="description"]')?.getAttribute("content") ?? "",
        ogTitle: document.querySelector('meta[property="og:title"]')?.getAttribute("content") ?? "",
      }));
      routeChecks.push({ viewport, route, status: response?.status() ?? null, ...metrics });
    }
  }

  await page.setViewportSize(viewports.desktop);
  await page.goto(`${baseUrl}/`, { waitUntil: "networkidle" });
  let focus = null;
  for (let index = 0; index < 6; index += 1) {
    await page.keyboard.press("Tab");
    focus = await page.evaluate(() => {
      const element = document.activeElement;
      const style = element ? getComputedStyle(element) : null;
      const rect = element?.getBoundingClientRect();
      return {
        tag: element?.tagName ?? "",
        text: element?.textContent?.trim().replace(/\s+/g, " ").slice(0, 120) ?? "",
        outlineStyle: style?.outlineStyle ?? "",
        outlineWidth: style?.outlineWidth ?? "",
        boxShadow: style?.boxShadow ?? "",
        visible: Boolean(rect && rect.width > 0 && rect.height > 0),
      };
    });
    if (focus.tag && focus.tag !== "BODY") break;
  }

  await page.setViewportSize(viewports.mobile);
  await page.goto(`${baseUrl}/`, { waitUntil: "networkidle" });
  const mobileNavigation = await page.evaluate(() => {
    const links = [...document.querySelectorAll("header a, nav a")].filter((element) => {
      const style = getComputedStyle(element);
      const rect = element.getBoundingClientRect();
      return style.visibility !== "hidden" && style.display !== "none" && rect.width > 0 && rect.height > 0;
    });
    const controls = [...document.querySelectorAll("header button, header summary, nav button, nav summary")]
      .filter((element) => {
        const style = getComputedStyle(element);
        const rect = element.getBoundingClientRect();
        return style.visibility !== "hidden" && style.display !== "none" && rect.width > 0 && rect.height > 0;
      });
    return {
      visibleLinkCount: links.length,
      visibleControlCount: controls.length,
      labels: [...links, ...controls].map((element) => element.textContent?.trim().replace(/\s+/g, " ").slice(0, 80)),
    };
  });

  const reducedContext = await browser.newContext({ viewport: viewports.desktop, reducedMotion: "reduce" });
  const reducedPage = await reducedContext.newPage();
  await reducedPage.goto(`${baseUrl}/`, { waitUntil: "networkidle" });
  await reducedPage.waitForTimeout(750);
  const reducedMotion = await reducedPage.evaluate(() => ({
    preferenceMatches: matchMedia("(prefers-reduced-motion: reduce)").matches,
    runningAnimations: document.getAnimations().filter((animation) => animation.playState === "running").length,
  }));
  await reducedContext.close();

  results[candidate] = {
    baseUrl,
    routes,
    routeChecks,
    focus,
    mobileNavigation,
    reducedMotion,
    consoleErrors,
    pageErrors,
  };
  await context.close();
}

await browser.close();
await mkdir(new URL("./results/", import.meta.url), { recursive: true });
await writeFile(
  new URL("./results/objective-checks.json", import.meta.url),
  `${JSON.stringify(results, null, 2)}\n`,
);
console.log(JSON.stringify(results, null, 2));
