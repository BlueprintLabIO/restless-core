"""Remove one launch credential without spawning a grep for every profile file.

The profile root is shared: other agents' sessions keep writing, renaming and
deleting their own files while this runs. The session that held this secret
is already stopped, so nothing writes the secret anew; a file that vanishes
or is atomically replaced mid-scan belongs to someone else's churn. A vanished
path is skipped and a replaced path is read again, so cleanup only fails when
the secret really stays behind or the profile never holds still.
"""
import errno
import os
import stat
import sys

secret = os.environb[b"RESTLESS_PURGE_SECRET"]
if not secret:
    raise RuntimeError("Refusing empty credential")

MAX_CLEAR_SIZE = 4 * 1024 * 1024
OPEN_SAFELY = os.O_CLOEXEC | os.O_NOFOLLOW
PATH_RETRIES = 8
profile_root = sys.argv[1]

try:
    root_metadata = os.stat(profile_root, follow_symlinks=False)
except FileNotFoundError:
    sys.exit(0)
if not stat.S_ISDIR(root_metadata.st_mode):
    raise RuntimeError("Credential cleanup root is not a directory")


class Changed(Exception):
    """The path now names a different file than the one just examined."""


def contains(stream):
    tail = b""
    while chunk := stream.read(64 * 1024):
        data = tail + chunk
        if secret in data:
            return True
        tail = data[-(len(secret) - 1):] if len(secret) > 1 else b""
    return False


def onerror(error):
    # A directory another session removed between listing and descent is gone.
    if isinstance(error, FileNotFoundError):
        return
    raise error


def identity(metadata):
    return (metadata.st_dev, metadata.st_ino)


def open_regular(path, writable):
    before = os.stat(path, follow_symlinks=False)
    if not stat.S_ISREG(before.st_mode):
        return None
    try:
        descriptor = os.open(
            path, (os.O_RDWR if writable else os.O_RDONLY) | OPEN_SAFELY
        )
    except OSError as error:
        # O_NOFOLLOW: a symlink swapped in after the stat is a different entry.
        if error.errno == errno.ELOOP:
            raise Changed() from error
        raise
    opened = os.fstat(descriptor)
    if not stat.S_ISREG(opened.st_mode) or identity(opened) != identity(before):
        os.close(descriptor)
        raise Changed()
    return descriptor


def clear_once(path):
    descriptor = open_regular(path, writable=False)
    if descriptor is None:
        return
    with os.fdopen(descriptor, "rb") as stream:
        found = contains(stream)
        scanned = os.fstat(stream.fileno())
    if not found:
        return

    descriptor = open_regular(path, writable=True)
    if descriptor is None:
        raise Changed()
    with os.fdopen(descriptor, "r+b") as stream:
        opened = os.fstat(stream.fileno())
        if identity(opened) != identity(scanned):
            raise Changed()
        if not contains(stream):
            return
        if opened.st_size >= MAX_CLEAR_SIZE:
            raise RuntimeError("Large profile file retained a scoped credential")
        stream.seek(0)
        stream.truncate(0)
        stream.flush()
        after = os.stat(path, follow_symlinks=False)
        if identity(after) != identity(os.fstat(stream.fileno())):
            raise Changed()


def verify_once(path):
    descriptor = open_regular(path, writable=False)
    if descriptor is None:
        return
    with os.fdopen(descriptor, "rb") as stream:
        found = contains(stream)
        after = os.stat(path, follow_symlinks=False)
        if identity(after) != identity(os.fstat(stream.fileno())):
            raise Changed()
    if found:
        raise RuntimeError("Profile retained a scoped credential after cleanup")


def settle(path, step):
    for _ in range(PATH_RETRIES):
        try:
            step(path)
            return
        except FileNotFoundError:
            return
        except Changed:
            continue
    raise RuntimeError(f"Profile file kept changing during credential cleanup: {path}")


def profile_files():
    for directory, _, filenames in os.walk(
        profile_root, followlinks=False, onerror=onerror
    ):
        for filename in filenames:
            yield os.path.join(directory, filename)


for path in profile_files():
    settle(path, clear_once)

# A separate no-follow pass catches replacements and files created during cleanup.
for path in profile_files():
    settle(path, verify_once)
