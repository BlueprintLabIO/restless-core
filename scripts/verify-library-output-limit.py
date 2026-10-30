#!/usr/bin/env python3
"""Verify Library output truncation against an isolated local company and browser."""
import argparse, datetime, json, secrets, shutil, signal, subprocess, sys, tempfile, urllib.parse
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
        special_names = ["line\nbreak.txt", "tab\tname.txt", "odd'$(echo nope)\té.txt"]
        create_special = "import os,sys; root='/company/outputs/limit-test'; names=" + repr(special_names) + "; [open(os.path.join(root,n),'w').close() for n in names]; [os.utime(os.path.join(root,n),(1793322060,1793322060)) for n in names]; os.makedirs(os.path.join(root,'.hidden'),exist_ok=True); open(os.path.join(root,'.hidden','skip.txt'),'w').close(); os.makedirs(os.path.join(root,'node_modules'),exist_ok=True); open(os.path.join(root,'node_modules','skip.txt'),'w').close(); os.symlink(os.path.join(root,'1.txt'),os.path.join(root,'skip-link'))"
        docker("exec", "-u", "company", plane.container(company), "python3", "-c", create_special)
        plane.stop()
        plane.start()
        plane.wait_for_port(plane.offset + 7788, 120)
        status, page = plane.owner(company, "/library/outputs")
        assert status == 200
        first_cursor = page["next_cursor"]
        first_paths = {entry["path"] for entry in page["files"]}
        pages, paths, timestamps = [], set(), {}
        while True:
            pages.append(len(page["files"]))
            for entry in page["files"]:
                assert entry["path"] not in paths, "duplicate across pages"
                paths.add(entry["path"])
                timestamps[entry["path"]] = int(datetime.datetime.fromisoformat(entry["modified"]).timestamp())
            if not page["truncated"]:
                break
            status, page = plane.owner(company, "/library/outputs?" + urllib.parse.urlencode(page["next_cursor"]))
            assert status == 200
            assert len(pages) < 4, "cursor did not advance"
        assert pages == [2000, 2000, 4] and len(paths) == 4004, (pages, len(paths))
        assert all(f"/company/outputs/limit-test/{n}.txt" in paths for n in range(1, 4002))
        special_paths = {"/company/outputs/limit-test/" + name for name in special_names}
        assert special_paths <= paths
        assert not any("/.hidden/" in path or "/node_modules/" in path or path.endswith("/skip-link") for path in paths)
        # Cursor arguments are passed as data even when a name contains shell syntax and whitespace.
        special_path = "/company/outputs/limit-test/" + special_names[-1]
        special_time = 1793322060
        status, after_special = plane.owner(company, "/library/outputs?" + urllib.parse.urlencode({"modified": special_time, "path": special_path}))
        assert status == 200
        expected_after = sorted((path for path in paths if (timestamps[path], path.encode("utf-8")) < (special_time, special_path.encode("utf-8"))),
                                key=lambda path: (timestamps[path], path.encode("utf-8")), reverse=True)
        assert [entry["path"] for entry in after_special["files"]] == expected_after[:2000]
        # Mutations can move files across a cursor: a fresh first page and cursor
        # are needed to obtain a complete current listing, not a mixed snapshot.
        removed = sorted(first_paths)[0]
        updated = sorted(paths - first_paths)[0]
        added = "/company/outputs/limit-test/new-during-browse.txt"
        mutate = ("import os,sys; removed,updated,added=sys.argv[1:]; os.unlink(removed); "
                  "os.utime(updated,(2000000000,2000000000)); open(added,'w').close(); "
                  "os.utime(added,(2000000001,2000000001))")
        docker("exec", "-u", "company", plane.container(company), "python3", "-c", mutate, removed, updated, added)
        expected_current = (paths - {removed}) | {added}
        status, stale_tail = plane.owner(company, "/library/outputs?" + urllib.parse.urlencode(first_cursor))
        assert status == 409, "a cursor from the previous listing must be rejected"
        status, refreshed = plane.owner(company, "/library/outputs?fresh=true")
        assert status == 200
        refreshed_paths = {entry["path"] for entry in refreshed["files"]}
        assert {updated, added} <= refreshed_paths and removed not in refreshed_paths
        current = set()
        page = refreshed
        while True:
            for entry in page["files"]:
                assert entry["path"] not in current
                current.add(entry["path"])
            if not page["truncated"]:
                break
            status, page = plane.owner(company, "/library/outputs?" + urllib.parse.urlencode(page["next_cursor"]))
            assert status == 200
        assert current == expected_current
        print(json.dumps({"staleCursorRejected": True, "freshTraversalFiles": len(current)}), flush=True)
        paths = expected_current
        # An invalid UTF-8 filename must not become a false empty or complete listing.
        docker("exec", "-u", "company", plane.container(company), "python3", "-c",
               "import os; p=b'/company/outputs/limit-test/invalid-\\xff'; fd=os.open(p,os.O_CREAT|os.O_WRONLY,0o600); os.close(fd)")
        plane.stop()
        plane.start()
        plane.wait_for_port(plane.offset + 7788, 120)
        status, page = plane.owner(company, "/library/outputs")
        assert status == 200 and page["skipped_names"] == 1 and page["truncated"]
        found = set()
        while True:
            assert page["skipped_names"] == 1
            found.update(entry["path"] for entry in page["files"])
            if not page["truncated"]:
                break
            status, page = plane.owner(company, "/library/outputs?" + urllib.parse.urlencode(page["next_cursor"]))
            assert status == 200
        assert found == paths, "unsupported name hid readable files"
        subprocess.run(["node", str(repo / "web/scripts/verify-library-limit.mjs"), plane.owner_origin, company, "unsupported"], check=True)
        subprocess.run(["node", str(repo / "web/scripts/verify-library-limit.mjs"), plane.owner_origin, company, "changed", plane.container(company)], check=True)
        print(json.dumps({"company": company, "exact": 2000, "pageSizes": pages, "uniqueFiles": len(paths), "specialNames": len(special_paths), "invalidNameReported": True}), flush=True)
    finally:
        problems = plane.close()
        print("cleanup:", json.dumps(problems), flush=True)
        if problems:
            raise RuntimeError("fixture cleanup failed")
