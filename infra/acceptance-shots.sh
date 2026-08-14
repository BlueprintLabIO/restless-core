#!/usr/bin/env bash
# Capture one screenshot per mode so the owner can judge the builds side by side.
#
# docs/specs/evaluation-dogfood.md §21.2: for creative outcomes a human gives
# final acceptance, and the harness's job is to make that inspection easy. It
# is NOT to score the result — §25 rule 2, agent self-assessment cannot be the
# sole evidence for material success.
#
# Usage: infra/acceptance-shots.sh <scenario-slug> [out-dir]
set -euo pipefail

SCENARIO="${1:?scenario slug}"
OUT="${2:-/tmp/$SCENARIO-acceptance}"
MODES=(single_agent minimal_team orgintel)
mkdir -p "$OUT"

mode_company() { printf '%s_%s' "$(echo "$1" | tr -cd 'a-z0-9')" "$2" | cut -c1-40; }

for mode in "${MODES[@]}"; do
  name="$(mode_company "$SCENARIO" "$mode")"
  container="restless-co-$name"
  echo "=== $mode ($container)"
  if ! docker ps --format '{{.Names}}' | grep -qx "$container"; then
    echo "  no container — skipped"
    continue
  fi

  # What actually changed, from git rather than from the agent's own account.
  docker exec "$container" sh -c '
    cd /company/repos/cosmon-game 2>/dev/null || exit 0
    git -c safe.directory=$PWD log --oneline 514b7b3..HEAD 2>/dev/null | sed "s/^/  commit: /"
    git -c safe.directory=$PWD diff --stat 514b7b3..HEAD 2>/dev/null | tail -1 | sed "s/^/  diff: /"
  ' || true

  docker exec "$container" sh -c '
    cd /company/repos/cosmon-game 2>/dev/null || exit 1
    pkill -f "http.server 8799" 2>/dev/null || true
    (python3 -m http.server 8799 >/dev/null 2>&1 &)
    sleep 2
    cat > /tmp/shot.mjs <<"EOF"
import { chromium } from "playwright";
const b = await chromium.launch({ executablePath: "/usr/bin/chromium", args: ["--no-sandbox"] });
const p = await b.newPage({ viewport: { width: 1280, height: 800 } });
const errors = [];
p.on("pageerror", e => errors.push(String(e)));
await p.goto("http://127.0.0.1:8799/index.html", { waitUntil: "domcontentloaded" });
await p.waitForTimeout(3500);
// Past the starter picker if one is showing, so the shot is of the world.
try { await p.click("text=/Choose /i", { timeout: 2500 }); await p.waitForTimeout(3000); } catch {}
await p.screenshot({ path: "/tmp/shot.png" });
const drew = await p.evaluate(() => {
  const c = document.querySelector("canvas");
  return c ? `${c.width}x${c.height}` : "no canvas";
});
console.log(`  canvas: ${drew}`);
console.log(`  pageerrors: ${errors.length}${errors.length ? " -> " + errors[0].slice(0,90) : ""}`);
await b.close();
EOF
    node /tmp/shot.mjs 2>&1 | tail -4
    pkill -f "http.server 8799" 2>/dev/null || true
  ' || echo "  build did not load"

  docker cp "$container:/tmp/shot.png" "$OUT/$mode.png" 2>/dev/null && echo "  shot: $OUT/$mode.png" || echo "  no shot"
done

echo
echo "Judge these against the success contract in docs/scenarios/$SCENARIO.md."
echo "A screenshot proves it renders. It does not prove the caverns are reachable"
echo "or the mini-boss gates them — load it and play if the shot looks plausible."
