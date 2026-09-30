/* The office engine's sprites are the product's own static assets. Copy them next to the page so
 * it serves them from the same path the app does (/vendor/pixel-agents/assets/), including the
 * upstream MIT licence that must travel with them. The copy is git-ignored. */
import { cpSync, existsSync, mkdirSync, rmSync } from "node:fs";

const from = "../web/static/vendor";
const to = "static/vendor";
if (!existsSync(from)) {
  console.error(
    `Missing ${from}. Run from the landing directory of a full checkout.`,
  );
  process.exit(1);
}
rmSync(to, { recursive: true, force: true });
mkdirSync(to, { recursive: true });
cpSync(from, to, { recursive: true });
console.log(`synced ${from} -> ${to}`);
