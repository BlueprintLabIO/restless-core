import os
from pathlib import Path
import shlex
import subprocess
import unittest


class BrowserLauncherInterpreter(unittest.TestCase):
    def launcher(self):
        if os.environ.get("RESTLESS_TEST_INSTALLED_RUNTIME") == "1":
            return Path("/usr/local/bin/start-company-chromium")
        return Path(__file__).with_name("start-chromium.sh")

    def command(self):
        lines = [line for line in self.launcher().read_text().splitlines()
                 if line.startswith("exec ")]
        self.assertEqual(len(lines), 1)
        return shlex.split(lines[0])[1:]

    def test_launcher_uses_image_node_not_desktop_dependency_node(self):
        self.assertEqual(self.command()[:2], [
            "/usr/local/bin/node", "/usr/local/lib/restless/run-chromium.mjs"])

    @unittest.skipUnless(os.environ.get("RESTLESS_TEST_INSTALLED_RUNTIME") == "1",
                         "requires the built Runtime image")
    def test_actual_launcher_interpreter_has_native_close_transport(self):
        command = self.command()
        # Exercise the interpreter selected by the installed shell launcher,
        # not whatever `node` happens to resolve to on the test runner's PATH.
        source = """
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
assert.equal(typeof globalThis.fetch, 'function');
assert.equal(typeof globalThis.WebSocket, 'function');
const { closeBrowser } = await import(pathToFileURL(process.argv[2]).href);
await assert.rejects(closeBrowser({fetchImpl: async () => ({ok:false})}), /browser discovery unavailable/);
"""
        result = subprocess.run([command[0], "--input-type=module", "-e", source,
                                 "installed-launcher-test", command[1]],
                                text=True, capture_output=True, timeout=15)
        self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
