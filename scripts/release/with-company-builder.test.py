#!/usr/bin/env python3
"""Concurrent jobs must reuse one builder without replacing each other's config."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import time
import unittest


class BuilderReuseTests(unittest.TestCase):
    def test_concurrent_initialization_creates_only_one_builder(self):
        with tempfile.TemporaryDirectory(prefix='builder-race-test-') as scratch:
            root = Path(scratch)
            script = root/'with-company-builder.sh'
            shutil.copyfile(Path(__file__).with_name('with-company-builder.sh'),script)
            (root/'prepare-builder-dns.sh').write_text('#!/usr/bin/env bash\nexit 0\n')
            binary = root/'bin'
            binary.mkdir()
            docker = binary/'docker'
            docker.write_text('''#!/usr/bin/env bash
set -eu
case "$1 $2" in
  'buildx create')
    printf 'create\\n' >> "$RESTLESS_BUILDER_TEST_STATE/operations"
    touch "$RESTLESS_BUILDER_TEST_STATE/creating"
    sleep 0.5
    touch "$RESTLESS_BUILDER_TEST_STATE/ready"
    ;;
  'buildx inspect')
    [ -f "$RESTLESS_BUILDER_TEST_STATE/ready" ] || exit 1
    case "$*" in *--format*) printf 'fixture-builder0\\n';; esac
    ;;
  'buildx rm') : ;;
  'buildx ls')
    # Buildx repeats the builder context for its node row.
    for row in 1 2; do printf '{"Name":"fixture-builder","Nodes":[{"Name":"fixture-builder0"}]}\\n'; done
    ;;
  'update '*) [ "${@: -1}" = buildx_buildkit_fixture-builder0 ] ;;
  *) exit 2 ;;
esac
''')
            docker.chmod(0o700)
            environment = dict(os.environ, PATH=str(binary)+':'+os.environ['PATH'],
                               XDG_CACHE_HOME=str(root/'cache'), RESTLESS_BUILDER_TEST_STATE=str(root),
                               RESTLESS_COMPANY_BUILDER='fixture-builder')
            first = subprocess.Popen(['bash',str(script),'true'],env=environment,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
            second = None
            try:
                deadline = time.monotonic()+5
                while not (root/'creating').exists() and time.monotonic()<deadline:
                    time.sleep(.01)
                self.assertTrue((root/'creating').exists(),'first builder creation did not start')
                second = subprocess.Popen(['bash',str(script),'true'],env=environment,stdout=subprocess.PIPE,stderr=subprocess.STDOUT,text=True)
                for child in [first,second]:
                    output,_ = child.communicate(timeout=10)
                    self.assertEqual(child.returncode,0,output)
                self.assertEqual((root/'operations').read_text().splitlines(),['create'])
                self.assertEqual(list((root/'cache/restless-runner').glob('company-buildkitd.*')), [root/'cache/restless-runner/company-buildkitd.toml'])
            finally:
                for child in [first,second]:
                    if child is not None:
                        if child.poll() is None:
                            child.kill()
                        child.wait()
                        child.stdout.close()


if __name__=='__main__':
    unittest.main()
