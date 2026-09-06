import configparser
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import unittest


def configuration(name):
    if os.environ.get("RESTLESS_TEST_INSTALLED_RUNTIME") == "1":
        path = Path("/etc/supervisor/restless-init.conf" if name == "init-supervisord.conf"
                    else "/etc/supervisor/conf.d/restless.conf")
    else:
        path = Path(__file__).with_name(name)
    result = configparser.ConfigParser(interpolation=None)
    result.read(path)
    return result


class SupervisorCustody(unittest.TestCase):
    def test_root_configuration_never_trusts_company_paths(self):
        root = configuration("init-supervisord.conf")
        self.assertEqual(root["supervisord"]["user"], "root")
        self.assertEqual(root["include"]["files"], "/run/restless-supervisor/*.conf")
        self.assertEqual(root["unix_http_server"]["chmod"], "0600")
        self.assertEqual(root["program:company-services"]["user"], "company")
        for section, field in [("supervisord", "logfile"), ("supervisord", "pidfile"),
                               ("supervisord", "childlogdir"), ("unix_http_server", "file"),
                               ("program:company-services", "stdout_logfile"),
                               ("program:company-services", "stderr_logfile")]:
            if field == "childlogdir":
                self.assertEqual(root[section][field], "/run/restless-control")
            else:
                self.assertTrue(root[section][field].startswith("/run/restless-control/"),
                                (section, field))

    def test_company_supervisor_is_unprivileged_and_cannot_load_agent(self):
        company = configuration("supervisord.conf")
        self.assertEqual(company["supervisord"]["user"], "company")
        self.assertEqual(company["include"]["files"], "/company/services/supervisor/*.conf")
        self.assertNotIn("program:runtime-agent", company)

    @unittest.skipUnless(os.getuid() == 0 and shutil.which("supervisord") and shutil.which("supervisorctl"),
                         "requires root in the disposable Linux Runtime")
    def test_company_rpc_can_run_services_but_not_root_commands_or_private_rpc(self):
        import pwd
        self.assertEqual(pwd.getpwnam("company").pw_uid, 2000)
        with tempfile.TemporaryDirectory(prefix="restless-supervisor-custody-") as directory:
            base = Path(directory)
            base.chmod(0o755)
            private = base / "control"
            private.mkdir(mode=0o700)
            marker = private / "marker"
            marker.write_text("synthetic-only")
            owned = base / "company"
            owned.mkdir(mode=0o700)
            os.chown(owned, 2000, 2000)
            company = configuration("supervisord.conf")
            for section in list(company.sections()):
                if section.startswith("program:"):
                    company.remove_section(section)
            company["supervisord"]["logfile"] = str(owned / "supervisord.log")
            company["supervisord"]["pidfile"] = str(owned / "supervisord.pid")
            company["supervisord"]["childlogdir"] = str(owned)
            company["unix_http_server"]["file"] = str(owned / "supervisor.sock")
            company["supervisorctl"]["serverurl"] = "unix://" + str(owned / "supervisor.sock")
            company["include"]["files"] = str(owned / "*.conf")
            company_path = base / "company.ini"
            with company_path.open("w") as output:
                company.write(output)
            root = configuration("init-supervisord.conf")
            for section in root.sections():
                for field, value in list(root[section].items()):
                    root[section][field] = value.replace("/run/restless-control", str(private))
            root["include"]["files"] = str(private / "*.conf")
            root["program:company-services"]["command"] = f"/usr/bin/supervisord -n -c {company_path}"
            root_path = base / "root.ini"
            with root_path.open("w") as output:
                root.write(output)

            def unprivileged():
                os.setgroups([])
                os.setgid(2000)
                os.setuid(2000)

            def owner(args):
                return subprocess.run(args, text=True, capture_output=True, timeout=10,
                                      preexec_fn=unprivileged)

            ctl = ["supervisorctl", "-c", str(company_path)]
            daemon = subprocess.Popen(["supervisord", "-n", "-c", str(root_path)],
                                      stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
            try:
                for _ in range(100):
                    if owner(ctl + ["pid"]).returncode == 0:
                        break
                    time.sleep(0.05)
                else:
                    self.fail("company supervisor did not start")
                denied = owner(["supervisorctl", "-c", str(root_path), "pid"])
                self.assertNotEqual(denied.returncode, 0, "company reached privileged RPC")
                # Create program config as the actual company UID. It omits a
                # user field: the daemon itself must already be unprivileged.
                script = owned / "probe.py"
                proof = owned / "proof.log"
                program = owned / "probe.conf"
                source = ("import os\nfrom pathlib import Path\nprint(os.getuid(), flush=True)\n"
                          f"Path({str(marker)!r}).read_text()\n")
                spec = (f"[program:probe]\ncommand=python3 {script}\nautostart=false\n"
                        f"autorestart=false\nstartsecs=0\nstdout_logfile={proof}\nstderr_logfile={proof}\n")
                result = owner(["python3", "-c", f"from pathlib import Path; Path({str(script)!r}).write_text({source!r}); Path({str(program)!r}).write_text({spec!r})"])
                self.assertEqual(result.returncode, 0, result.stderr)
                for args in [["reread"], ["add", "probe"], ["start", "probe"]]:
                    result = owner(ctl + args)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                for _ in range(100):
                    text = proof.read_text() if proof.exists() else ""
                    if "PermissionError" in text:
                        break
                    time.sleep(0.05)
                self.assertTrue(text.startswith("2000\n"), text)
                self.assertIn("PermissionError", text)
                # Explicitly asking that same company RPC daemon for UID 0 also
                # fails at the kernel's setuid boundary, not a naming filter.
                root_proof = owned / "root-proof.log"
                root_spec = spec.replace("program:probe", "program:root-probe").replace(str(proof), str(root_proof)) + "user=root\n"
                root_program = owned / "root-probe.conf"
                created = owner(["python3", "-c", f"from pathlib import Path; Path({str(root_program)!r}).write_text({root_spec!r})"])
                self.assertEqual(created.returncode, 0, created.stderr)
                owner(ctl + ["reread"])
                owner(ctl + ["add", "root-probe"])
                # With startsecs=0 RPC may acknowledge fork before the child
                # fails setuid. Observe that failure, not just the RPC status.
                owner(ctl + ["start", "root-probe"])
                for _ in range(100):
                    root_text = root_proof.read_text() if root_proof.exists() else ""
                    if "setuid" in root_text.lower():
                        break
                    time.sleep(0.05)
                self.assertIn("setuid", root_text.lower(), root_text)
                self.assertNotIn("synthetic-only", root_text)
                self.assertFalse(root_text.startswith("0\n"), root_text)
            finally:
                daemon.terminate()
                try:
                    daemon.wait(timeout=15)
                except subprocess.TimeoutExpired:
                    daemon.kill()
                    daemon.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
