#!/usr/bin/env python3
"""Exercise Cargo's actual rebuild decisions in a tiny isolated Git project."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

BUILD_SCRIPT = Path(__file__).resolve().parents[1] / 'crates/restlessd/build.rs'

class ReleaseRevisionTest(unittest.TestCase):
    def test_commit_dirty_worktree_and_exported_override(self):
        with tempfile.TemporaryDirectory(prefix='restless-revision-test-') as directory:
            root = Path(directory) / 'repo'
            root.mkdir()
            env = dict(os.environ)
            env.pop('RESTLESS_SOURCE_REVISION', None)
            env['CARGO_TARGET_DIR'] = str(Path(directory) / 'target')
            def run(*args, cwd=root, override=None):
                result = subprocess.run(args, cwd=cwd, env=env | (override or {}), capture_output=True, text=True, check=True)
                return result.stdout.strip()
            run('git', 'init', '--quiet')
            run('git', 'config', 'user.name', 'Release test')
            run('git', 'config', 'user.email', 'release-test@restless.test')
            (root / 'Cargo.toml').write_text('[workspace]\nmembers=["crates/probe"]\nresolver="2"\n')
            package = root / 'crates/probe'
            (package / 'src').mkdir(parents=True)
            (package / 'Cargo.toml').write_text('[package]\nname="revision-probe"\nversion="0.0.0"\nedition="2021"\n')
            (package / 'src/main.rs').write_text('fn main() { println!("{}", env!("RESTLESS_SOURCE_REVISION")); }\n')
            shutil.copyfile(BUILD_SCRIPT, package / 'build.rs')
            run('cargo', 'generate-lockfile', '--offline')
            run('git', 'add', '.')
            run('git', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '-m', 'First revision')
            first = run('git', 'rev-parse', 'HEAD')
            self.assertEqual(run('cargo', 'run', '--quiet', '--offline'), first)
            (root / 'README.md').write_text('A new release.\n')
            run('git', 'add', 'README.md')
            run('git', '-c', 'commit.gpgsign=false', 'commit', '--quiet', '-m', 'Next revision')
            second = run('git', 'rev-parse', 'HEAD')
            self.assertNotEqual(first, second)
            self.assertEqual(run('cargo', 'run', '--quiet', '--offline'), second)
            (root / 'README.md').write_text('Uncommitted release edit.\n')
            self.assertEqual(run('cargo', 'run', '--quiet', '--offline'), second + '-dirty')
            run('git', 'restore', 'README.md')
            self.assertEqual(run('cargo', 'run', '--quiet', '--offline'), second)
            run('git', 'pack-refs', '--all')
            self.assertEqual(run('cargo', 'run', '--quiet', '--offline'), second)
            linked = Path(directory) / 'linked'
            run('git', 'worktree', 'add', '--quiet', '--detach', str(linked), second)
            self.assertEqual(run('cargo', 'run', '--quiet', '--offline', cwd=linked), second)
            self.assertEqual(run('cargo', 'run', '--quiet', '--offline', cwd=linked, override={'RESTLESS_SOURCE_REVISION': 'exported-release'}), 'exported-release')

if __name__ == '__main__':
    unittest.main()
