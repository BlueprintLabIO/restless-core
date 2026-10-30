#!/usr/bin/env python3
"""Exercise Library isolation using two disposable accounts and cgroup pools."""
import json, os, secrets, signal, subprocess, sys, tempfile, time, urllib.request
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
planes, pools, old_high = [], [], {}
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
        old_high[slice_name] = (pool / "memory.high").read_text().strip()
        status, sample = pressured.owner(name, "/company/compute")
        assert status == 200
        working = sample["sample"]["work_admission"]["working_set_bytes"]
        maximum = int((pool / "memory.max").read_text().strip())
        high = working + min(128 * 1024 * 1024, working // 2)
        assert working > 0 and high < maximum
        subprocess.run(["sudo", "-n", "systemctl", "set-property", "--runtime", slice_name, "MemoryHigh=" + str(high)], check=True, timeout=30)
        status, sample = pressured.owner(name, "/company/compute")
        assert status == 200 and sample["sample"]["work_admission"]["waiting_for_memory"]
        status, _ = pressured.owner(name, "/library/outputs?fresh=true")
        assert status == 503, "pressured account must defer its scan"
        status, body = peer.owner(peer_name, "/library/outputs?fresh=true")
        assert status == 200 and any(f["path"].endswith("/isolation/visible.txt") for f in body["files"])
        for plane, company in ((pressured, name), (peer, peer_name)):
            with urllib.request.urlopen(plane.owner_origin + "/health", timeout=10) as response:
                assert response.status == 200
            for path in ("/principal", "/attention", "/work-graph"):
                status, _ = plane.owner(company, path)
                assert status == 200, (company, path, status)
        subprocess.run(["sudo", "-n", "systemctl", "set-property", "--runtime", slice_name, "MemoryHigh=" + old_high[slice_name]], check=True, timeout=30)
        # An interrupted diagnostic retains a short safety backoff.
        recovered = False
        for _ in range(40):
            status, body = pressured.owner(name, "/library/outputs?fresh=true")
            if status == 200:
                recovered = any(f["path"].endswith("/isolation/visible.txt") for f in body["files"])
                break
            time.sleep(1)
        assert recovered, "the deferred listing must recover without restarting the daemon"
        print(json.dumps({"accounts": 2, "pressuredListing": 503, "peerListing": 200,
                          "entryChecks": 8, "recoveredWithoutRestart": True}), flush=True)
    finally:
        for name, value in old_high.items():
            subprocess.run(["sudo", "-n", "systemctl", "set-property", "--runtime", name, "MemoryHigh=" + value], check=False, timeout=30)
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
