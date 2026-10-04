"""Remove one launch credential without spawning a grep for every profile file."""
import os
import stat
import sys

secret = os.environb[b"RESTLESS_PURGE_SECRET"]
if not secret:
    raise RuntimeError("Refusing empty credential")

MAX_CLEAR_SIZE = 4 * 1024 * 1024
OPEN_SAFELY = os.O_CLOEXEC | os.O_NOFOLLOW
profile_root = sys.argv[1]

try:
    root_metadata = os.stat(profile_root, follow_symlinks=False)
except FileNotFoundError:
    sys.exit(0)
if not stat.S_ISDIR(root_metadata.st_mode):
    raise RuntimeError("Credential cleanup root is not a directory")


def contains(stream):
    tail = b""
    while chunk := stream.read(64 * 1024):
        data = tail + chunk
        if secret in data:
            return True
        tail = data[-(len(secret) - 1):] if len(secret) > 1 else b""
    return False


def onerror(error):
    raise error


def open_regular(path, writable):
    before = os.stat(path, follow_symlinks=False)
    if not stat.S_ISREG(before.st_mode):
        return None
    descriptor = os.open(path, (os.O_RDWR if writable else os.O_RDONLY) | OPEN_SAFELY)
    opened = os.fstat(descriptor)
    if not stat.S_ISREG(opened.st_mode) or (
        opened.st_dev,
        opened.st_ino,
    ) != (before.st_dev, before.st_ino):
        os.close(descriptor)
        raise RuntimeError("Profile file changed during credential cleanup")
    return descriptor


def profile_files():
    for directory, _, filenames in os.walk(
        profile_root, followlinks=False, onerror=onerror
    ):
        for filename in filenames:
            yield os.path.join(directory, filename)


for path in profile_files():
    descriptor = open_regular(path, writable=False)
    if descriptor is None:
        continue
    with os.fdopen(descriptor, "rb") as stream:
        found = contains(stream)
        scanned = os.fstat(stream.fileno())
    if not found:
        continue

    descriptor = open_regular(path, writable=True)
    if descriptor is None:
        raise RuntimeError("Profile file changed during credential cleanup")
    with os.fdopen(descriptor, "r+b") as stream:
        opened = os.fstat(stream.fileno())
        if (opened.st_dev, opened.st_ino) != (scanned.st_dev, scanned.st_ino):
            raise RuntimeError("Profile file changed during credential cleanup")
        if not contains(stream):
            raise RuntimeError("Profile file changed during credential cleanup")
        if os.fstat(stream.fileno()).st_size >= MAX_CLEAR_SIZE:
            raise RuntimeError("Large profile file retained a scoped credential")
        stream.seek(0)
        stream.truncate(0)
        stream.flush()
        after = os.stat(path, follow_symlinks=False)
        opened = os.fstat(stream.fileno())
        if (after.st_dev, after.st_ino) != (opened.st_dev, opened.st_ino):
            raise RuntimeError("Profile file changed during credential cleanup")


# A separate no-follow pass catches replacements and files created during cleanup.
for path in profile_files():
    descriptor = open_regular(path, writable=False)
    if descriptor is None:
        continue
    with os.fdopen(descriptor, "rb") as stream:
        found = contains(stream)
        after = os.stat(path, follow_symlinks=False)
        opened = os.fstat(stream.fileno())
        if (after.st_dev, after.st_ino) != (opened.st_dev, opened.st_ino):
            raise RuntimeError("Profile file changed during credential cleanup")
        if found:
            raise RuntimeError("Profile retained a scoped credential after cleanup")
