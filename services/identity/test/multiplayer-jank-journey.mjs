// Zero-jank gate (Sprint 109 C7): a headless pass over every primary surface and Inbox card
// that fails on any layout shift, any unstyled frame or any named edge off the grid.
//
// Started by run-core.mjs (RESTLESS_TEST_JOURNEY=multiplayer-jank-journey.mjs, or
// scripts/multiplayer-smoke --jank) on the shared multiplayer harness: a real isolated plane,
// account service and _test company, an owner and a member signed in over network entry.
//
// Matrix, per role: the six primary surfaces (Inbox, Work, Library, People, Apps, Company; what
// the role's navigation offers), at 1440px and 390px, in four states:
//   empty      the company as created, before anything is seeded
//   populated  after seeding documents, a sheet, goals, a team, a room with messages, direction
//              requests, a charter draft and a review (several Inbox card kinds)
//   slow       populated, with every company API read held 1.2 s (the loading state, then data)
//   failure    populated, with every company API read except the principal answered 503
// and three loads: cold (a fresh browser context: no HTTP cache, no stored queries), warm (the
// same page loaded again) and nav (arriving by the navigation tab from another surface). Every
// Inbox card the populated owner and member see is also opened, cold and slow, at both widths.
//
// What fails a visit:
//   layout-shift  a PerformanceObserver layout-shift entry with no input in the 500 ms before it
//                 (hadRecentInput, or a pointer/key event the page saw). Input-driven movement is
//                 recorded as intentional and never fails.
//   movement      a landmark (top bar, navigation, panes, headers, the first rows, the composer)
//                 that moved or resized after the page was revealed, with no input. Chrome scores
//                 an in-place swap as zero; this sees it.
//   unstyled      a frame with visible text before the design tokens applied, or while one of
//                 the page's stylesheets was still loading.
//   grid          a named shared edge off the grid (see GRID below).
// A failure listed in jank-allowlist.json (same visit, kind and element, value within its
// measured maximum) is reported as known; anything else fails the gate. RESTLESS_JANK_RECORD=1
// writes the observed failures in that file's shape to the evidence folder.
//
// Evidence (under the run's evidence folder, jank/): results.json with every visit, a screenshot
// per state, the loading frame of each slow visit, and for each failure its screenshot, geometry
// and a filmstrip of a replayed load (CDP screencast frames with their timestamps).
import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { mkdir, readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  base,
  boot,
  browser,
  company,
  core,
  coreApi,
  daemonCommand,
  direction,
  evidence,
  invite,
  note,
  pass,
  pause,
  person,
  run,
  until,
} from "./multiplayer-harness.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const out = join(evidence, "jank");
await mkdir(out, { recursive: true });
const allowlist = JSON.parse(await readFile(join(here, "jank-allowlist.json"), "utf8"));

const WIDTHS = { desktop: { width: 1440, height: 900 }, phone: { width: 390, height: 844 } };
const SLOW_MS = 1200;
const SETTLE_MS = 2500; // observed after the reveal; the slow state waits for its held reads too
const SLOW_SETTLE_MS = 6500;
// The Email card (S109 H2) needs a Resend key the daemon can resolve; nothing is ever sent.
process.env.RESTLESS_JANK_RESEND_KEY ??= "re_jank_never_sent";
const ONLY = (process.env.RESTLESS_JANK_ONLY ?? "").split(",").filter(Boolean);
const wanted = (id) => !ONLY.length || ONLY.some((part) => id.includes(part));

// ------------------------------------------------------------------ in-page probes
/* Installed before any of the page's own scripts: layout-shift entries, input times, and every
 * animation frame until the page is styled, checked for visible text without the styles. */
function probe() {
  const jank = (window.__jank = { shifts: [], inputs: [], unstyled: [], fontFrames: 0 });
  const describe = (node) => {
    let element = node && (node.nodeType === 1 ? node : node.parentElement);
    const parts = [];
    while (element && element !== document.body && parts.length < 4) {
      const classes = [...element.classList].filter((name) => !name.startsWith("svelte-")).slice(0, 3);
      parts.unshift(element.tagName.toLowerCase() + (element.id ? `#${element.id}` : "") + classes.map((name) => `.${name}`).join(""));
      element = element.parentElement;
    }
    return parts.join(" > ");
  };
  window.__jankDescribe = describe;
  const rect = (r) => (r ? { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) } : null);
  for (const type of ["pointerdown", "keydown", "wheel", "touchstart"])
    addEventListener(type, () => jank.inputs.push(performance.now()), { capture: true, passive: true });
  try {
    new PerformanceObserver((list) => {
      for (const entry of list.getEntries())
        jank.shifts.push({
          t: Math.round(entry.startTime),
          value: entry.value,
          hadRecentInput: entry.hadRecentInput,
          sources: (entry.sources ?? []).map((source) => ({
            node: describe(source.node),
            text: (source.node?.textContent ?? "").replace(/\s+/g, " ").trim().slice(0, 60),
            before: rect(source.previousRect),
            after: rect(source.currentRect),
          })),
        });
    }).observe({ type: "layout-shift", buffered: true });
  } catch {}
  /* An unstyled frame: visible text before the tokens apply, or a visible element whose
   * component styles (Svelte's scoped svelte-<hash> class) are in no loaded stylesheet. */
  const hashesBySheet = new WeakMap();
  let sheetCount = -1;
  let loaded = new Set();
  const loadedHashes = () => {
    if (document.styleSheets.length === sheetCount) return loaded;
    sheetCount = document.styleSheets.length;
    loaded = new Set();
    for (const sheet of document.styleSheets) {
      let hashes = hashesBySheet.get(sheet);
      if (!hashes) {
        try {
          hashes = new Set([...sheet.cssRules].map((rule) => rule.cssText).join("\n").match(/svelte-[a-z0-9]+/g) ?? []);
        } catch {
          sheetCount = -1; // not readable yet; look again next frame
          continue;
        }
        hashesBySheet.set(sheet, hashes);
      }
      for (const hash of hashes) loaded.add(hash);
    }
    return loaded;
  };
  let until = performance.now() + 15000;
  let running = false;
  const frame = () => {
    const now = performance.now();
    if (document.body) {
      /* The global tokens (:root) and, once the cockpit's frame exists, its own (.bridge-root). */
      const shell = document.querySelector(".bridge-root");
      const token =
        getComputedStyle(document.documentElement).getPropertyValue("--t-body").trim() &&
        (!shell || getComputedStyle(shell).getPropertyValue("--topbar-h").trim());
      if (!token) {
        const text = (document.body.innerText ?? "").replace(/\s+/g, " ").trim();
        if (text) jank.unstyled.push({ t: Math.round(now), cause: "tokens", text: text.slice(0, 80) });
      } else {
        const hashes = loadedHashes();
        for (const element of document.querySelectorAll('[class*="svelte-"]')) {
          const hash = [...element.classList].find((name) => name.startsWith("svelte-"));
          if (!hash || hashes.has(hash)) continue;
          if (!element.checkVisibility({ visibilityProperty: true, opacityProperty: true })) continue;
          const text = (element.innerText ?? "").replace(/\s+/g, " ").trim();
          const r = element.getBoundingClientRect();
          if (!text && (r.width < 4 || r.height < 4)) continue;
          jank.unstyled.push({ t: Math.round(now), cause: hash, node: describe(element), text: text.slice(0, 80) });
          break;
        }
      }
      if (document.fonts?.status === "loading" && (document.body.innerText ?? "").trim()) jank.fontFrames += 1;
    }
    if (now < until) requestAnimationFrame(frame);
    else running = false;
  };
  /* Watch the next `ms` of frames (a visit that navigates within the app calls this first). */
  window.__jankWatch = (ms) => {
    until = performance.now() + ms;
    if (!running) {
      running = true;
      requestAnimationFrame(frame);
    }
  };
  running = true;
  requestAnimationFrame(frame);
}

/* After the reveal: every frame for `ms`, the rectangle of each landmark; any that moves or
 * resizes by a pixel or more, with no input in the 500 ms before, is reported once. */
async function watchLandmarks(page, ms) {
  return page.evaluate(async (ms) => {
    const selector = [
      ".bridge-topbar", ".tb-tabs", ".bridge-dock", "#main-content", ".resizable-sidebar", "#bridge-exrail",
      "main aside", "main header", "main h1", "main h2", "main [role=tablist]", "main textarea",
      "main [contenteditable=true]", "main [role=list] > :nth-child(-n+3)", "main ul > li:nth-child(-n+3)",
      "main [role=row]:nth-child(-n+3)", ".exr-composer", ".exr-msgs",
    ].join(",");
    const seen = new Map();
    const moved = [];
    const describe = window.__jankDescribe;
    const started = performance.now();
    const fixed = (element) => {
      for (let node = element; node; node = node.parentElement) {
        const style = getComputedStyle(node);
        if (style.position === "fixed" && node.id !== "main-content") return true;
        if (style.display === "none") return true;
      }
      return false;
    };
    /* What made a landmark move: every element of the page as it was revealed, so a report names
     * the innermost elements that changed size and those that arrived since (a reserved line
     * that came late, a button that grew), not only the landmark they pushed. */
    const root = document.querySelector("#main-content") ?? document.body;
    const sized = (element) => {
      const r = element.getBoundingClientRect();
      return { w: Math.round(r.width), h: Math.round(r.height) };
    };
    const STYLE = ["paddingTop", "paddingBottom", "borderTopWidth", "borderBottomWidth", "minHeight", "lineHeight", "fontSize", "display", "gridTemplateRows", "rowGap", "marginTop", "marginBottom", "transform"];
    const styled = (element) => {
      const style = getComputedStyle(element);
      return STYLE.map((name) => style[name]).join("|");
    };
    const before = new Map([...root.querySelectorAll("*")].map((element) => [element, { ...sized(element), style: styled(element), scroll: element.scrollTop }]));
    const causes = (landmark) => {
      const changed = [];
      for (const element of root.querySelectorAll("*")) {
        const now = sized(element);
        if (!now.w && !now.h) continue;
        const was = before.get(element);
        if (was && was.w === now.w && was.h === now.h) continue;
        changed.push({ element, was: was ?? null, now });
      }
      const set = new Set(changed.map((entry) => entry.element));
      const found = changed
        .filter((entry) => ![...set].some((other) => other !== entry.element && entry.element.contains(other)))
        .slice(0, 8)
        .map((entry) => {
          const restyled = entry.was && entry.was.style !== styled(entry.element);
          return {
            node: describe(entry.element),
            text: (entry.element.textContent ?? "").replace(/\s+/g, " ").trim().slice(0, 40),
            was: entry.was && { w: entry.was.w, h: entry.was.h },
            now: entry.now,
            ...(restyled ? { styleWas: entry.was.style, styleNow: styled(entry.element) } : {}),
          };
        });
      /* A landmark that moved with nothing resized: a scrolled ancestor, or one restyled. */
      for (let node = landmark; node && node !== document.body; node = node.parentElement) {
        const was = before.get(node);
        if (!was) continue;
        if (was.scroll !== node.scrollTop) found.push({ node: describe(node), scrolled: [was.scroll, node.scrollTop] });
        if (was.style !== styled(node)) found.push({ node: describe(node), styleWas: was.style, styleNow: styled(node) });
      }
      return found;
    };
    while (performance.now() - started < ms) {
      const now = performance.now();
      const input = window.__jank.inputs.some((t) => t <= now && now - t < 500);
      for (const element of document.querySelectorAll(selector)) {
        const r = element.getBoundingClientRect();
        if (!r.width && !r.height) continue;
        const box = { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) };
        const first = seen.get(element);
        if (!first) { seen.set(element, { box, reported: false }); continue; }
        if (first.reported) continue;
        const delta = Math.max(Math.abs(box.x - first.box.x), Math.abs(box.y - first.box.y), Math.abs(box.w - first.box.w), Math.abs(box.h - first.box.h));
        if (delta >= 1) {
          first.reported = true;
          if (!input && !fixed(element))
            moved.push({ t: Math.round(now), node: describe(element), text: (element.textContent ?? "").replace(/\s+/g, " ").trim().slice(0, 60), before: first.box, after: box, delta, causes: causes(element) });
        }
      }
      await new Promise((resolve) => requestAnimationFrame(resolve));
    }
    return moved;
  }, ms);
}

/* Named shared edges, each a sentence a designer would say; a pixel off fails. Measured on the
 * settled page. The panes are named: a surface's sidebar, its main pane and the Exec rail. */
const SIDEBARS = [".sidebar-nav", ".attention-index", ".conversation-index"];
const MAIN_PANES = [".sidebar-main", ".attention-focus", ".conversation-main"];
async function gridEdges(page) {
  return page.evaluate(([SIDEBARS, MAIN_PANES]) => {
    const tokens = document.querySelector(".bridge-root") ?? document.documentElement;
    const px = (name) => parseFloat(getComputedStyle(tokens).getPropertyValue(name));
    const box = (element) => element.getBoundingClientRect();
    const visible = (element) => !!element && element.checkVisibility({ visibilityProperty: true }) && box(element).width > 0 && box(element).height > 0;
    const name = (element) => window.__jankDescribe(element).split(" > ").pop();
    const edges = [];
    const check = (edge, actual, expected, detail) => {
      if (![actual, expected].every(Number.isFinite)) return;
      edges.push({ name: edge, actual: Math.round(actual * 10) / 10, expected: Math.round(expected * 10) / 10, ok: Math.abs(actual - expected) < 1, detail });
    };
    const topbar = document.querySelector(".bridge-topbar");
    const content = document.querySelector("#main-content");
    if (!visible(topbar) || !visible(content)) return edges;
    const frame = box(content);
    const first = (selectors) => selectors.flatMap((selector) => [...content.querySelectorAll(selector)]).find(visible);
    const sidebar = first(SIDEBARS);
    const main = first(MAIN_PANES);
    const rail = document.querySelector("#bridge-exrail.open");
    const wide = matchMedia("(min-width: 900px)").matches;
    check("the top bar is --topbar-h tall", box(topbar).height, px("--topbar-h"));
    check("the content starts at the top bar's left edge", frame.left, box(topbar).left);
    if (visible(rail)) {
      check("the Exec rail starts at the content's top edge", box(rail).top, frame.top);
      check("the Exec rail is --pane-gap from the content", box(rail).left - frame.right, px("--pane-gap"));
      const head = rail.querySelector(".exr-head");
      if (visible(head)) check("the Exec rail's header is --chat-head-h tall", box(head).height, px("--chat-head-h"));
    }
    if (sidebar) {
      check("the sidebar starts at the content's left edge", box(sidebar).left, frame.left, name(sidebar));
      check("the sidebar starts at the content's top edge", box(sidebar).top, frame.top, name(sidebar));
      const tab = [...document.querySelectorAll(".tb-tabs .tb-tab")].find(visible);
      if (wide && tab) check("the sidebar's right edge lines up with the top tabs' left edge (F1)", box(sidebar).right, box(tab).left, name(sidebar));
    }
    if (main) check("the main pane starts at the content's top edge", box(main).top, frame.top, name(main));
    /* A boxed sidebar (a pane of its own, like the Inbox list) sits one seam from the main pane; an
     * unboxed one (Work, Library, Apps, Company) is part of the canvas, so no seam is measured. */
    const boxed = sidebar && parseFloat(getComputedStyle(sidebar).borderTopWidth) > 0;
    if (boxed && main && box(main).left >= box(sidebar).right - 1)
      check("the sidebar and the main pane are --pane-gap apart", box(main).left - box(sidebar).right, px("--pane-gap"), `${name(sidebar)} | ${name(main)}`);
    /* The header row: each pane's own header, where it opens the pane, ends on one line. */
    const head = (pane, selectors) => pane && [...pane.querySelectorAll(selectors)].find((node) => visible(node) && box(node).top - box(pane).top < 12 && box(node).height < 120);
    const heads = [
      head(sidebar, ".sidebar-head, .inbox-head, header"),
      head(main, "header, .head, .page-head, .cockpit-pane-head"),
      visible(rail) ? rail.querySelector(".exr-head") : null,
    ].filter(Boolean);
    if (wide && heads.length > 1) {
      const bottoms = heads.map((node) => box(node).bottom);
      check("the panes' headers end on one line", Math.max(...bottoms) - Math.min(...bottoms), 0, heads.map((node) => `${name(node)} ${Math.round(box(node).bottom)}`).join(" | "));
    }
    return edges;
  }, [SIDEBARS, MAIN_PANES]);
}

/* The page's structure for review: panes and headers with their rectangles. */
async function outline(page) {
  return page.evaluate(() => {
    const rows = [];
    const walk = (element, depth) => {
      if (depth > 11) return;
      for (const child of element.children) {
        const r = child.getBoundingClientRect();
        if (r.width >= 100 && r.height >= 20)
          rows.push(`${"  ".repeat(depth)}${window.__jankDescribe(child).split(" > ").pop()} [${Math.round(r.x)},${Math.round(r.y)} ${Math.round(r.width)}x${Math.round(r.height)}]`);
        walk(child, depth + 1);
      }
    };
    walk(document.body, 0);
    return rows.join("\n");
  });
}

// ------------------------------------------------------------------ the gate's bookkeeping
const visits = [];
const failures = [];
const filmed = new Set();
function matchesVisit(pattern, id) {
  const a = pattern.split("/"), b = id.split("/");
  return a.length === b.length && a.every((part, i) => part === "*" || part === b[i]);
}
function allowed(failure) {
  return allowlist.entries.find(
    (entry) =>
      matchesVisit(entry.visit, failure.visit) &&
      entry.kind === failure.kind &&
      (entry.element === "*" || failure.element.includes(entry.element)) &&
      (failure.value === undefined || entry.max === undefined || failure.value <= entry.max * 1.5 + 0.001),
  );
}

function routeFor(state) {
  if (state !== "slow" && state !== "failure") return null;
  return async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (request.method() !== "GET" || path.endsWith("/principal")) return route.continue();
    if (state === "slow") {
      await pause(SLOW_MS);
      return route.continue().catch(() => {});
    }
    return route.fulfill({ status: 503, contentType: "application/json", body: JSON.stringify({ error: "unavailable", message: "Service unavailable" }) });
  };
}

/* The wall clock stands still for a visit: a relative time ("9m ago") ticking over to "10m ago"
 * mid-measurement is the clock, not the page. Only Date is fixed, at the moment the visit's
 * context opens; timers, animation frames and performance.now run, so a loading label replaced
 * by its value, or anything else the page swaps, still moves and still fails. */
function fixedClock(now) {
  const Real = Date;
  function Fixed(...parts) {
    if (!new.target) return new Real(now).toString();
    return parts.length ? new Real(...parts) : new Real(now);
  }
  Fixed.prototype = Real.prototype;
  Fixed.now = () => now;
  Fixed.parse = Real.parse;
  Fixed.UTC = Real.UTC;
  globalThis.Date = Fixed;
}

async function freshContext(who, width, state) {
  const context = await browser.newContext({
    viewport: WIDTHS[width],
    storageState: { cookies: await who.context.cookies(), origins: [] },
  });
  await context.addInitScript(fixedClock, Date.now());
  await context.addInitScript(probe);
  /* What the page said before a visit failed: its errors and console errors. */
  context.on("page", (page) => {
    page.__problems = [];
    page.on("pageerror", (error) => page.__problems.push(`pageerror: ${error.message}`));
    page.on("console", (message) => {
      if (message.type() === "error") page.__problems.push(`console: ${message.text().slice(0, 200)}`);
    });
  });
  const handler = routeFor(state);
  if (handler) await context.route(`${core}/api/**`, handler);
  return context;
}

async function revealed(page) {
  await page.waitForSelector(".company-browser-link-capture:not(.first-paint-pending)", { timeout: 60000 });
}

async function measure(page, id, { since = 0, state, shot }) {
  const errors = [];
  const onError = (error) => errors.push(error.message);
  page.on("pageerror", onError);
  try {
    await revealed(page);
    if (state === "slow" && shot) {
      await pause(300);
      await page.screenshot({ path: join(out, `${shot}-loading.png`) }).catch(() => {});
    }
    const moved = await watchLandmarks(page, state === "slow" ? SLOW_SETTLE_MS : SETTLE_MS);
    const jank = await page.evaluate(() => window.__jank);
    const grid = await gridEdges(page);
    if (shot) await page.screenshot({ path: join(out, `${shot}.png`) }).catch(() => {});
    const shifts = jank.shifts.filter((entry) => entry.t >= since && entry.value > 0);
    const intentional = shifts.filter((entry) => entry.hadRecentInput || jank.inputs.some((t) => t <= entry.t && entry.t - t < 500));
    const unexpected = shifts.filter((entry) => !intentional.includes(entry));
    const record = {
      visit: id,
      url: page.url(),
      score: unexpected.reduce((sum, entry) => sum + entry.value, 0),
      intentional: intentional.reduce((sum, entry) => sum + entry.value, 0),
      shifts: unexpected,
      moved,
      unstyled: jank.unstyled.filter((frame) => frame.t >= since),
      fontFrames: jank.fontFrames,
      grid,
      errors,
    };
    visits.push(record);
    const found = [];
    const byElement = new Map();
    for (const entry of unexpected) {
      const element = entry.sources[0]?.node ?? "(no source)";
      byElement.set(element, (byElement.get(element) ?? 0) + entry.value);
    }
    for (const [element, value] of byElement)
      found.push({ visit: id, kind: "layout-shift", element, value: Math.round(value * 10000) / 10000 });
    for (const entry of moved) found.push({ visit: id, kind: "movement", element: entry.node, value: entry.delta, detail: entry });
    if (record.unstyled.length) found.push({ visit: id, kind: "unstyled", element: record.unstyled[0].node ?? "tokens", value: record.unstyled.length, detail: record.unstyled.slice(0, 3) });
    for (const edge of grid.filter((edge) => !edge.ok)) found.push({ visit: id, kind: "grid", element: edge.name, value: Math.abs(edge.actual - edge.expected), detail: edge });
    for (const failure of found) {
      failure.known = !!allowed(failure);
      failures.push(failure);
    }
    if (found.length) {
      const safe = id.replaceAll("/", "_");
      await page.screenshot({ path: join(out, `fail-${safe}.png`) }).catch(() => {});
      await writeFile(join(out, `fail-${safe}.json`), JSON.stringify({ found, record }, null, 2));
    }
    return record;
  } finally {
    page.off("pageerror", onError);
  }
}

/* Replay a failing cold load with the screencast on: the frames and their timestamps. */
async function filmstrip(who, width, state, url, id) {
  if (filmed.has(id) || filmed.size >= 16) return;
  filmed.add(id);
  const context = await freshContext(who, width, state);
  try {
    const page = await context.newPage();
    const cdp = await context.newCDPSession(page);
    const dir = join(out, `film-${id.replaceAll("/", "_")}`);
    await mkdir(dir, { recursive: true });
    let first;
    const writes = [];
    cdp.on("Page.screencastFrame", (frame) => {
      first ??= frame.metadata.timestamp;
      const ms = Math.round((frame.metadata.timestamp - first) * 1000);
      writes.push(writeFile(join(dir, `${String(ms).padStart(6, "0")}ms.jpg`), Buffer.from(frame.data, "base64")));
      cdp.send("Page.screencastFrameAck", { sessionId: frame.sessionId }).catch(() => {});
    });
    await cdp.send("Page.startScreencast", { format: "jpeg", quality: 70, everyNthFrame: 1 });
    await page.goto(url);
    await revealed(page).catch(() => {});
    await pause(state === "slow" ? SLOW_SETTLE_MS : SETTLE_MS);
    await cdp.send("Page.stopScreencast").catch(() => {});
    await Promise.all(writes);
  } finally {
    await context.close();
  }
}

/* The surfaces this role's navigation offers, in order. */
async function surfacesOf(who) {
  await who.page.setViewportSize(WIDTHS.desktop);
  await who.page.goto(`${core}/${company}`);
  await revealed(who.page);
  const hrefs = await who.page.$$eval(".tb-tabs a[href]", (links) => links.map((link) => new URL(link.href).pathname));
  const names = { "": "inbox", work: "work", library: "library", people: "people", apps: "apps", company: "company" };
  return [...new Set(hrefs)]
    .map((path) => ({ path, name: names[path.split("/")[2] ?? ""] }))
    .filter((surface) => surface.name);
}

async function navTab(page, surface) {
  const link = page.locator(`.tb-tabs a[href="${surface.path}"], .bridge-dock a[href="${surface.path}"]`).filter({ visible: true }).first();
  const since = await page.evaluate(() => {
    window.__jankWatch(15000);
    return performance.now();
  });
  await link.click();
  await page.waitForURL((url) => url.pathname === surface.path);
  return since;
}

/* The host's network changing under Chromium (ERR_NETWORK_CHANGED, as docker creates and removes
 * networks for other work on the computer) aborts the page's own script loads. That is the
 * computer, not the page: such a visit is measured again, once, from scratch. */
const ENVIRONMENT = /ERR_NETWORK_CHANGED|Failed to fetch dynamically imported module/;
async function visitOnce(who, width, state, id, body) {
  for (let attempt = 1; ; attempt += 1) {
    const marks = [visits.length, failures.length];
    const context = await freshContext(who, width, state);
    let step = { id };
    try {
      await body(await context.newPage(), step);
      return;
    } catch (error) {
      const problems = context.pages().flatMap((open) => open.__problems ?? []).slice(-12);
      if (attempt === 1 && problems.some((line) => ENVIRONMENT.test(line))) {
        visits.length = marks[0];
        failures.length = marks[1];
        console.log(`${step.id}: the host network changed under the browser; measuring again`);
        continue;
      }
      failures.push({ visit: step.id, kind: "error", element: "visit", value: 1, detail: `${String(error.message ?? error).slice(0, 300)}\n${problems.join("\n")}`, known: false });
      for (const open of context.pages()) await open.screenshot({ path: join(out, `error-${step.id.replaceAll("/", "_")}.png`) }).catch(() => {});
      return;
    } finally {
      await context.close();
    }
  }
}

async function pass_(who, role, state, surfaces, loads = ["cold", "warm", "nav"]) {
  for (const width of Object.keys(WIDTHS)) {
    for (const [index, surface] of surfaces.entries()) {
      const url = `${core}${surface.path}`;
      const key = (load) => `${role}/${state}/${width}/${load}/${surface.name}`;
      if (!loads.some((load) => wanted(key(load)))) continue;
      await visitOnce(who, width, state, key(loads[0]), async (page, step) => {
        if (loads.includes("cold")) {
          step.id = key("cold");
          await page.goto(url);
          const record = await measure(page, key("cold"), { state, shot: `${role}-${state}-${width}-${surface.name}` });
          if (failures.some((failure) => failure.visit === record.visit)) await filmstrip(who, width, state, url, record.visit);
        }
        if (loads.includes("warm")) {
          step.id = key("warm");
          await page.goto(url);
          await measure(page, key("warm"), { state });
        }
        if (loads.includes("nav")) {
          step.id = key("nav");
          // Arrive from the previous surface by its navigation tab, once it has settled.
          const from = surfaces[(index + surfaces.length - 1) % surfaces.length];
          await page.goto(`${core}${from.path}`);
          await revealed(page);
          await pause(state === "slow" ? SLOW_SETTLE_MS : 800);
          const since = await navTab(page, surface);
          await measure(page, key("nav"), { state, since });
        }
      });
    }
  }
}

/* Inbox cards: each item the role sees, opened directly, cold, populated and slow. */
async function cardPass(who, role) {
  await who.page.setViewportSize(WIDTHS.desktop);
  await who.page.goto(`${core}/${company}?filter=all`);
  await revealed(who.page);
  await pause(1500);
  const cards = await who.page.$$eval("[data-inbox-key]", (rows) =>
    rows.map((row) => ({ key: row.getAttribute("data-inbox-key"), href: row.querySelector("a[href]")?.getAttribute("href"), kind: row.querySelector(".kind")?.getAttribute("title") ?? "update" })),
  );
  const kinds = new Map();
  for (const card of cards) if (card.href && !kinds.has(card.kind)) kinds.set(card.kind, card);
  console.log(`${role} cards: ${cards.length} items, kinds ${[...kinds.keys()].join(", ")}`);
  for (const [kind, card] of kinds) {
    const name = `card-${kind.toLowerCase().replace(/[^a-z0-9]+/g, "-")}`;
    for (const state of ["populated", "slow"])
      for (const width of Object.keys(WIDTHS)) {
        const id = `${role}/${state}/${width}/cold/${name}`;
        if (!wanted(id)) continue;
        const url = new URL(card.href, core).href;
        await visitOnce(who, width, state, id, async (page) => {
          await page.goto(url);
          const record = await measure(page, id, { state, shot: `${role}-${state}-${width}-${name}` });
          if (failures.some((failure) => failure.visit === record.visit)) await filmstrip(who, width, state, url, record.visit);
        });
      }
  }
  return [...kinds.keys()];
}

const paragraph = (text) => ({ type: "paragraph", attrs: { block_id: randomUUID() }, content: [{ type: "text", text }] });
async function seed(owner, alice) {
  const titles = [
    "Harbour Street café renovation: launch plan, budget and the supplier shortlist for spring",
    "Brand voice",
    "Weekly notes",
  ];
  const documents = [];
  /* Suggestions (S109 E4): the plan also carries a suggested replacement, so its margin holds a
   * suggestion's card beside the comments' and the marks draw in their author's colour. */
  const suggested = { id: "jank-suggestion", author: "exec", at: "2026-11-03T09:00:00.000Z", thread: null };
  const suggestion = {
    type: "paragraph",
    attrs: { block_id: randomUUID() },
    content: [
      { type: "text", text: "We open in " },
      { type: "text", text: "April", marks: [{ type: "suggestedDelete", attrs: suggested }] },
      { type: "text", text: "March", marks: [{ type: "suggestedInsert", attrs: suggested }] },
      { type: "text", text: "." },
    ],
  };
  for (const [index, title] of titles.entries())
    documents.push(
      await coreApi(owner, `${base}/documents`, {
        method: "POST",
        expected: 201,
        body: { title, kind: "brief", visibility: "company", content_json: { type: "doc", content: [paragraph(`${title}. First draft.`), paragraph("A second paragraph with more words in it."), ...(index === 0 ? [suggestion] : [])] }, reason: "Jank gate fixture" },
      }),
    );
  await coreApi(owner, `${base}/sheets`, { method: "POST", body: { id: randomUUID(), title: "Budget", visibility: "company" } });
  for (const objective of ["Open the Harbour Street café by the first week of spring", "Ship the new menu"])
    await coreApi(owner, `${base}/goals`, { method: "POST", expected: 201, body: { objective } });
  await daemonCommand({ cmd: "actor-create", company, as_actor: "copy-critic", role: "Copywriter", name: "Wren", actor: "exec", reason: "Jank gate fixture" });
  await daemonCommand({ cmd: "team-create", company, name: "Website", to: "copy-critic", body: "Copy is plain.", actor: "exec" });
  const room = await coreApi(owner, `${base}/rooms`, {
    method: "POST",
    body: { kind: "group", title: "Launch room", command_id: randomUUID(), participant_actor_ids: [alice.principal.actor_id] },
  });
  for (const body of ["Kickoff at nine.", "Bring the draft menu and the supplier quotes, please."])
    await coreApi(owner, `${base}/rooms/${room.id}/messages`, { method: "POST", expected: 201, body: { body, command_id: randomUUID() } });
  await coreApi(owner, `${base}/rooms/${room.id}/messages`, {
    method: "POST",
    expected: 201,
    body: { body: "Alice, can you confirm the budget?", command_id: randomUUID(), mentions: [{ actor_id: alice.principal.actor_id, expected_response: "Confirm the budget" }] },
  });
  const requester = alice.principal.actor_id;
  await direction({ operation: "request_file", requester, class: null, team: null, summary: "Raise the ads budget to $200", change: "Monthly ads envelope from $100 to $200", reason: "Launch week" });
  await direction({ operation: "request_file", requester, class: "staffing", team: "Website", summary: "Give the copywriter a stronger model", change: "The copywriter on the frontier tier", reason: "" });
  await direction({ operation: "charter_propose", requester, markdown: "# Charter\n\nWe renovate small cafés.", reason: "Alice suggested it", source_message: null });
  const reviewed = documents[1].document_id;
  let view = await coreApi(owner, `${base}/documents/${reviewed}`);
  await coreApi(owner, `${base}/documents/${reviewed}/participants/${requester}`, { method: "PUT", body: { expected_document_version: view.document.version, access: "edit" } });
  view = await coreApi(owner, `${base}/documents/${reviewed}`);
  await coreApi(owner, `${base}/documents/${reviewed}/reviews`, {
    method: "POST",
    expected: 201,
    body: { expected_document_version: view.document.version, expected_current_version_id: view.current_version.version.id, summary: "Alice checks the voice", reviewer_actor_id: requester },
  });
  /* Comments in the margin (S109 E4): the plan carries two threads on its words, so its view
   * loads with cards beside the text, and must not move the text when they arrive. */
  const plan = documents[0].document_id;
  const planView = await coreApi(owner, `${base}/documents/${plan}`);
  const [firstBlock, secondBlock] = planView.current_version.version.content_json.content;
  for (const [block, quote, text] of [
    [firstBlock, "First draft", "Is this still the first draft?"],
    [secondBlock, "more words", "Fewer words here, please."],
  ])
    await coreApi(owner, `${base}/documents/${plan}/comments`, {
      method: "POST",
      expected: 201,
      body: { block_id: block.attrs.block_id, quote, content_json: { type: "doc", content: [paragraph(text)] } },
    });
  /* An email Exec asks the owner to send (S109 H2), with a long subject and body. */
  await daemonCommand({ cmd: "company-set", company, state: "credentials.RESEND_API_KEY", body: "env:RESTLESS_JANK_RESEND_KEY" });
  const asked = await daemonCommand({
    cmd: "email-send",
    company,
    actor: "exec",
    email_request: {
      from: "hello@harbour-street-cafe-renovations.example.com",
      from_name: "Harbour Street Café",
      to: "the.supplier.with.a.long.address@spring-joinery-and-fitouts.example.net",
      subject: "Harbour Street café renovation: confirming the joinery quote, the delivery window and the site access for the first week of spring",
      text: Array.from({ length: 12 }, (_, line) => `Line ${line + 1}: the counters, the shelving and the bench seat, delivered to the lane behind the café before nine.`).join("\n"),
      effect_key: `jank-email-${randomUUID()}`,
    },
  });
  assert.equal(asked.status, "waiting_for_owner");
  await until(async () => (await coreApi(owner, `${base}/attention`)).items.length >= 2, "seeded Inbox items", 30);
  pass("seeded documents, a sheet, goals, a team, a room with a mention, direction requests, a charter draft, a review, margin comments and an email to approve");
  return { path: `/${company}/library/documents?document=${plan}`, name: "document" };
}

await run("multiplayer-jank", async () => {
  const owner = await boot();
  const alice = await person("alice");
  await invite(owner, [alice]);
  for (const who of [owner, alice]) await who.context.addInitScript(probe);

  const surfaces = { owner: await surfacesOf(owner), member: await surfacesOf(alice) };
  console.log(`surfaces: owner ${surfaces.owner.map((s) => s.name).join(", ")}; member ${surfaces.member.map((s) => s.name).join(", ")}`);
  for (const [role, list] of Object.entries(surfaces))
    for (const surface of list) {
      await (role === "owner" ? owner : alice).page.goto(`${core}${surface.path}`);
      await revealed((role === "owner" ? owner : alice).page);
      await pause(800);
      await writeFile(join(out, `outline-${role}-${surface.name}.txt`), await outline((role === "owner" ? owner : alice).page));
    }

  await pass_(owner, "owner", "empty", surfaces.owner);
  await pass_(alice, "member", "empty", surfaces.member, ["cold"]);
  pass("empty state measured");

  const documentView = await seed(owner, alice);
  for (const state of ["populated", "slow", "failure"]) {
    // With every read failing the shell has no navigation to arrive by, so that state has no nav load.
    await pass_(owner, "owner", state, surfaces.owner, state === "failure" ? ["cold", "warm"] : undefined);
    await pass_(alice, "member", state, surfaces.member, state === "failure" ? ["cold"] : ["cold", "nav"]);
    // A document with margin comments: opened by its address (no tab arrives at it).
    await pass_(owner, "owner", state, [documentView], ["cold", "warm"]);
    await pass_(alice, "member", state, [documentView], ["cold"]);
    pass(`${state} state measured`);
  }
  const kinds = { owner: await cardPass(owner, "owner"), member: await cardPass(alice, "member") };

  const fresh = failures.filter((failure) => !failure.known);
  const known = failures.filter((failure) => failure.known);
  const stale = allowlist.entries.filter((entry) => !failures.some((failure) => failure.known && allowed(failure) === entry));
  const summary = {
    visits: visits.length,
    surfaces,
    cardKinds: kinds,
    failures: fresh.length,
    known: known.length,
    staleAllowlist: stale.map((entry) => `${entry.visit} ${entry.kind} ${entry.element}`),
    worst: visits.map((visit) => [visit.visit, Math.round(visit.score * 10000) / 10000]).filter(([, score]) => score > 0).sort((a, b) => b[1] - a[1]).slice(0, 20),
    fontFrames: visits.filter((visit) => visit.fontFrames).map((visit) => [visit.visit, visit.fontFrames]),
  };
  await writeFile(join(out, "results.json"), JSON.stringify({ summary, failures, visits }, null, 2));
  if (process.env.RESTLESS_JANK_RECORD)
    await writeFile(
      join(out, "allowlist-observed.json"),
      JSON.stringify(
        {
          entries: Object.values(
            failures.reduce((groups, { visit, kind, element, value }) => {
              const [, , width, , surface] = visit.split("/");
              const key = `${width}/${surface}/${kind}/${element}`;
              const group = (groups[key] ??= { visit: `*/*/${width}/*/${surface}`, kind, element, max: 0, seen: [] });
              group.max = Math.max(group.max, value);
              group.seen.push(visit);
              return groups;
            }, {}),
          ),
        },
        null,
        2,
      ),
    );
  console.log(`JANK ${JSON.stringify(summary, null, 2)}`);
  for (const failure of fresh) console.log(`JANK FAIL ${failure.visit} ${failure.kind} ${failure.element} ${failure.value}`);
  for (const entry of summary.staleAllowlist) note(`jank allowlist entry no longer observed: ${entry}`);
  assert.equal(fresh.length, 0, `${fresh.length} new jank failures across ${visits.length} visits; evidence in ${out}`);
  pass(`zero new jank across ${visits.length} visits (${known.length} known, allowlisted)`);
});
