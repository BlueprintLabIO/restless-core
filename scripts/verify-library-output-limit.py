#!/usr/bin/env python3
"""Verify Library output truncation against an isolated local company and browser."""
import argparse, json, secrets, shutil, signal, subprocess, sys, tempfile
from pathlib import Path

repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / "scripts/lib"))
from smoke_plane import Plane, docker

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--daemon", required=True)
parser.add_argument("--image", required=True)
args = parser.parse_args()
def interrupted(*_):
    raise RuntimeError("fixture interrupted")
signal.signal(signal.SIGTERM, interrupted)
signal.signal(signal.SIGALRM, interrupted)
signal.alarm(900)
assert (repo / "web/build/index.html").is_file(), "Build the cockpit before running this test"
with tempfile.TemporaryDirectory(prefix="s109-library-limit-") as directory:
    root = Path(directory)
    (root / "plane").mkdir()
    (root / "home").mkdir()
    suffix = secrets.token_hex(3)
    namespace = f"librarylimit_{suffix}_test"
    company = namespace
    plane = Plane(root / "plane", namespace, args.daemon, args.image, {
        "HOME": str(root / "home"), "RESTLESS_SOURCE_ROOT": str(repo),
        "RESTLESS_COCKPIT_DIR": str(repo / "web/build"),
        "RESTLESS_NODE_BIN": shutil.which("node"), "RESTLESS_TEST_DISABLE_SCHEDULER": "1",
    })
    try:
        plane.start_database()
        plane.start()
        plane.create_company(company, f'name = "{company}"\nauto_sleep_after_minutes = 0\n')
        plane.raw(company, {"cmd": "up"}, timeout=600)
        plane.wait_for_port(plane.offset + 7788, 120)
        docker("exec", "-u", "company", plane.container(company), "sh", "-c",
               'mkdir -p /company/outputs/limit-test; n=1; while [ "$n" -le 2000 ]; do : > "/company/outputs/limit-test/$n.txt"; n=$((n+1)); done')
        status, exact = plane.owner(company, "/library/outputs")
        assert status == 200 and len(exact["files"]) == 2000 and exact["truncated"] is False, (status, len(exact.get("files", [])), exact.get("truncated"))
        subprocess.run(["node", str(repo / "web/scripts/verify-library-limit.mjs"), plane.owner_origin, company, "2000"], check=True)
        docker("exec", "-u", "company", plane.container(company), "sh", "-c", ': > /company/outputs/limit-test/2001.txt')
        # Clear only the disposable daemon's 30-second listing cache.
        plane.stop()
        plane.start()
        plane.wait_for_port(plane.offset + 7788, 120)
        status, more = plane.owner(company, "/library/outputs")
        assert status == 200 and len(more["files"]) == 2000 and more["truncated"] is True, (status, len(more.get("files", [])), more.get("truncated"))
        subprocess.run(["node", str(repo / "web/scripts/verify-library-limit.mjs"), plane.owner_origin, company, "2000+"], check=True)
        print(json.dumps({"company": company, "exact": 2000, "moreThanLimit": True}), flush=True)
    finally:
        problems = plane.close()
        print("cleanup:", json.dumps(problems), flush=True)
        if problems:
            raise RuntimeError("fixture cleanup failed")
