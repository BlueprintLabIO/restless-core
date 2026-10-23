import { chromium } from "playwright";
const b = await chromium.launch(); const p = await b.newPage();
await p.goto("http://127.0.0.1:5199/gallery/zz-picker"); await p.waitForTimeout(2500);
console.log(await p.evaluate(() => { const s = getComputedStyle(document.querySelector(".model-field summary")); return [s.borderTopWidth, s.borderTopColor, s.height, getComputedStyle(document.documentElement).getPropertyValue("--border")]; }));
await b.close();
