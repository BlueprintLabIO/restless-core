"""Run native Staff prompt-cap checks with a disposable PostgreSQL container.

The test uses a simulated model session and never contacts a model provider.
Requires Docker and the normal Rust build tools. The fixture removes its own
uniquely named database container and temporary files on completion.
"""
import os
import secrets
import signal
import subprocess
import sys
import tempfile
from pathlib import Path

repo = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(repo / 'scripts/lib'))
from smoke_plane import Plane  # noqa: E402


def interrupted(*_):
    raise RuntimeError('native Staff cap test interrupted')


signal.signal(signal.SIGTERM, interrupted)
signal.signal(signal.SIGALRM, interrupted)
signal.alarm(900)
with tempfile.TemporaryDirectory(prefix='restless-native-staff-cap-') as folder:
    root = Path(folder)
    (root / 'plane').mkdir()
    (root / 'runs').mkdir()
    namespace = 'nativecap_' + secrets.token_hex(3) + '_test'
    plane = Plane(root / 'plane', namespace, '/bin/false', 'unused')
    try:
        plane.start_database()
        env = os.environ.copy()
        env.update(RESTLESS_TEST_DATABASE_URL=plane.admin_url,
                   RESTLESS_NATIVE_TEST_ROOT=str(root / 'runs'))
        for scenario in (
            'native_staff_rechecks_before_completion_and_records_unknown_turns',
            'native_prompt_admission_coordinates_caps_without_blocking_peers',
            'concurrent_native_exec_requests_cannot_share_one_remaining_turn',
        ):
            subprocess.run([str(repo / 'scripts/restless-cargo'), 'test', '-p', 'restless-engine',
                            '--lib', scenario, '--', '--nocapture'],
                           cwd=repo, env=env, check=True, timeout=600)
    finally:
        problems = plane.close()
        print('fixture cleanup problems:', problems, flush=True)
        if problems:
            raise RuntimeError('native Staff cap fixture cleanup failed')
