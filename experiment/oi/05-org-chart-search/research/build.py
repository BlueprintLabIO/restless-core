#!/usr/bin/env python3
"""Build the 30 research tasks: 15 invented worlds, 2 questions each.

Every world is a small set of documents about invented things (no real names,
so nothing can be looked up or remembered). The answer to each question is a
set of names that can only be found by joining several documents and applying
the corrections, unit note, withdrawals, former names and void entries.

The key comes from the generator's own records. research/solve.py reads only
the written documents and must reach the same answer.

Seeded: running this twice gives the same bytes.

Usage: python3 research/build.py
"""
import datetime
import hashlib
import json
import pathlib
import random
import shutil

SUITE = pathlib.Path(__file__).resolve().parent.parent
OUT = SUITE / "tasks" / "research"
SEED = "oi-05-research-v1"

# (thing, things, measure, small unit, big unit, factor, event, events, regions, categories)
THEMES = [
    ("reservoir", "reservoirs", "stored volume", "megalitres", "gigalitres", 1000, "inspection", "inspections"),
    ("orchard", "orchards", "harvest", "crates", "pallets", 40, "frost alert", "frost alerts"),
    ("ferry route", "ferry routes", "monthly passengers", "passengers", "thousand passengers", 1000, "cancellation", "cancellations"),
    ("quarry", "quarries", "extracted stone", "tonnes", "kilotonnes", 1000, "blast permit", "blast permits"),
    ("clinic", "clinics", "yearly visits", "visits", "hundred visits", 100, "audit finding", "audit findings"),
    ("wind farm", "wind farms", "yearly output", "megawatt hours", "gigawatt hours", 1000, "outage", "outages"),
    ("library branch", "library branches", "loans", "loans", "hundred loans", 100, "closure day", "closure days"),
    ("vineyard", "vineyards", "pressed juice", "litres", "hectolitres", 100, "hail report", "hail reports"),
    ("depot", "depots", "parcels handled", "parcels", "thousand parcels", 1000, "safety stop", "safety stops"),
    ("observatory", "observatories", "clear hours", "hours", "ten-hour blocks", 10, "instrument fault", "instrument faults"),
    ("fishery", "fisheries", "landed catch", "kilograms", "tonnes", 1000, "quota breach", "quota breaches"),
    ("tram line", "tram lines", "weekly boardings", "boardings", "hundred boardings", 100, "signal fault", "signal faults"),
    ("bakery", "bakeries", "flour used", "kilograms", "sacks", 25, "hygiene visit", "hygiene visits"),
    ("mine shaft", "mine shafts", "ore raised", "tonnes", "wagon loads", 20, "roof fall report", "roof fall reports"),
    ("school", "schools", "enrolled pupils", "pupils", "class groups", 30, "building defect", "building defects"),
]
SYL = ["ar", "bel", "cor", "dun", "el", "fen", "gar", "hol", "is", "jor", "kel", "lum", "mor", "nes", "or", "pel",
       "quin", "ros", "sil", "tor", "ul", "ven", "wim", "yar", "zan", "bra", "cre", "dro", "fal", "gri", "tha", "ske"]
ENDS = ["a", "en", "ick", "ow", "ham", "ford", "eth", "ar", "ine", "os", "wick", "mere", "by", "stead"]
N_THINGS = 36
N_EVENTS = 170


def make_names(rng, n, used):
    out = []
    while len(out) < n:
        name = (rng.choice(SYL) + rng.choice(SYL) + rng.choice(ENDS)).capitalize()
        if name not in used and not any(name in u or u in name for u in used):
            used.add(name)
            out.append(name)
    return out


def table(header, rows):
    lines = ["| " + " | ".join(header) + " |", "|" + "|".join(["---"] * len(header)) + "|"]
    lines += ["| " + " | ".join(str(c) for c in r) + " |" for r in rows]
    return "\n".join(lines) + "\n"


def build_world(w, theme, attempt):
    rng = random.Random(f"{SEED}-{w}-{attempt}")
    thing, things, measure, small, big, factor, event, events = theme
    used = set()
    regions = [n + " " + s for n, s in zip(make_names(rng, 4, used), rng.sample(["District", "Coast", "Valley", "Reach", "Basin", "Shire"], 4))]
    cats = rng.sample(["Class A", "Class B", "Class C", "Tier 1", "Tier 2", "Tier 3", "Grade I", "Grade II", "Grade III"], 9)
    cats = sorted([c for c in cats if c.split()[0] == cats[0].split()[0]])
    names = make_names(rng, N_THINGS, used)
    prefix = "".join(x[0] for x in thing.split()).upper() + rng.choice("KQXZ")
    codes = rng.sample(range(100, 999), N_THINGS)
    things_ = []
    for name, code in zip(names, codes):
        things_.append({"id": f"{prefix}-{code}", "name": name, "region": rng.choice(regions), "category": rng.choice(cats)})
    things_.sort(key=lambda t: t["id"])
    # measurements, in whole big units so every conversion is exact
    for t in things_:
        t["first"] = rng.randrange(20, 400) * factor
        t["in_big"] = rng.random() < 0.35
        t["value"] = t["first"]
    corrected = rng.sample(things_, 11)
    for t in corrected:
        shift = rng.choice([-1, 1]) * rng.randrange(15, 160) * factor
        t["value"] = max(5 * factor, t["first"] + shift)
        t["corr_in_big"] = rng.random() < 0.5
    # the busy ones attract most log entries; some of them are withdrawn or renamed so the rules matter
    busy = rng.sample(things_, 16)
    quiet = [t for t in things_ if t not in busy]
    withdrawn = rng.sample(busy, 3) + rng.sample(quiet, 3)
    for t in things_:
        t["withdrawn"] = t in withdrawn
    renamed = rng.sample(busy, 6) + rng.sample(quiet, 2)
    former = dict(zip([t["id"] for t in renamed], make_names(rng, 8, used)))
    # events
    start = datetime.date(2025, 1, 1)
    evs = []
    for n in range(1, N_EVENTS + 1):
        t = rng.choice(busy) if rng.random() < 0.6 else rng.choice(things_)
        day = start + datetime.timedelta(days=rng.randrange(0, 365))
        how = rng.random()
        ref = t["id"] if how < 0.4 else (former[t["id"]] if t["id"] in former and how < 0.75 else t["name"])
        evs.append({"n": n, "date": day.isoformat(), "ref": ref, "thing": t["id"], "void": False})
    evs.sort(key=lambda e: (e["date"], e["n"]))
    for i, e in enumerate(evs, 1):
        e["n"] = i
    for e in rng.sample(evs, 34):
        e["void"] = True

    # ---- questions: choose settings so that every shortcut gives a wrong answer
    def q1_answer(region, threshold, use_corr=True, use_withdrawn=True):
        return sorted(t["name"] for t in things_ if t["region"] == region
                      and (t["value"] if use_corr else t["first"]) > threshold
                      and not (use_withdrawn and t["withdrawn"]))

    q1 = None
    for _ in range(4000):
        region = rng.choice(regions)
        threshold = rng.randrange(40, 380) * factor + factor // 2 if factor > 1 else rng.randrange(40, 380)
        if any(t["value"] == threshold or t["first"] == threshold for t in things_):
            continue
        key = q1_answer(region, threshold)
        wrong_unit = sorted(t["name"] for t in things_ if t["region"] == region and not t["withdrawn"]
                            and (t["value"] // factor if t.get("corr_in_big", t["in_big"]) else t["value"]) > threshold)
        if 3 <= len(key) <= 6 and key != q1_answer(region, threshold, use_corr=False) \
                and key != q1_answer(region, threshold, use_withdrawn=False) and key != wrong_unit:
            q1 = {"region": region, "threshold": threshold, "key": key}
            break
    if not q1:
        return None

    by_ref = {t["name"]: t["id"] for t in things_}
    by_ref.update({t["id"]: t["id"] for t in things_})

    def q2_answer(cat, k, d1, d2, use_alias=True, use_void=True, use_withdrawn=True):
        counts = {}
        for e in evs:
            if not (d1 <= e["date"] <= d2) or (use_void and e["void"]):
                continue
            tid = e["thing"] if use_alias else by_ref.get(e["ref"])
            if tid:
                counts[tid] = counts.get(tid, 0) + 1
        return sorted(t["name"] for t in things_ if t["category"] == cat and counts.get(t["id"], 0) >= k
                      and not (use_withdrawn and t["withdrawn"]))

    q2 = None
    for _ in range(3000):
        cat = rng.choice(cats)
        m1 = rng.randrange(1, 8)
        d1 = datetime.date(2025, m1, rng.choice([1, 10, 15])).isoformat()
        d2 = (datetime.date(2025, m1 + rng.randrange(3, 6), 1) - datetime.timedelta(days=rng.choice([1, 12]))).isoformat()
        k = rng.choice([2, 3, 4, 5])
        key = q2_answer(cat, k, d1, d2)
        if 2 <= len(key) <= 6 and key != q2_answer(cat, k, d1, d2, use_alias=False) \
                and key != q2_answer(cat, k, d1, d2, use_void=False) and key != q2_answer(cat, k, d1, d2, use_withdrawn=False):
            q2 = {"category": cat, "at_least": k, "from": d1, "to": d2, "key": key}
            break
    if not q2:
        return None

    # ---- documents
    docs = {}
    docs["01-register.md"] = (
        f"# Register of {things}\n\nIssued 3 February 2026. One row per {thing}. The code never changes; names can.\n\n"
        + table(["Code", "Name", "Region", "Category"], [[t["id"], t["name"], t["region"], t["category"]] for t in things_]))
    rows = []
    for t in things_:
        rows.append([t["id"], f"{t['first'] // factor} †" if t["in_big"] else t["first"]])
    docs["02-annual-return.md"] = (
        f"# Annual return 2025: {measure}\n\nPublished 20 January 2026. Figures are listed by register code. "
        f"See the units note before using them.\n\n" + table(["Code", measure.capitalize()], rows))
    docs["03-units-note.md"] = (
        f"# Units note\n\nFigures for {measure} are in {small} unless the figure carries the mark †. "
        f"A marked figure is in {big}. One of the {big} is {factor} {small}. "
        f"The same rule applies in the annual return and in any later correction notice.\n")
    crow = []
    for t in sorted(corrected, key=lambda t: t["name"]):
        crow.append([t["name"], f"{t['value'] // factor} †" if t["corr_in_big"] else t["value"]])
    docs["04-correction-notice.md"] = (
        f"# Correction notice to the annual return 2025\n\nPublished 2 March 2026. The figures below replace the ones in the annual return "
        f"for the {things} named. All other figures in the annual return stand.\n\n" + table(["Name", "Corrected " + measure], crow))
    draft = rng.sample(things_, 12)
    docs["05-draft-return.md"] = (
        f"# Draft return 2025 (working copy)\n\nCirculated 9 December 2025 for comment. This draft was replaced by the annual return "
        f"and must not be quoted.\n\n"
        + table(["Code", measure.capitalize() + f" ({small})"], [[t["id"], t["first"] + rng.choice([-3, 2, 7]) * factor] for t in sorted(draft, key=lambda t: t["id"])]))
    docs["06-withdrawals.md"] = (
        f"# Withdrawals memo\n\nDated 11 March 2026. The following {things} were withdrawn from the programme and are left out of every count, "
        f"list and total for 2025, whatever their figures say.\n\n" + "".join(f"- {t['name']}\n" for t in sorted(withdrawn, key=lambda t: t["name"])))
    docs["07-former-names.md"] = (
        f"# Former names\n\nSome {things} were renamed during 2025. Older paperwork, including parts of the log, still uses the former name.\n\n"
        + table(["Former name", "Code"], sorted([[former[i], i] for i in former])))
    docs["08-log.md"] = (
        f"# Log of {events}, 2025\n\nOne row per {event}. The reference is whatever the reporting officer wrote: a register code, "
        f"a current name or a former name.\n\n" + table(["Entry", "Date", "Reference"], [[e["n"], e["date"], e["ref"]] for e in evs]))
    voids = sorted(e["n"] for e in evs if e["void"])
    docs["09-void-entries.md"] = (
        f"# Void log entries\n\nDated 14 January 2026. After review the log entries numbered below were made in error. "
        f"They are void and do not count as {events}.\n\n" + ", ".join(str(v) for v in voids) + "\n")

    questions = [
        {"kind": "threshold",
         "text": f"Which {things} in {q1['region']} had {measure} for 2025 above {q1['threshold']} {small}, on the final figures, "
                 f"leaving out any that were withdrawn?",
         "params": {"region": q1["region"], "threshold": q1["threshold"]}, "key": q1["key"]},
        {"kind": "count",
         "text": f"Which {q2['category']} {things} had at least {q2['at_least']} {events} from {q2['from']} to {q2['to']}, both days included, "
                 f"counting only entries that are not void and leaving out any {things} that were withdrawn?",
         "params": {k: q2[k] for k in ("category", "at_least", "from", "to")}, "key": q2["key"]},
    ]
    return docs, questions


def sha(data):
    return hashlib.sha256(data).hexdigest()


def main():
    if OUT.exists():
        shutil.rmtree(OUT)
    index = []
    n = 0
    for w, theme in enumerate(THEMES, 1):
        wid = f"w-{w:02d}"
        # a draw where some rule would not matter to the answer is thrown away and redrawn
        built, attempt = None, 0
        while built is None:
            attempt += 1
            built = build_world(w, theme, attempt)
        docs, questions = built
        wdir = OUT / "worlds" / wid
        wdir.mkdir(parents=True)
        doc_hashes = {}
        for name, text in docs.items():
            (wdir / name).write_text(text)
            doc_hashes[name] = sha(text.encode())
        for q in questions:
            n += 1
            tid = f"research-{n:02d}"
            tdir = OUT / tid
            tdir.mkdir()
            question = (
                f"# Question\n\n{q['text']}\n\n"
                "Use only the documents in this folder. Put the answer under a heading `## Answer`: the current names "
                "as written in the register, one per line, with nothing else under that heading. "
                "Say how you reached it under a second heading.\n")
            key = json.dumps({"names": q["key"]}, indent=1) + "\n"
            (tdir / "question.md").write_text(question)
            (tdir / "key.json").write_text(key)
            task = {"id": tid, "family": "research", "world": wid, "kind": q["kind"], "params": q["params"],
                    "documents": doc_hashes, "question_sha256": sha(question.encode()), "key_sha256": sha(key.encode()),
                    "scoring": "The answer is the set of names given. Pass means the set equals the key exactly. "
                               "Share of key names found and share of given names that are right are also recorded."}
            (tdir / "task.json").write_text(json.dumps(task, indent=1) + "\n")
            index.append({"id": tid, "world": wid, "kind": q["kind"], "thing": theme[1], "key_size": len(q["key"])})
    (OUT / "index.json").write_text(json.dumps(index, indent=1) + "\n")
    print(f"wrote {n} research tasks in {len(THEMES)} worlds")


if __name__ == "__main__":
    main()
