#!/usr/bin/env python3
"""Recover Linux Chromium's transient links under the inherited Runtime lock.

This is not a distributed lease or authority to attach a second copy of a
company. Providers must fence old writers, including pre-guard Core releases,
before starting this image. Cookies, tabs and all other profile data are retained.
"""
import fcntl
import os
import stat
import sys

SINGLETON_LINKS = ("SingletonLock", "SingletonSocket", "SingletonCookie")


def recover_profile(company_fd):
    # Reassert on the same open-file description inherited from company-init.
    # A competing open description fails here; closing this process's copy does
    # not release the parent's lock. Never explicitly unlock this descriptor.
    fcntl.flock(company_fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
    try:
        profile_fd = os.open("browser-profile", os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW,
                             dir_fd=company_fd)
    except FileNotFoundError:
        return 0  # First boot: the entrypoint will seed the profile later.
    try:
        links = []
        # Validate every entry before removing anything. A regular file or
        # directory at these reserved names is unexpected: leave it untouched.
        for name in SINGLETON_LINKS:
            try:
                info = os.stat(name, dir_fd=profile_fd, follow_symlinks=False)
            except FileNotFoundError:
                continue
            if not stat.S_ISLNK(info.st_mode):
                raise RuntimeError("unexpected non-symlink Chromium singleton entry")
            links.append(name)
        for name in links:
            # Remove only the link, never its target. Directory-relative access
            # also prevents a profile symlink from redirecting privileged cleanup.
            os.unlink(name, dir_fd=profile_fd)
        return len(links)
    finally:
        os.close(profile_fd)


def main():
    company = os.stat("/company", follow_symlinks=False)
    inherited = os.fstat(9)
    if (not stat.S_ISDIR(company.st_mode)
            or (company.st_dev, company.st_ino) != (inherited.st_dev, inherited.st_ino)):
        raise RuntimeError("company-init must supply the mounted company directory on FD 9")
    recovered = recover_profile(9)
    if recovered:
        print(f"Recovered {recovered} Chromium singleton links under the company filesystem lock",
              flush=True)


if __name__ == "__main__":
    try:
        main()
    except (OSError, RuntimeError) as error:
        print(f"Browser profile recovery refused: {type(error).__name__}", file=sys.stderr)
        sys.exit(75)
