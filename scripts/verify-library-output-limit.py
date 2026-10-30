#!/usr/bin/env python3
"""Verify Library output truncation against an isolated local company and browser."""
import argparse, json, secrets, shutil, signal, subprocess, sys, tempfile, urllib.parse
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
               'mkdir -p /company/outputs/limit-test; n=1; while [ "$n" -le 2000 ]; do : > "/company/outputs/limit-test/$n.txt"; n=$((n+1)); done; find /company/outputs/limit-test -type f -exec touch -t 202610300101 {} +')
        status, exact = plane.owner(company, "/library/outputs")
        assert status == 200 and len(exact["files"]) == 2000 and exact["truncated"] is False, (status, len(exact.get("files", [])), exact.get("truncated"))
        subprocess.run(["node", str(repo / "web/scripts/verify-library-limit.mjs"), plane.owner_origin, company, "2000"], check=True)
        docker("exec", "-u", "company", plane.container(company), "sh", "-c", ': > /company/outputs/limit-test/2001.txt; touch -t 202610300101 /company/outputs/limit-test/2001.txt')
        # Clear only the disposable daemon's 30-second listing cache.
        plane.stop()
        plane.start()
        plane.wait_for_port(plane.offset + 7788, 120)
        status, more = plane.owner(company, "/library/outputs")
        assert status == 200 and len(more["files"]) == 2000 and more["truncated"] is True, (status, len(more.get("files", [])), more.get("truncated"))
        cursor = more["next_cursor"]
        status, older = plane.owner(company, "/library/outputs?" + urllib.parse.urlencode(cursor))
        assert status == 200 and len(older["files"]) == 1 and older["truncated"] is False, (status, older)
        paths = {entry["path"] for entry in more["files"] + older["files"]}
        assert len(paths) == 2001 and all(f"/company/outputs/limit-test/{n}.txt" in paths for n in range(1, 2002))
        subprocess.run(["node", str(repo / "web/scripts/verify-library-limit.mjs"), plane.owner_origin, company, "2000+"], check=True)
        docker("exec", "-u", "company", plane.container(company), "sh", "-c",
               'n=2002; while [ "$n" -le 4001 ]; do : > "/company/outputs/limit-test/$n.txt"; n=$((n+1)); done; find /company/outputs/limit-test -type f -exec touch -t 202610300101 {} +')
        plane.stop()
        plane.start()
        plane.wait_for_port(plane.offset + 7788, 120)
        status, page = plane.owner(company, "/library/outputs")
        assert status == 200
        pages, paths = [], set()
        while True:
            pages.append(len(page["files"]))
            for entry in page["files"]:
                assert entry["path"] not in paths, "duplicate across pages"
                paths.add(entry["path"])
            if not page["truncated"]:
                break
            status, page = plane.owner(company, "/library/outputs?" + urllib.parse.urlencode(page["next_cursor"]))
            assert status == 200
            assert len(pages) < 4, "cursor did not advance"
        assert pages == [2000, 2000, 1] and len(paths) == 4001, (pages, len(paths))
        assert all(f"/company/outputs/limit-test/{n}.txt" in paths for n in range(1, 4002))
        print(json.dumps({"company": company, "exact": 2000, "pageSizes": pages, "uniqueFiles": len(paths)}), flush=True)
    finally:
        problems = plane.close()
        print("cleanup:", json.dumps(problems), flush=True)
        if problems:
            raise RuntimeError("fixture cleanup failed")
