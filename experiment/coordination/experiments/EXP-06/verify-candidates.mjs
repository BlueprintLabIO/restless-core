import { writeFile } from "node:fs/promises";
import { chromium } from "file:///Users/yao/.nvm/versions/node/v24.16.0/lib/node_modules/playwright/index.mjs";

const candidates = {
  "candidate-a": "http://127.0.0.1:4401",
  "candidate-b": "http://127.0.0.1:4402",
};

const routes = [
  "/",
  "/product/",
  "/how-it-works/",
  "/research/",
  "/compare/",
  "/findings/",
  "/findings/four-departments-one-invalid-evaluator/",
  "/findings/supervision-needs-to-stay-available/",
  "/findings/teams-need-a-crossover/",
  "/findings/work-graph-is-a-record/",
];

const viewports = {
  desktop: { width: 1440, height: 1000 },
  mobile: { width: 390, height: 844 },
};

const browser = await chromium.launch({ headless: true });
const results = {};

for (const [candidate, baseUrl] of Object.entries(candidates)) {
  const context = await browser.newContext();
  const page = await context.newPage();
  const consoleErrors = [];
  const pageErrors = [];

  page.on("console", (message) => {
    if (message.type() === "error") consoleErrors.push(message.text());
  });
  page.on("pageerror", (error) => pageErrors.push(error.message));

  const routeChecks = [];
  for (const [viewport, size] of Object.entries(viewports)) {
    await page.setViewportSize(size);
    for (const route of routes) {
      const response = await page.goto(baseUrl + route, { waitUntil: "load" });
      await page.evaluate(async () => {
        await document.fonts.ready;
      });
      const metrics = await page.evaluate(() => ({
        title: document.title,
        h1: document.querySelector("h1")?.innerText ?? "",
        clientWidth: document.documentElement.clientWidth,
        scrollWidth: document.documentElement.scrollWidth,
        emDashCount: document.body.innerText.split(String.fromCharCode(8212)).length - 1,
      }));
      routeChecks.push({ viewport, route, status: response?.status() ?? null, ...metrics });
    }
  }

  await page.setViewportSize(viewports.desktop);
  await page.goto(baseUrl + "/", { waitUntil: "load" });
  await page.keyboard.press("Tab");
  const focus = await page.evaluate(() => {
    const element = document.activeElement;
    const style = element ? getComputedStyle(element) : null;
    return {
      tag: element?.tagName ?? "",
      text: element?.textContent?.trim() ?? "",
      outlineStyle: style?.outlineStyle ?? "",
      outlineWidth: style?.outlineWidth ?? "",
      top: element?.getBoundingClientRect().top ?? null,
    };
  });

  await page.goto(baseUrl + "/product/", { waitUntil: "load" });
  const productTabs = await page.getByRole("tab").count();
  let arrowKeyChangedTab = false;
  if (productTabs > 1) {
    const tabs = page.getByRole("tab");
    await tabs.first().focus();
    await page.keyboard.press("ArrowRight");
    arrowKeyChangedTab = (await tabs.nth(1).getAttribute("aria-selected")) === "true";
  }

  await page.setViewportSize(viewports.mobile);
  await page.goto(baseUrl + "/", { waitUntil: "load" });
  const menu = page.locator("header details").first();
  const menuPresent = (await menu.count()) === 1;
  if (menuPresent) await menu.locator("summary").click();
  const menuOpened = menuPresent && (await menu.getAttribute("open")) !== null;

  const reducedContext = await browser.newContext({
    viewport: viewports.desktop,
    reducedMotion: "reduce",
  });
  const reducedPage = await reducedContext.newPage();
  await reducedPage.goto(baseUrl + "/", { waitUntil: "load" });
  // Let the browser retire any 0.01 ms reduced-motion replacement frames.
  await reducedPage.waitForTimeout(250);
  const reducedMotion = await reducedPage.evaluate(() => ({
    preferenceMatches: matchMedia("(prefers-reduced-motion: reduce)").matches,
    runningAnimations: document
      .getAnimations()
      .filter((animation) => animation.playState === "running").length,
  }));
  await reducedContext.close();

  results[candidate] = {
    routeChecks,
    focus,
    productTabs,
    arrowKeyChangedTab,
    menuPresent,
    menuOpened,
    reducedMotion,
    consoleErrors,
    pageErrors,
  };

  await context.close();
}

await browser.close();

const outputUrl = new URL("./results/objective-checks.json", import.meta.url);
await writeFile(outputUrl, `${JSON.stringify(results, null, 2)}\n`);
console.log(JSON.stringify(results, null, 2));
