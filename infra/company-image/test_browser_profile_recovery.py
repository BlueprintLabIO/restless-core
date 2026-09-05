import fcntl
import importlib.util
import os
from pathlib import Path
import subprocess
import shutil
import sys
import tempfile
import unittest

recovery_path = (Path("/usr/local/lib/restless/recover-browser-profile.py")
                 if os.environ.get("RESTLESS_TEST_INSTALLED_RUNTIME") == "1"
                 else Path(__file__).with_name("recover-browser-profile.py"))
spec = importlib.util.spec_from_file_location("recovery", recovery_path)
recovery = importlib.util.module_from_spec(spec)
spec.loader.exec_module(recovery)


class BrowserProfileRecovery(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.root = Path(self.temporary.name)
        self.company = self.root / "company"
        self.company.mkdir()
        self.fd = os.open(self.company, os.O_RDONLY | os.O_DIRECTORY)
        self.addCleanup(os.close, self.fd)
        self.addCleanup(self.temporary.cleanup)

    def seed(self):
        profile = self.company / "browser-profile"
        profile.mkdir()
        for name in recovery.SINGLETON_LINKS:
            (profile / name).symlink_to("old-host-or-socket")
        (profile / "Cookies").write_bytes(b"persistent-cookie-data")
        (profile / "restless-tabs.json").write_text('["file:///company/test.html"]')
        return profile

    def test_first_boot_never_creates_profile(self):
        self.assertEqual(recovery.recover_profile(self.fd), 0)
        self.assertFalse((self.company / "browser-profile").exists())

    def test_only_three_transient_links_are_removed(self):
        profile = self.seed()
        self.assertEqual(recovery.recover_profile(self.fd), 3)
        self.assertEqual((profile / "Cookies").read_bytes(), b"persistent-cookie-data")
        self.assertEqual((profile / "restless-tabs.json").read_text(), '["file:///company/test.html"]')
        self.assertEqual(recovery.recover_profile(self.fd), 0)

    def test_live_lock_refuses_recovery_without_touching_profile(self):
        profile = self.seed()
        fcntl.flock(self.fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        # A distinct process/open description, not an inherited copy of our lock.
        result = subprocess.run([sys.executable, "-c", "import fcntl,os,sys; f=os.open(sys.argv[1],os.O_RDONLY); fcntl.flock(f,fcntl.LOCK_EX|fcntl.LOCK_NB)", str(self.company)], capture_output=True)
        self.assertNotEqual(result.returncode, 0)
        rival = os.open(self.company, os.O_RDONLY | os.O_DIRECTORY)
        try:
            with self.assertRaises(BlockingIOError):
                recovery.recover_profile(rival)
        finally:
            os.close(rival)
        self.assertTrue(all((profile / name).is_symlink() for name in recovery.SINGLETON_LINKS))

    def test_profile_symlink_cannot_redirect_cleanup(self):
        outside = self.root / "outside"
        outside.mkdir()
        (outside / "SingletonLock").symlink_to("keep-me")
        (self.company / "browser-profile").symlink_to(outside, target_is_directory=True)
        with self.assertRaises(OSError):
            recovery.recover_profile(self.fd)
        self.assertTrue((outside / "SingletonLock").is_symlink())

    def test_unexpected_reserved_file_preserves_all_entries(self):
        profile = self.seed()
        (profile / "SingletonCookie").unlink()
        (profile / "SingletonCookie").write_text("do-not-delete")
        with self.assertRaises(RuntimeError):
            recovery.recover_profile(self.fd)
        self.assertTrue((profile / "SingletonLock").is_symlink())
        self.assertEqual((profile / "SingletonCookie").read_text(), "do-not-delete")

    def test_link_target_is_never_removed(self):
        profile = self.seed()
        target = self.root / "protected"
        target.write_text("keep-me")
        (profile / "SingletonSocket").unlink()
        (profile / "SingletonSocket").symlink_to(target)
        recovery.recover_profile(self.fd)
        self.assertEqual(target.read_text(), "keep-me")

    @unittest.skipUnless(shutil.which("flock") and shutil.which("tini"), "Linux Runtime tools required")
    def test_tini_retains_lock_but_child_does_not_inherit_it(self):
        script = ('exec 9<"$1"; flock -n 9 || exit 75; '
                  'exec tini -- sh -c \'exec 9<&-; test ! -e /proc/self/fd/9 || exit 76; '
                  'printf "ready\\n"; exec sleep 30\'')
        holder = subprocess.Popen(["sh", "-c", script, "guard-test", str(self.company)],
                                  stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        try:
            self.assertEqual(holder.stdout.readline(), "ready\n")
            self.assertEqual(os.readlink(f"/proc/{holder.pid}/fd/9"), str(self.company))
            with self.assertRaises(BlockingIOError):
                fcntl.flock(self.fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
            holder.terminate()
            holder.wait(timeout=5)
            # Once the init exits, a fresh owner can acquire the same inode.
            fcntl.flock(self.fd, fcntl.LOCK_EX | fcntl.LOCK_NB)
        finally:
            if holder.poll() is None:
                holder.kill()
                holder.wait(timeout=5)
            holder.stdout.close()


if __name__ == "__main__":
    unittest.main()
