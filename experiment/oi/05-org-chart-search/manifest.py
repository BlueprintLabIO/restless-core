#!/usr/bin/env python3
"""Write MANIFEST.sha256: one line per frozen file (everything under tasks/, plus suite.json).

split.json is left out on purpose: it is a draft that will be remade after calibration.

Usage: python3 manifest.py
"""
import hashlib
import pathlib

SUITE = pathlib.Path(__file__).resolve().parent
files = sorted(p for p in (SUITE / "tasks").rglob("*") if p.is_file()) + [SUITE / "suite.json"]
lines = [f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.relative_to(SUITE).as_posix()}" for p in files]
(SUITE / "MANIFEST.sha256").write_text("\n".join(lines) + "\n")
print(f"MANIFEST.sha256: {len(lines)} files")
