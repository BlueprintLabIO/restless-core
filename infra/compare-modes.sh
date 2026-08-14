#!/usr/bin/env bash
# Run one scenario in all three organisation modes and report them comparably.
#
# docs/specs/evaluation-dogfood.md §21.1. The point is the comparison, so
# everything except org_mode is held identical: same mission, model, ceiling,
# starting repo commit, and wake reason. §25 rule 3 — a baseline given worse
# tools proves nothing.
#
# Usage: infra/compare-modes.sh <scenario-slug> <source-company> "<wake reason>"
set -euo pipefail

SCENARIO="${1:?scenario slug, e.g. lumaara-biome}"
SOURCE="${2:?company to clone the starting state from, e.g. cosmon}"
REASON="${3:?the wake reason, identical for every mode}"

ROOT="${RESTLESS_HOME:-$HOME/.restless}"
REPO="$(cd "$(dirname "$0")/.." && pwd)"
OUT="$REPO/docs/scenarios/$SCENARIO-results"
CEILING="${CEILING:-15.0}"
MODEL="${MODEL:-zai/glm-5.2}"
mkdir -p "$OUT"

MODES=(single_agent minimal_team orgintel)

# A company name becomes a Postgres schema name: [a-z_][a-z0-9_]* only.
mode_company() {
  printf '%s_%s' "$(echo "$1" | tr -cd 'a-z0-9')" "$2" | cut -c1-40
}

echo "scenario   : $SCENARIO"
echo "source     : $SOURCE"
echo "modes      : ${MODES[*]}"
echo "ceiling    : \$$CEILING per mode"
echo

# --- provision one company per mode, identical but for org_mode -------------
for mode in "${MODES[@]}"; do
  name="$(mode_company "$SCENARIO" "$mode")"
  echo "=== provisioning $name ($mode)"
  python3 - "$ROOT" "$SOURCE" "$name" "$mode" "$CEILING" "$MODEL" <<'PY'
import sys, pathlib, tomllib
root, source, name, mode, ceiling, model = sys.argv[1:7]
src = pathlib.Path(root) / "companies" / f"{source}.toml"
cfg = tomllib.load(open(src, "rb"))
mission = cfg["mission"]
out = pathlib.Path(root) / "companies" / f"{name}.toml"
out.write_text(
    f'name = "{name}"\n'
    f'mission = """\n{mission}"""\n'
    f'spend_ceiling_usd = {ceiling}\n'
    f'model = "{model}"\n'
    f'org_mode = "{mode}"\n'
)
print(f"  wrote {out}")
PY
  "$REPO/target/debug/restless" up -c "$name" >/dev/null
  # Same starting artifact for every mode: prior work is not a variable.
  docker cp "restless-co-$SOURCE:/company/repos/cosmon-game" "/tmp/$SCENARIO-seed" 2>/dev/null || true
  docker exec "restless-co-$name" sh -c 'mkdir -p /company/repos && rm -rf /company/repos/cosmon-game' || true
  docker cp "/tmp/$SCENARIO-seed" "restless-co-$name:/company/repos/cosmon-game" 2>/dev/null || true
  docker exec "restless-co-$name" sh -c 'chown -R company:company /company/repos' || true
done
rm -rf "/tmp/$SCENARIO-seed"

# --- run each mode ----------------------------------------------------------
for mode in "${MODES[@]}"; do
  name="$(mode_company "$SCENARIO" "$mode")"
  echo
  echo "=== running $name ($mode)  $(date +%H:%M:%S)"
  start=$(date +%s)
  "$REPO/target/debug/restless" wake -c "$name" --reason "$REASON" > "$OUT/$mode.json" 2>&1 || true
  echo "  elapsed $(( ($(date +%s) - start) / 60 ))m"
done

# --- report -----------------------------------------------------------------
echo
python3 - "$ROOT" "$OUT" "${MODES[@]}" <<'PY'
import sys, json, pathlib, subprocess
root, out = sys.argv[1], pathlib.Path(sys.argv[2])
modes = sys.argv[3:]
spool = pathlib.Path(root) / "spend" / "spend.jsonl"
spend = {}
if spool.exists():
    for line in spool.read_text().splitlines():
        if not line.strip():
            continue
        try:
            r = json.loads(line)
        except Exception:
            continue
        spend[r["companyId"]] = spend.get(r["companyId"], 0) + r["costMicroUsd"]

print(f"{'mode':<14} {'term':<9} {'tools':>6} {'staff':>6} {'cost':>8}  reason")
print("-" * 100)
for mode in modes:
    f = out / f"{mode}.json"
    scenario = "".join(ch for ch in out.name.split("-results")[0] if ch.isalnum())
    slug = f"{scenario}_{mode}"[:40]
    cost = spend.get(slug, 0) / 1e6
    try:
        d = json.loads(f.read_text())
        print(f"{mode:<14} {d.get('termination','?'):<9} {len(d.get('tool_calls',[])):>6} "
              f"{len(d.get('spawn_requests',[])):>6} {cost:>7.2f}$  {d.get('reason','')[:60]}")
    except Exception:
        head = f.read_text()[:70].replace("\n", " ") if f.exists() else "(no output)"
        print(f"{mode:<14} {'ERROR':<9} {'':>6} {'':>6} {cost:>7.2f}$  {head}")
print()
print("Acceptance is manual: load each build and judge it against the success")
print("contract in the scenario package. Do not substitute these numbers for that.")
PY
