#!/usr/bin/python3
"""Validate the deliberately small company-authored supervisord surface."""

from __future__ import annotations

import configparser
import os
from pathlib import Path
import re
import stat
import sys
from typing import NoReturn


PROGRAM_SECTION = re.compile(r"program:[A-Za-z0-9][A-Za-z0-9_.-]{0,127}\Z")
EXPECTED_DIRECTORY = Path("/company/services/supervisor")
EXPECTED_UID = 2000
MAX_CONFIG_FILES = 128
MAX_CONFIG_BYTES = 64 * 1024
MAX_TOTAL_CONFIG_BYTES = 1024 * 1024


def fail(message: str) -> NoReturn:
    raise SystemExit(f"invalid company supervisor config: {message}")


def validate(
    directory: Path,
    *,
    expected_directory: Path = EXPECTED_DIRECTORY,
    expected_uid: int = EXPECTED_UID,
) -> None:
    if not directory.is_absolute() or directory != expected_directory:
        fail("configuration directory must be /company/services/supervisor")

    try:
        entries = [
            entry
            for entry in os.scandir(directory)
            if entry.name.endswith(".conf")
        ]
    except OSError as error:
        fail(f"configuration directory cannot be read: {error}")
    if len(entries) > MAX_CONFIG_FILES:
        fail(f"configuration directory exceeds {MAX_CONFIG_FILES} files")

    total_bytes = 0
    for entry in sorted(entries, key=lambda candidate: candidate.name):
        path = Path(entry.path)
        try:
            metadata = entry.stat(follow_symlinks=False)
        except OSError as error:
            fail(f"{entry.name} cannot be inspected: {error}")
        if not stat.S_ISREG(metadata.st_mode):
            fail(f"{path.name} must be a regular file")
        if metadata.st_uid != expected_uid:
            fail(f"{path.name} must be owned by uid {expected_uid}")
        if metadata.st_size > MAX_CONFIG_BYTES:
            fail(f"{path.name} exceeds {MAX_CONFIG_BYTES} bytes")
        total_bytes += metadata.st_size
        if total_bytes > MAX_TOTAL_CONFIG_BYTES:
            fail(f"configuration exceeds {MAX_TOTAL_CONFIG_BYTES} bytes in total")

        parser = configparser.ConfigParser(interpolation=None, strict=True)
        parser.optionxform = str.lower
        try:
            with path.open("r", encoding="utf-8") as source:
                parser.read_file(source)
        except (configparser.Error, UnicodeError, OSError) as error:
            fail(f"{path.name} cannot be parsed: {error}")

        if not parser.sections():
            fail(f"{path.name} has no program section")
        for section in parser.sections():
            if not PROGRAM_SECTION.fullmatch(section):
                fail(f"{path.name} contains unsupported section [{section}]")
            if parser.has_option(section, "user"):
                fail(f"{path.name} may not select a Unix user")


if __name__ == "__main__":
    if len(sys.argv) != 2:
        fail("expected exactly one configuration directory")
    validate(Path(os.path.abspath(sys.argv[1])))
