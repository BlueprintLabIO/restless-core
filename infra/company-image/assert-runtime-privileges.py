#!/usr/bin/python3
"""Fail closed unless a Company Runtime process has its exact privilege shape."""

from __future__ import annotations

import os
from pathlib import Path
import sys


COMPANY_UID = 2000
COMPANY_GID = 2000
BRIDGE_SECRET_GID = 10002

# CAP_CHOWN (0), CAP_KILL (5), CAP_SETGID (6), CAP_SETUID (7), CAP_SETPCAP (8).
TRUSTED_CAPABILITIES = (1 << 0) | (1 << 5) | (1 << 6) | (1 << 7) | (1 << 8)
CAPABILITY_FIELDS = ("CapInh", "CapPrm", "CapEff", "CapBnd", "CapAmb")


def fail(message: str) -> None:
    raise SystemExit(f"Company Runtime privilege contract refused startup: {message}")


def process_status() -> dict[str, str]:
    try:
        lines = Path("/proc/self/status").read_text(encoding="utf-8").splitlines()
    except OSError as error:
        fail(f"cannot inspect /proc/self/status: {error}")
    return {
        key: value.strip()
        for line in lines
        if ":" in line
        for key, value in [line.split(":", 1)]
    }


def capability(status: dict[str, str], field: str) -> int:
    value = status.get(field)
    if value is None:
        fail(f"{field} is absent from /proc/self/status")
    try:
        return int(value, 16)
    except ValueError:
        fail(f"{field} is not a hexadecimal capability mask")


def require_no_new_privileges(status: dict[str, str]) -> None:
    if status.get("NoNewPrivs") != "1":
        fail("NoNewPrivs is not set")


def require_company_process(status: dict[str, str]) -> None:
    ids = (os.getuid(), os.geteuid(), os.getgid(), os.getegid())
    if ids != (COMPANY_UID, COMPANY_UID, COMPANY_GID, COMPANY_GID):
        fail(f"company process identity is {ids}, expected 2000:2000")
    if os.getgroups():
        fail(f"company process retained supplementary groups {os.getgroups()}")
    for field in CAPABILITY_FIELDS:
        if capability(status, field) != 0:
            fail(f"company process retained {field}")
    require_no_new_privileges(status)


def require_trusted_bridge(status: dict[str, str]) -> None:
    ids = (os.getuid(), os.geteuid(), os.getgid(), os.getegid())
    if ids != (0, 0, COMPANY_GID, COMPANY_GID):
        fail(f"trusted bridge identity is {ids}, expected uid 0 and gid 2000")
    if os.getgroups() != [BRIDGE_SECRET_GID]:
        fail(
            "trusted bridge supplementary groups are "
            f"{os.getgroups()}, expected only [{BRIDGE_SECRET_GID}]"
        )
    for field in ("CapPrm", "CapEff", "CapBnd"):
        observed = capability(status, field)
        if observed != TRUSTED_CAPABILITIES:
            fail(
                f"trusted bridge {field} is 0x{observed:x}, "
                f"expected exactly 0x{TRUSTED_CAPABILITIES:x}"
            )
    for field in ("CapInh", "CapAmb"):
        if capability(status, field) != 0:
            fail(f"trusted bridge retained {field}")
    require_no_new_privileges(status)


def main() -> None:
    if len(sys.argv) != 2 or sys.argv[1] not in {"company", "trusted-bridge"}:
        fail("usage: assert-runtime-privileges.py company|trusted-bridge")
    status = process_status()
    if sys.argv[1] == "company":
        require_company_process(status)
    else:
        require_trusted_bridge(status)


if __name__ == "__main__":
    main()
