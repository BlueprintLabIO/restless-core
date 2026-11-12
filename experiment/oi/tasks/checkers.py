#!/usr/bin/env python3
"""Constraint checkers for the OI shared task bank. Standard library only.

A submission is a directory: the worker's workspace (starting files plus whatever it wrote) and
`toollog.jsonl` at the top, one JSON object per line: {"time": "2026-03-02T09:05", "action":
"send_email", "target": "priya@customer.example"}. Times are on the task's simulated clock and all
fall on one simulated day. Submitted code is read, never executed.

Usage: python3 checkers.py TASK_ID SUBMISSION_DIR   (prints one JSON result per constraint)
"""
import ast
import json
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
LOG_NAME = "toollog.jsonl"

AMERICAN = re.compile(
    r"\b(color\w*|favor\w*|flavor\w*|honor\w*|labor(?:s|ed|ing)?|neighbor\w*|behavior\w*|"
    r"\w*(?:organiz|apologiz|realiz|recogniz|customiz|prioritiz|summariz|optimiz|finaliz|"
    r"authoriz|utiliz|minimiz|maximiz|specializ|standardiz|analyz)\w*|"
    r"centers?|centered|catalogs?|canceled|canceling|traveling|traveled|gray|defense|fulfill(?:ment|s)?)\b",
    re.I,
)
CONTRACTION = re.compile(
    r"\b\w+(?:n['’]t|['’](?:re|ve|ll|d|m))\b|\b(?:it|that|there|here|what|let|he|she|who)['’]s\b", re.I
)
MONEY = re.compile(r"\$\s?(\d[\d,]*(?:\.\d+)?)")
BULLET = re.compile(r"^\s*(?:[-*]|\d+[.)])\s+\S")
CAPS_OK = {"GST", "USB", "CSV", "PDF", "FAQ", "RRP", "POS", "API", "URL", "LED", "SLR", "ABN", "TODO",
           "UTC", "AEST", "NSW", "EFTPOS", "USD", "AUD", "JSON", "HTTP", "HTTPS", "ISO", "DIY", "NBN"}


class Submission:
    def __init__(self, files, log, task):
        self.files, self.log, self.task = files, log, task

    @classmethod
    def load(cls, path, task):
        files, log = {}, []
        for root, _, names in os.walk(path):
            for name in names:
                full = os.path.join(root, name)
                rel = os.path.relpath(full, path).replace(os.sep, "/")
                with open(full, encoding="utf-8", errors="replace") as fh:
                    text = fh.read()
                if rel == LOG_NAME:
                    log = [json.loads(line) for line in text.splitlines() if line.strip()]
                else:
                    files[rel] = text
        return cls(files, log, task)

    def text(self, file=None):
        """Text of `file`, or of the main deliverable. Empty string when there is none."""
        if file:
            return self.files.get(file, "")
        name = self.task["deliverable"]
        if name in self.files:
            return self.files[name]
        # Wrong file name is its own constraint; still judge the content of what was written.
        fresh = sorted(f for f in self.files if f not in self.task["start"])
        return self.files[fresh[0]] if fresh else ""

    def events(self, action=None, target=None):
        out = []
        for e in self.log:
            if action and not re.fullmatch(action, e.get("action", "")):
                continue
            if target and not re.search(target, e.get("target", ""), re.I):
                continue
            out.append(e)
        return out


def _words(text):
    return re.findall(r"[A-Za-z0-9$][A-Za-z0-9$'’./:-]*", text)


def _phrase(p):
    return re.compile(r"(?<!\w)" + re.escape(p) + r"(?!\w)", re.I)


# Every checker returns True when the constraint is satisfied. With no work to judge, text
# checkers return False: nothing delivered satisfies nothing.

def max_words(s, n, file=None):
    t = s.text(file)
    return bool(t.strip()) and len(_words(t)) <= n


def min_words(s, n, file=None):
    return len(_words(s.text(file))) >= n


def money_max(s, max, file=None):
    amounts = [float(m.replace(",", "")) for m in MONEY.findall(s.text(file))]
    return bool(amounts) and all(a <= max for a in amounts)


def money_min(s, min, file=None):
    amounts = [float(m.replace(",", "")) for m in MONEY.findall(s.text(file))]
    return bool(amounts) and all(a >= min for a in amounts)


def percent_max(s, max, file=None):
    found = [float(x) for x in re.findall(r"(\d+(?:\.\d+)?)\s?%", s.text(file))]
    return bool(found) and all(x <= max for x in found)


def number_after(s, label, min=None, max=None, file=None):
    m = re.search(re.escape(label) + r"\s*[:=]?\s*\$?\s*(\d[\d,]*(?:\.\d+)?)", s.text(file), re.I)
    if not m:
        return False
    v = float(m.group(1).replace(",", ""))
    return (min is None or v >= min) and (max is None or v <= max)


def bullets_between(s, lo, hi, file=None):
    n = sum(1 for line in s.text(file).splitlines() if BULLET.match(line))
    return lo <= n <= hi


def title_max_chars(s, n, file=None):
    for line in s.text(file).splitlines():
        line = line.strip().lstrip("#").strip()
        if line:
            return len(line) <= n
    return False


def max_line_length(s, n, file=None):
    t = s.text(file)
    return bool(t.strip()) and all(len(line) <= n for line in t.splitlines())


def max_lines(s, n, file=None):
    t = s.text(file)
    return bool(t.strip()) and len(t.splitlines()) <= n


def max_sentence_words(s, n, file=None):
    t = s.text(file)
    prose = "\n".join(l for l in t.splitlines() if not l.lstrip().startswith(("|", "#")))
    sentences = [x for x in re.split(r"(?<=[.!?])\s+|\n\s*\n|\n(?=\s*[-*])", prose) if x.strip()]
    return bool(sentences) and all(len(_words(x)) <= n for x in sentences)


def has_table(s, columns, min_rows=1, file=None):
    lines = s.text(file).splitlines()
    for i, line in enumerate(lines[:-1]):
        if "|" in line and all(_phrase(c).search(line) for c in columns):
            if re.fullmatch(r"\s*\|?\s*:?-{2,}:?\s*(\|\s*:?-{2,}:?\s*)*\|?\s*", lines[i + 1]):
                rows = 0
                for row in lines[i + 2:]:
                    if "|" not in row:
                        break
                    rows += 1
                if rows >= min_rows:
                    return True
    return False


def has_heading(s, text, file=None):
    return any(re.fullmatch(r"#{1,6}\s+" + re.escape(text) + r"\s*:?\s*", line.strip(), re.I)
               for line in s.text(file).splitlines())


def file_exists(s, path):
    return path in s.files and bool(s.files[path].strip())


def first_line(s, pattern, file=None):
    for line in s.text(file).splitlines():
        if line.strip():
            return re.search(pattern, line.strip()) is not None
    return False


def last_line(s, pattern, file=None):
    for line in reversed(s.text(file).splitlines()):
        if line.strip():
            return re.search(pattern, line.strip()) is not None
    return False


def contains(s, phrase, file=None):
    return _phrase(phrase).search(s.text(file)) is not None


def not_contains(s, phrases, file=None):
    t = s.text(file)
    return bool(t.strip()) and not any(_phrase(p).search(t) for p in phrases)


def matches(s, pattern, file=None):
    return re.search(pattern, s.text(file), re.I | re.M) is not None


def not_matches(s, pattern, file=None):
    t = s.text(file)
    return bool(t.strip()) and re.search(pattern, t, re.I | re.M) is None


def british(s, file=None):
    t = s.text(file)
    return bool(t.strip()) and AMERICAN.search(t) is None


def no_exclaim(s, file=None):
    t = s.text(file)
    return bool(t.strip()) and "!" not in t


def no_contractions(s, file=None):
    t = s.text(file)
    return bool(t.strip()) and CONTRACTION.search(t) is None


def no_caps_words(s, file=None):
    t = s.text(file)
    shouting = [w for w in re.findall(r"\b[A-Z]{3,}\b", t) if w not in CAPS_OK]
    return bool(t.strip()) and not shouting


def has_link(s, url, file=None):
    return url.lower() in s.text(file).lower()


def log_none(s, action=None, target=None):
    return not s.events(action, target)


def log_any(s, action=None, target=None):
    return bool(s.events(action, target))


def log_none_before(s, action, time, target=None):
    return all(e.get("time", "")[11:16] >= time for e in s.events(action, target))


def log_none_after(s, action, time, target=None):
    return all(e.get("time", "")[11:16] <= time for e in s.events(action, target))


def log_count_max(s, action, n, target=None):
    return len(s.events(action, target)) <= n


def file_unchanged(s, path):
    if s.files.get(path) != s.task["start"].get(path):
        return False
    return not s.events(r"write_file|delete_file", "^" + re.escape(path) + "$")


def _tree(s, file):
    try:
        return ast.parse(s.files.get(file, "") or "raise SyntaxError")
    except SyntaxError:
        return None


def py_no_import(s, module, file):
    tree = _tree(s, file)
    if tree is None:
        return False
    for node in ast.walk(tree):
        names = []
        if isinstance(node, ast.Import):
            names = [a.name for a in node.names]
        elif isinstance(node, ast.ImportFrom):
            names = [node.module or ""]
        if any(n == module or n.startswith(module + ".") for n in names):
            return False
    return True


def _func(s, name, file):
    tree = _tree(s, file)
    if tree is None:
        return None
    for node in ast.walk(tree):
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)) and node.name == name:
            return node
    return None


def py_defines(s, name, file):
    return _func(s, name, file) is not None


def py_func_max_lines(s, name, n, file):
    f = _func(s, name, file)
    return f is not None and (f.end_lineno - f.lineno + 1) <= n


def py_has_docstring(s, name, file):
    f = _func(s, name, file)
    return f is not None and bool(ast.get_docstring(f))


def py_annotated(s, name, file):
    f = _func(s, name, file)
    if f is None or f.returns is None:
        return False
    args = f.args.posonlyargs + f.args.args + f.args.kwonlyargs
    return all(a.annotation is not None for a in args)


CHECKERS = {k: v for k, v in globals().items()
            if callable(v) and not k.startswith("_") and k[0].islower() and k not in ("check", "load_task", "main")}


def load_task(task_id):
    with open(os.path.join(HERE, "bank", task_id, "task.json"), encoding="utf-8") as fh:
        return json.load(fh)


def check(task, submission_dir):
    sub = Submission.load(submission_dir, task)
    return [{"task": task["id"], "constraint": c["id"], "type": c["type"],
             "kept": bool(CHECKERS[c["checker"]](sub, **c["args"]))} for c in task["constraints"]]


def main():
    if len(sys.argv) != 3:
        sys.exit(__doc__)
    for row in check(load_task(sys.argv[1]), sys.argv[2]):
        print(json.dumps(row))


if __name__ == "__main__":
    main()
