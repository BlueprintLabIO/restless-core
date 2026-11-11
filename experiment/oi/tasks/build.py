#!/usr/bin/env python3
"""Build the frozen task bank in bank/ from the authoring files in src/. Standard library only.

    python3 build.py          write bank/ and MANIFEST.sha256
    python3 build.py --check  rebuild in memory and fail if bank/ or the manifest differs

Each task is written in 4 versions. Across the 4, every constraint appears once in each of:
edge + important, edge + aside, middle + important, middle + aside. "Edge" is the start or the end
of the message; "middle" sits between background sentences. The order inside a block and the
wording frame are fixed by a hash of the task id, version and constraint id, so the build is
repeatable on any machine.
"""
import hashlib
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
sys.path.insert(0, os.path.join(HERE, "src"))

KINDS = ["code", "research", "resale", "email"]
TYPES = ["number", "format", "phrase", "style", "action", "include"]
IMPORTANT = ["Important: {t}.", "This one is a must: {t}.", "Whatever else happens, {t}.",
             "Non-negotiable: {t}."]
ASIDE = ["By the way, {t}.", "Oh, and {t}.", "Small thing: {t}.", "While you're at it, {t}."]
CELLS = [("edge", "important"), ("edge", "aside"), ("middle", "important"), ("middle", "aside")]


def _h(*parts):
    return int(hashlib.sha256(":".join(map(str, parts)).encode()).hexdigest(), 16)


def load_tasks():
    tasks = []
    for kind in KINDS:
        mod = __import__(kind)
        for n, raw in enumerate(mod.TASKS, 1):
            t = dict(raw)
            t["id"] = f"{kind}-{n:02d}"
            t["kind"] = kind
            t["constraints"] = [
                {"id": f"c{i:02d}", "type": c[0], "text": c[1], "checker": c[2], "args": c[3]}
                for i, c in enumerate(raw["constraints"], 1)]
            t["start"] = dict(raw.get("start", {}))
            for side in ("good", "bad"):
                files, log = raw[side]
                t[side] = {"files": files, "log": [
                    {"time": f"{t['clock'][:10]}T{e.split()[0]}", "action": e.split()[1],
                     "target": e.split(None, 2)[2]} for e in log]}
            tasks.append(t)
    return tasks


def render(task, v):
    """Return (message text, layout rows) for version v (1 to 4)."""
    placed = {"edge": [], "middle": []}
    meta = {}
    for i, c in enumerate(task["constraints"]):
        where, tone = CELLS[(i + v - 1) % 4]
        frames = IMPORTANT if tone == "important" else ASIDE
        frame = frames[_h(task["id"], v, c["id"], "frame") % 4]
        sentence = frame.format(t=c["text"])
        sentence = sentence[0].upper() + sentence[1:]
        placed[where].append((_h(task["id"], v, c["id"], "order"), c["id"], sentence))
        meta[c["id"]] = {"phrasing": tone, "frame": frame}
    edge = sorted(placed["edge"])
    middle = sorted(placed["middle"])
    start, end = edge[:(len(edge) + 1) // 2], edge[(len(edge) + 1) // 2:]
    mid_a, mid_b = middle[:(len(middle) + 1) // 2], middle[(len(middle) + 1) // 2:]
    bg = task["background"]
    join = lambda rows: " ".join(r[2] for r in rows)
    paras = [task["ask"], join(start),
             " ".join(x for x in [bg[0], join(mid_a), bg[1], join(mid_b), bg[2]] if x),
             join(end), task["signoff"]]
    layout = []
    for block, rows in (("start", start), ("middle", mid_a + mid_b), ("end", end)):
        for r in rows:
            layout.append({"constraint": r[1], "block": block, "position": len(layout) + 1, **meta[r[1]]})
    return "\n\n".join(p for p in paras if p) + "\n", layout


def build():
    """Return {relative path: bytes} for the whole frozen bank."""
    out = {}
    index = []
    for t in load_tasks():
        base = f"bank/{t['id']}/"
        task = {k: t[k] for k in ("id", "kind", "title", "clock", "deliverable", "ask", "background",
                                  "signoff", "constraints", "start")}
        out[base + "task.json"] = json.dumps(task, indent=1, ensure_ascii=False) + "\n"
        versions = {}
        for v in range(1, 5):
            text, layout = render(t, v)
            out[base + f"v{v}.md"] = text
            versions[f"v{v}"] = layout
        out[base + "versions.json"] = json.dumps(versions, indent=1) + "\n"
        out[base + "fixtures.json"] = json.dumps(
            {"good": t["good"], "bad": t["bad"]}, indent=1, ensure_ascii=False) + "\n"
        index.append({"id": t["id"], "kind": t["kind"], "title": t["title"]})
    out["bank/index.json"] = json.dumps(index, indent=1) + "\n"
    out = {k: v.encode("utf-8") for k, v in out.items()}
    for frozen in ("checkers.py",):
        with open(os.path.join(HERE, frozen), "rb") as fh:
            out[frozen] = fh.read()
    manifest = "".join(f"{hashlib.sha256(out[k]).hexdigest()}  {k}\n" for k in sorted(out))
    del out["checkers.py"]
    out["MANIFEST.sha256"] = manifest.encode()
    return out


def main():
    out = build()
    if "--check" in sys.argv:
        bad = []
        for rel, data in out.items():
            path = os.path.join(HERE, rel)
            if not os.path.exists(path) or open(path, "rb").read() != data:
                bad.append(rel)
        on_disk = {os.path.relpath(os.path.join(r, n), HERE).replace(os.sep, "/")
                   for r, _, ns in os.walk(os.path.join(HERE, "bank")) for n in ns}
        bad += sorted(on_disk - set(out))
        if bad:
            sys.exit("bank/ is not what src/ builds: " + ", ".join(bad[:10]))
        print(f"bank matches src: {len(out)} files")
        return
    for rel, data in out.items():
        path = os.path.join(HERE, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "wb") as fh:
            fh.write(data)
    print(f"wrote {len(out)} files")


if __name__ == "__main__":
    main()
