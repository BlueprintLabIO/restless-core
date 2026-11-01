"""List a bounded page of company outputs, newest first.

This source is embedded in the daemon and runs as the company user inside the
Runtime. Paths are encoded as JSON, so whitespace in a filename is not syntax.
"""
import heapq
import hashlib
import json
import os
import stat

limit = int(os.environ["RESTLESS_OUTPUT_LIMIT"])
cursor_time = os.environ.get("RESTLESS_CURSOR_TIME", "")
cursor_path = os.environ.get("RESTLESS_CURSOR_PATH", "")
cursor = None if cursor_time == "" else (int(cursor_time), cursor_path.encode("utf-8"))
heap = []
skipped_names = 0
revision = 0
stack = ["/company/outputs", "/company/downloads"]
while stack:
    directory = stack.pop()
    try:
        entries = os.scandir(directory)
    except FileNotFoundError:
        continue
    with entries:
        for entry in entries:
            if entry.name.startswith("."):
                continue
            try:
                path = entry.path.encode("utf-8")
            except UnicodeEncodeError:
                # The API cannot represent this name. Keep the other files available.
                skipped_names += 1
                revision ^= int.from_bytes(hashlib.sha256(b"unsupported\0" + os.fsencode(entry.path)).digest(), "big")
                continue
            try:
                if entry.is_dir(follow_symlinks=False):
                    if entry.name != "node_modules":
                        stack.append(entry.path)
                    continue
                info = entry.stat(follow_symlinks=False)
            except FileNotFoundError:
                # A file may disappear while the company is editing it.
                continue
            if not stat.S_ISREG(info.st_mode):
                continue
            revision ^= int.from_bytes(hashlib.sha256(path + b"\0" + str(info.st_mtime_ns).encode() + b"\0" + str(info.st_size).encode()).digest(), "big")
            key = (info.st_mtime_ns // 1_000_000_000, path)
            if cursor is not None and key >= cursor:
                continue
            candidate = (key, info.st_size)
            if len(heap) < limit:
                heapq.heappush(heap, candidate)
            elif key > heap[0][0]:
                heapq.heapreplace(heap, candidate)

records = [{"modified": key[0], "path": key[1].decode("utf-8"), "bytes": size}
           for key, size in sorted(heap, reverse=True)]
print(json.dumps({"files": records, "skipped_names": skipped_names, "revision": f"{revision:064x}"}, ensure_ascii=True, separators=(",", ":")))
