#!/usr/bin/env python3
"""Exercise Library and owner entry with one disposable account CPU-throttled."""
import json, os, secrets, signal, subprocess, sys, tempfile, time, urllib.parse, urllib.request
from pathlib import Path

repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / "scripts/lib"))
from smoke_plane import Plane, docker

def interrupted(*_):
    raise RuntimeError("fixture interrupted")
signal.signal(signal.SIGTERM, interrupted)
signal.signal(signal.SIGALRM, interrupted)
signal.alarm(900)
suffix = secrets.token_hex(4)
cloud = Path(os.environ["RESTLESS_CLOUD_DIR"])
daemon = os.environ["RESTLESSD_BIN"]
image = os.environ["RESTLESS_COMPANY_IMAGE"]
source = (cloud / "services/vm-provider/agent/restless-agent.sh").read_text()
start = source.index("install_workload_budget() {")
end = source.index("\nupdate_agent()", start)
planes, pools = [], []
load = None
cleanup_errors = []
with tempfile.TemporaryDirectory(prefix="s109-library-isolation-") as folder:
    root = Path(folder)
    (root / "bin").mkdir()
    (root / "bin/getconf").write_text("#!/bin/sh\necho 4\n")
    (root / "bin/getconf").chmod(0o755)
    (root / "meminfo").write_text("MemTotal: 8388608 kB\n")
    try:
        for label in ("a", "b"):
            slice_name = f"restless-s109iso{suffix}{label}.slice"
            unit = Path("/run/systemd/system") / slice_name
            if unit.exists():
                raise RuntimeError("temporary unit already exists")
            setup = root / f"setup-{label}.sh"
            setup.write_text("log(){ :; }\n" + source[start:end] + "\ninstall_workload_budget " + slice_name + "\n")
            # Track the unit before installation so partial setup also gets cleaned.
            pools.append((slice_name, unit))
            subprocess.run(["sudo", "-n", "env", "PATH=" + str(root / "bin") + ":" + os.environ["PATH"],
                            "RESTLESS_HOST_MEMINFO=" + str(root / "meminfo"), "RESTLESS_WORKLOAD_UNIT_DIR=/run/systemd/system",
                            "bash", str(setup)], check=True, timeout=30)
            pool = Path("/sys/fs/cgroup/restless.slice") / slice_name
            company = f"libiso_{suffix}_{label}_test"
            base = root / label
            base.mkdir(); (base / "home").mkdir(); (base / "plane").mkdir()
            plane = Plane(base / "plane", company, daemon, image, {
                "HOME": str(base / "home"), "RESTLESS_SOURCE_ROOT": str(repo),
                "RESTLESS_COCKPIT_DIR": str(repo / "web/build"),
                "RESTLESS_COMPANY_CPUS": "1", "RESTLESS_COMPANY_MEMORY": "1024m",
                "RESTLESS_COMPUTER_MEMORY": "512m", "RESTLESS_WORKLOAD_CGROUP_PARENT": slice_name,
                "RESTLESS_WORKLOAD_CGROUP_ROOT": str(pool), "RESTLESS_TEST_DISABLE_SCHEDULER": "1"})
            planes.append((plane, company, pool, slice_name))
            plane.start_database(); plane.start(); plane.wait_for_port(plane.offset + 7788)
            plane.create_company(company, f'name = "{company}"\nauto_sleep_after_minutes = 0\n')
            plane.raw(company, {"cmd": "up"}, timeout=300)
            docker("exec", "-u", "company", plane.container(company), "sh", "-c",
                   "mkdir -p /company/outputs/isolation && printf ready > /company/outputs/isolation/visible.txt")
            status, body = plane.owner(company, "/library/outputs?fresh=true")
            assert status == 200 and any(f["path"].endswith("/isolation/visible.txt") for f in body["files"])
        pressured, name, pool, slice_name = planes[0]
        peer, peer_name, _, _ = planes[1]
        # Exercise multiple pages of real output while the first account is busy.
        docker("exec", "-u", "company", pressured.container(name), "sh", "-c",
               'n=1; while [ "$n" -le 4000 ]; do : > "/company/outputs/isolation/$n.txt"; n=$((n+1)); done')
        # Generate work in this account's actual workload cgroup. Its quota is
        # one quarter of one CPU, leaving both owner APIs outside the pool.
        subprocess.run(["sudo", "-n", "systemctl", "set-property", "--runtime", slice_name, "CPUQuota=25%"], check=True, timeout=30)
        def throttled():
            return int(dict(line.split() for line in (pool / "cpu.stat").read_text().splitlines())["nr_throttled"])
        before = throttled()
        load = subprocess.Popen(["docker", "exec", "-u", "company", pressured.container(name),
                                 "timeout", "180", "sh", "-c", "while :; do :; done"],
                                stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        for _ in range(40):
            if load.poll() is not None: raise RuntimeError("CPU load exited before pressure was observed")
            if throttled() > before: break
            time.sleep(0.5)
        else: raise RuntimeError("account CPU was not throttled")
        started = time.monotonic()
        for plane, company in ((pressured, name), (peer, peer_name)):
            status, body = plane.owner(company, "/library/outputs?fresh=true")
            assert status == 200, (company, status, body)
            if plane is pressured:
                paths = [f["path"] for f in body["files"]]
                while body["truncated"]:
                    status, body = plane.owner(company, "/library/outputs?" + urllib.parse.urlencode(body["next_cursor"]))
                    assert status == 200, (company, status, body)
                    paths.extend(f["path"] for f in body["files"])
                assert len(paths) == 4001 and len(set(paths)) == 4001, len(paths)
                assert any(path.endswith("/isolation/visible.txt") for path in paths)
            else:
                assert any(f["path"].endswith("/isolation/visible.txt") for f in body["files"])
            with urllib.request.urlopen(plane.owner_origin + "/health", timeout=10) as response:
                assert response.status == 200
            for path in ("/principal", "/attention", "/work-graph"):
                status, _ = plane.owner(company, path)
                assert status == 200, (company, path, status)
        assert load.poll() is None, "CPU load must remain active throughout entry checks"
        after = throttled()
        assert after > before
        print(json.dumps({"accounts": 2, "cpuQuotaPercent": 25, "throttledPeriods": after-before,
                          "freshListings": 2, "pressuredFiles": 4001, "entryChecks": 8,
                          "elapsedSeconds": round(time.monotonic()-started, 2)}), flush=True)
    finally:
        if load and load.poll() is None:
            load.terminate()
            try: load.wait(timeout=15)
            except subprocess.TimeoutExpired:
                load.kill(); load.wait()
        for plane, _, _, _ in reversed(planes):
            cleanup_errors.extend(plane.close())
        for name, unit in reversed(pools):
            if not unit.exists():
                continue
            cg = Path("/sys/fs/cgroup/restless.slice") / name
            for _ in range(30):
                if not cg.exists() or not any(p.read_text().strip() for p in cg.rglob("cgroup.procs") if p.exists()):
                    break
                time.sleep(1)
            else:
                cleanup_errors.append(f"pool still has processes: {name}")
                continue
            subprocess.run(["sudo", "-n", "systemctl", "stop", name], check=True, timeout=30)
            subprocess.run(["sudo", "-n", "rm", str(unit)], check=True, timeout=30)
            subprocess.run(["sudo", "-n", "systemctl", "daemon-reload"], check=True, timeout=30)
        print("cleanup:", json.dumps(cleanup_errors), flush=True)
        if cleanup_errors:
            raise RuntimeError("fixture cleanup failed")
