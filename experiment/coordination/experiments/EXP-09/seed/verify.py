#!/usr/bin/env python3
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parent
LEDGER = ROOT / "publication" / "event-ledger.json"
ALLOWED = {"publish", "update", "defer", "no_op_duplicate", "no_op_irrelevant"}

rows = json.loads(LEDGER.read_text())
if not isinstance(rows, list):
    raise SystemExit("event ledger must be a JSON array")

content_rows = 0
for index, row in enumerate(rows):
    required = {"source_ref", "observed_at", "disposition", "reason", "artifact", "accepted_commit"}
    missing = sorted(required - set(row))
    if missing:
        raise SystemExit(f"ledger row {index} missing {missing}")
    if row["disposition"] not in ALLOWED:
        raise SystemExit(f"ledger row {index} has invalid disposition {row['disposition']!r}")
    if not str(row["source_ref"]).startswith("exp09://"):
        raise SystemExit(f"ledger row {index} lost _test source identity")
    if row["disposition"] in {"publish", "update", "defer"}:
        content_rows += 1
        artifact = row["artifact"]
        if not artifact or not (ROOT / artifact).is_file():
            raise SystemExit(f"ledger row {index} references missing artifact {artifact!r}")
    elif row["artifact"] is not None or row["accepted_commit"] is not None:
        raise SystemExit(f"no-op ledger row {index} must not claim artifact or commit")

article_files = sorted((ROOT / "publication" / "articles").glob("*.md"))
review_files = sorted((ROOT / "publication" / "reviews").glob("*.md"))
for article in article_files:
    text = article.read_text()
    if "—" in text:
        raise SystemExit(f"public article contains an em dash: {article}")
    for heading in ("## What happened", "## Why it matters", "## Where this stops"):
        if heading not in text:
            raise SystemExit(f"article missing {heading!r}: {article}")

print(json.dumps({
    "ledger_rows": len(rows),
    "content_rows": content_rows,
    "articles": [str(path.relative_to(ROOT)) for path in article_files],
    "reviews": [str(path.relative_to(ROOT)) for path in review_files],
}, indent=2))
