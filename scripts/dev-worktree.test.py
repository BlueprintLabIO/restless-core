#!/usr/bin/env python3
"""Exercise the worktree pool against a throwaway origin and clone.

The invariant: a slot holding someone's work (uncommitted, unpushed, or claimed
and not yet landed) is never reused, released or removed; a slot whose work
has landed is released and reused warm, at the same path.
"""
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
LIB = ROOT / 'scripts/lib/dev-worktree.sh'


def run(*args, cwd=None, env=None):
    return subprocess.run(args, cwd=cwd, env=env, check=True, text=True, capture_output=True).stdout


with tempfile.TemporaryDirectory(prefix='restless-worktree-pool-') as directory:
    base = Path(directory)
    origin, repo, work = base / 'origin.git', base / 'repo', base / 'work'
    run('git', 'init', '--quiet', '--bare', '--initial-branch=main', str(origin))
    run('git', 'clone', '--quiet', str(origin), str(repo))
    for key, value in (('user.name', 'Pool test'), ('user.email', 'pool@restless.test'), ('commit.gpgsign', 'false')):
        run('git', 'config', key, value, cwd=repo)
    (repo / 'README.md').write_text('pool\n')
    # The repository's own rules: a slot's build targets are not its work.
    (repo / '.gitignore').write_text('/target\n/target-*/\n')
    run('git', 'add', '.', cwd=repo)
    run('git', 'commit', '--quiet', '-m', 'start', cwd=repo)
    run('git', 'push', '--quiet', 'origin', 'HEAD:main', cwd=repo)
    env = dict(os.environ, RESTLESS_WORKTREE_ROOT=str(work), RESTLESS_WORKTREE_POOL_SIZE='2')

    def pool(*args):
        return run('bash', '-c', 'STACK_REPO_ROOT="$1"; source "$2"; shift 2; "$@"', 'pool',
                   str(repo), str(LIB), *args, cwd=base, env=env)

    def land(slot, name):
        (slot / f'{name}.txt').write_text(name + '\n')
        run('git', 'add', '.', cwd=slot)
        run('git', 'commit', '--quiet', '-m', name, cwd=slot)
        run('git', 'pull', '--quiet', '--rebase', 'origin', 'main', cwd=slot)
        run('git', 'push', '--quiet', 'origin', 'HEAD:main', cwd=slot)

    first = pool('dev_worktree_new', 'alpha')
    slot1 = work / 'slot-1'
    assert str(slot1) in first and 'new slot' in first, first
    assert (slot1 / 'target' / '.metadata_never_index').exists(), 'slots are excluded from Spotlight'
    marker = slot1 / 'target-dev' / 'warm'
    marker.parent.mkdir(parents=True)
    marker.write_text('incremental cache\n')

    # A fresh, clean, claimed slot is someone's: a second task gets a new slot.
    second = pool('dev_worktree_new', 'beta')
    assert str(work / 'slot-2') in second, second
    out = pool('dev_worktree_prune')
    assert 'KEEP     ' + str(slot1) + ' (claimed by alpha; nothing landed yet)' in out, out

    # Uncommitted and unpushed work are kept too.
    (work / 'slot-2' / 'draft.txt').write_text('draft\n')
    run('git', 'add', 'draft.txt', cwd=work / 'slot-2')
    assert 'KEEP     ' + str(work / 'slot-2') + ' (uncommitted changes)' in pool('dev_worktree_prune')
    run('git', 'commit', '--quiet', '-m', 'draft', cwd=work / 'slot-2')
    assert '1 commit(s) on no remote' in pool('dev_worktree_prune')

    # Old incremental caches pile up per crate variant; releasing keeps the three
    # newest per crate and touches nothing outside the target directories.
    incremental = slot1 / 'target' / 'debug' / 'incremental'
    for age, name in enumerate(['engine-e', 'engine-d', 'engine-c', 'engine-b', 'engine-a', 'owner-x']):
        cache = incremental / name
        cache.mkdir(parents=True)
        (cache / 'query-cache.bin').write_text('cache\n')
        stamp = 1_700_000_000 - age * 60
        os.utime(cache, (stamp, stamp))
    keepsake = slot1 / 'notes-engine-a.txt'
    keepsake.write_text('not a cache\n')
    (slot1 / '.gitignore').write_text('/target-*/\n/target/\nnotes-*\n')

    # Once alpha lands, prune releases slot-1 and the next task reuses it warm.
    land(slot1, 'alpha')
    out = pool('dev_worktree_prune')
    assert 'RELEASED ' + str(slot1) + ' (alpha landed' in out, out
    assert sorted(p.name for p in incremental.iterdir()) == ['engine-c', 'engine-d', 'engine-e', 'owner-x'], \
        sorted(p.name for p in incremental.iterdir())
    assert keepsake.exists(), 'only cache directories are trimmed'
    third = pool('dev_worktree_new', 'gamma')
    assert str(slot1) in third and 'reused' in third, third
    assert marker.read_text() == 'incremental cache\n', 'the slot kept its build cache'
    assert run('git', 'rev-parse', '--abbrev-ref', 'HEAD', cwd=slot1).strip() == 'feat/gamma'
    assert (slot1 / 'alpha.txt').exists(), 'the reused slot starts from the latest origin/main'
    branches = run('git', 'branch', '--list', 'feat/alpha', cwd=repo)
    assert branches.strip() == '', 'the landed branch is cleaned up'

    # Beyond the pool size an idle slot is removed, not kept.
    land(work / 'slot-2', 'beta')
    land(slot1, 'gamma')
    pool('dev_worktree_new', 'delta')  # slot-1 or slot-2 is reused, no new slot
    assert not (work / 'slot-3').exists()
    env['RESTLESS_WORKTREE_POOL_SIZE'] = '1'
    out = pool('dev_worktree_prune')
    assert 'REMOVED  ' in out and out.count('KEEP') == 1, out
    print('PASS claimed, uncommitted and unpushed slots kept; landed slot released and reused warm; pool trimmed')
