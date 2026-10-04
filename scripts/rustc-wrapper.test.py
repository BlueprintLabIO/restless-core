#!/usr/bin/env python3
"""Compile through scripts/rustc-wrapper and check nothing is lost on the way.

A wrapper bug once sent rustc's stderr to /dev/null: builds that succeeded
looked fine while every error and warning vanished. Require a warning and an
error to reach Cargo's output through the wrapper.
"""
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]

with tempfile.TemporaryDirectory(prefix='restless-rustc-wrapper-') as directory:
    crate = Path(directory) / 'probe'
    (crate / 'src').mkdir(parents=True)
    (crate / 'Cargo.toml').write_text('[package]\nname = "wrapper-probe"\nversion = "0.0.0"\nedition = "2021"\n')
    main = crate / 'src/main.rs'
    env = dict(os.environ, RUSTC_WRAPPER=str(ROOT / 'scripts/rustc-wrapper'),
               CARGO_TARGET_DIR=str(Path(directory) / 'target'),
               RESTLESS_RUSTC_SLOTS='2', XDG_RUNTIME_DIR=directory)
    env.pop('RESTLESS_RUSTC_DIRECT', None)

    def build(source):
        main.write_text(source)
        result = subprocess.run(['cargo', 'build', '--offline', '--message-format', 'short'],
                                cwd=crate, env=env, capture_output=True, text=True)
        return result.returncode, result.stdout + result.stderr

    # rustc skips lints once type checking fails, so check each kind alone.
    code, output = build('fn main() {\n    let unused_marker = 1;\n}\n')
    assert code == 0 and 'unused_marker' in output, f'the warning never reached Cargo:\n{output}'
    code, output = build('fn main() {\n    let value: u32 = "not a number";\n}\n')
    assert code != 0 and 'error[E0308]' in output, f'the type error never reached Cargo:\n{output}'
    print('PASS errors and warnings pass through the compiler slot wrapper')
