#!/usr/bin/env python3
"""Exercise unavailable Docker observations without touching host resources."""
import os
from pathlib import Path
import signal
import subprocess
import tempfile
import unittest


SOURCE = (Path(__file__).parent / 'restless-reap').read_text()
PREAMBLE = SOURCE.split('# --- 1. Orphaned test-company configs')[0]


class UnavailableDockerTests(unittest.TestCase):
    def run_fixture(self, directory, source, docker, mode='--check'):
        directory = Path(directory)
        binary = directory / 'bin'
        binary.mkdir()
        fake = binary / 'docker'
        fake.write_text('#!/usr/bin/env bash\n'+docker)
        fake.chmod(0o700)
        script = directory / 'reap'
        script.write_text(source)
        environment = dict(os.environ, PATH=str(binary)+':'+os.environ['PATH'],
                           RESTLESS_COMPANY_DIR=str(directory / 'companies'),
                           STACK_REPO_ROOT=str(directory), RESTLESS_REAP_QUERY_TIMEOUT_SECONDS='1')
        child = subprocess.Popen(['bash', str(script), mode], env=environment,
                                 stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True,
                                 start_new_session=True)
        try:
            output, _ = child.communicate(timeout=4)
            self.assertEqual(child.returncode, 0, output)
            return output
        finally:
            # A failed timeout assertion must not abandon even the fake CLI.
            try:
                os.killpg(child.pid, signal.SIGKILL)
            except ProcessLookupError:
                pass
            child.wait()
            child.stdout.close()

    def test_failed_inventory_cannot_delete_a_test_company(self):
        with tempfile.TemporaryDirectory(prefix='reap-test-') as scratch:
            company = Path(scratch) / 'companies' / 'keep_test.toml'
            company.parent.mkdir()
            company.write_text('owner data retained for this fixture\n')
            source = SOURCE.split('# --- 2. Exited test containers')[0]
            output = self.run_fixture(scratch, source, '''
case "$1" in
  info) exit 0 ;;
  ps) exit 42 ;;
  *) printf 'unexpected mutation: %s\\n' "$*"; exit 1 ;;
esac
''', '--purge')
            self.assertTrue(company.exists(), 'an unavailable inventory is not evidence of orphanhood')
            self.assertIn('inventory unavailable', output)
            self.assertNotIn('unexpected mutation', output)

    def test_stalled_cache_query_reports_unavailable_and_continues(self):
        with tempfile.TemporaryDirectory(prefix='reap-test-') as scratch:
            cache = SOURCE.split('# --- 9. Docker build cache')[1].split('\n', 1)[1].split('# --- 10. Stale daemon socket')[0]
            source = PREAMBLE+'\n# cache section\n'+cache+"\nprintf 'reached next observation\\n'\n"
            output = self.run_fixture(scratch, source, '''
case "$1 $2" in
  'info ') exit 0 ;;
  'system df') sleep 30 ;;
  *) exit 1 ;;
esac
''')
            self.assertIn('unavailable', output)
            self.assertIn('reached next observation', output)
            self.assertNotIn('clean.', output)

    def test_unavailable_agent_inventory_cannot_deregister_a_service(self):
        with tempfile.TemporaryDirectory(prefix='reap-test-') as scratch:
            section = SOURCE.split('# --- 5. Agent-registered supervisor services')[1].split('\n', 1)[1].split('# --- 6. Unsupervised leaked processes')[0]
            output = self.run_fixture(scratch, PREAMBLE+'\n'+section, '''
case "$1" in
  info) exit 0 ;;
  ps) printf 'fixture_test\\n' ;;
  exec)
    case "$3" in
      sh) printf 'preview\\t120\\tpreview.conf\\n' ;;
      ps) exit 42 ;;
      *) printf 'unexpected mutation: %s\\n' "$*"; exit 1 ;;
    esac ;;
  *) exit 1 ;;
esac
''', '--purge')
            self.assertIn('agent inventory unavailable', output)
            self.assertNotIn('unexpected mutation', output)
            self.assertNotIn('stopped and deregistered', output)


if __name__ == '__main__':
    unittest.main()
