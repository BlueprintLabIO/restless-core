# Company Runtime startup and browser recovery

The public `company-init` works locally and under a hosted provider. Before any
company initialization or profile modification, it takes a nonblocking Linux
`flock` on an open descriptor for the mounted `/company` **directory inode**.
There is no lock file an ordinary company process could unlink and recreate to
bypass the cooperating-init check. Unsupported filesystem locking fails closed.

FD 9 stays open across `exec` into `tini`. Tini's child explicitly closes its copy
before starting Supervisor, so company services do not inherit an unlockable copy
of the same open-file description. The lock lasts for the Core init lifetime; it
is not a lease service, runtime-generation authority or fencing for unrelated
processes. See the [Linux flock contract](https://man7.org/linux/man-pages/man2/flock.2.html).

Under that lock, `recover-browser-profile.py` removes only Chromium's transient
`SingletonLock`, `SingletonSocket` and `SingletonCookie` symlinks. Chromium can
leave these behind even after a successful SIGTERM shutdown; a different
container hostname then causes its profile-in-use refusal. Cookies, tab
checkpoints, preferences and all other profile content are retained. The helper
does not follow the profile directory or singleton links and refuses unexpected
non-symlink entries without deleting any of the three entries.

## Required provider fencing

This guard coordinates **cooperating new inits sharing one actual filesystem**.
It cannot fence an older release without the guard, an independent restored copy,
an uncooperative process, or an old host isolated by a network partition. A
provider must stop/drain and verify the previous Runtime is gone before granting
the replacement access. File locking is not permission to start a second live
company or to resume a stale backup. The first upgrade from a pre-guard release
therefore needs the same verified stop/remove boundary as other replacements.

NFS/CIFS/FUSE and multi-node storage need their own locking and fencing
qualification; the existence of a `ReadWriteOnce` claim is not proof. Neither
successful profile recovery nor filesystem integrity establishes a zero-data-loss
recovery point for interrupted Work or unflushed browser writes.

## Tests and qualification

Run `python3 -B -m unittest discover -s infra/company-image -p 'test_*.py' -v`.
The release workflow runs these checks. Test startup under the actual image too:
verify the FD in tini, refusal of a second init before it touches the profile,
and real CDP tabs/cookies after graceful and forced replacement. Check OOM
counters so a too-small test allocation is not mistaken for a recovery failure.

`qualification.Dockerfile` is a **test-only overlay** for checking the startup
source against an explicitly pinned existing Core image without rebuilding
unchanged binaries. Its label identifies it as experimental. Build-baked release
identity still names the base release; this image is not a new Core release and
must never be substituted into Cloud's release lock. Record the candidate image
identity and startup-file hashes in experiment evidence. Ship through the full
Core Dockerfile and immutable release pipeline before updating any Cloud pin.
