#!/usr/bin/env python3
from pathlib import Path

ROOT = Path(__file__).resolve().parent
opportunities = sorted((ROOT / "opportunities").glob("*.md"))
reviews = sorted((ROOT / "reviews").glob("*.md"))

if len(opportunities) > 1:
    raise SystemExit(f"expected at most one evolving opportunity dossier, found {len(opportunities)}")

for dossier in opportunities:
    text = dossier.read_text()
    if "—" in text:
        raise SystemExit(f"dossier contains an em dash: {dossier}")
    for heading in (
        "## Qualification",
        "## Evidence and unknowns",
        "## Prepared next action",
        "## Prepared last mile",
        "## Where this stops",
    ):
        if heading not in text:
            raise SystemExit(f"dossier missing {heading!r}: {dossier}")

print({
    "opportunities": [str(path.relative_to(ROOT)) for path in opportunities],
    "reviews": [str(path.relative_to(ROOT)) for path in reviews],
})
