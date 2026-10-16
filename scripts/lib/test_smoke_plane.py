"""Port-isolation regressions: no daemon, Docker or real company is touched."""
import json
import pathlib
import socket
import subprocess
import sys
import unittest

from smoke_plane import reserve_offset


class SmokePortIsolation(unittest.TestCase):
    def child_offset(self):
        source = pathlib.Path(__file__).resolve().parent
        result = subprocess.run([
            sys.executable, '-c',
            "import json; from smoke_plane import reserve_offset; "
            "offset, lease, ports = reserve_offset(); print(json.dumps(offset))"
        ], cwd=source, capture_output=True, text=True, timeout=10, check=True)
        return json.loads(result.stdout)

    def test_another_process_cannot_choose_a_reserved_port_block(self):
        offset, lease, ports = reserve_offset()
        try:
            with socket.socket() as probe:
                with self.assertRaises(OSError):
                    probe.bind(('127.0.0.1', offset + 7788))
            self.assertNotEqual(self.child_offset(), offset)
        finally:
            for probe in ports:
                probe.close()
            lease.close()

    def test_lease_protects_daemon_handoff_and_releases_after_exit(self):
        offset, lease, ports = reserve_offset()
        for probe in ports:
            probe.close()
        try:
            self.assertNotEqual(self.child_offset(), offset,
                                'Closing the sockets for launch must retain the process lease')
        finally:
            lease.close()
        selected, next_lease, next_ports = reserve_offset(offset, offset + 100)
        try:
            self.assertEqual(selected, offset)
        finally:
            for probe in next_ports:
                probe.close()
            next_lease.close()


if __name__ == '__main__':
    unittest.main()
