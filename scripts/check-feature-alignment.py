#!/usr/bin/env python3
"""Fail when building one daemon crate alone resolves different dependency
features than the stack build (`-p restlessd -p restless`).

Different features mean a second compiled copy of the engine: ~2 minutes and
several GB the first time an agent runs, say, `cargo test -p restless-engine`
after a stack build. The fix is a feature request in restless-engine's
alignment block. Uses `cargo --unit-graph`, which is unstable, hence
RUSTC_BOOTSTRAP for this read-only query.
"""
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
STACK = ['restlessd', 'restless']
ALONE = [['restless-engine'], ['restless-owner'], ['restlessd']]


def features(packages):
    args = ['cargo', 'build', '--unit-graph', '-Z', 'unstable-options']
    for package in packages:
        args += ['-p', package]
    result = subprocess.run(args, cwd=ROOT, capture_output=True, text=True,
                            env=os.environ | {'RUSTC_BOOTSTRAP': '1'})
    if result.returncode:
        sys.exit(result.stderr[-2000:])
    resolved = {}
    for unit in json.loads(result.stdout)['units']:
        if unit['mode'] == 'build' and unit['target']['kind'] != ['custom-build']:
            resolved.setdefault(unit['pkg_id'], set()).update(unit['features'])
    return resolved


stack = features(STACK)
failed = False
for packages in ALONE:
    alone = features(packages)
    for package in sorted(set(alone) & set(stack)):
        if alone[package] != stack[package]:
            failed = True
            name = package.split('#')[-1]
            print(f"-p {' -p '.join(packages)}: {name} lacks {sorted(stack[package] - alone[package])}"
                  f" and adds {sorted(alone[package] - stack[package])}")
if failed:
    sys.exit('feature resolution differs from the stack build; extend the alignment block in crates/restless-engine/Cargo.toml')
print('PASS each daemon crate alone resolves the same dependency features as the stack build')
