#!/usr/bin/python3
"""Focused security contract tests for the Company Runtime supervisor split."""

from __future__ import annotations

import configparser
import importlib.util
import os
from pathlib import Path
import tempfile
import unittest


HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def load_validator():
    path = HERE / "validate-company-supervisor-config.py"
    spec = importlib.util.spec_from_file_location("company_config_validator", path)
    if spec is None or spec.loader is None:
        raise RuntimeError("could not load company supervisor validator")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


VALIDATOR = load_validator()


def read_config(path: Path) -> configparser.ConfigParser:
    parser = configparser.ConfigParser(interpolation=None, strict=True)
    parser.read(path)
    return parser


class SupervisionContractTests(unittest.TestCase):
    def test_trusted_supervisor_never_parses_company_configuration(self) -> None:
        trusted = read_config(HERE / "supervisord.conf")
        self.assertNotIn("include", trusted.sections())
        self.assertEqual(
            {section for section in trusted.sections() if section.startswith("program:")},
            {"program:company-supervisor", "program:runtime-bridge"},
        )
        self.assertEqual(
            trusted["unix_http_server"]["file"],
            "/run/restless/trusted-supervisor/supervisor.sock",
        )
        for key in ("logfile", "pidfile", "childlogdir"):
            self.assertTrue(trusted["supervisord"][key].startswith("/run/restless/"))
        self.assertEqual(trusted["program:company-supervisor"]["user"], "company")
        self.assertEqual(trusted["program:runtime-bridge"]["user"], "root")
        self.assertNotIn("/company/services", (HERE / "supervisord.conf").read_text())

    def test_company_supervisor_and_every_built_in_child_inherit_uid_2000(self) -> None:
        company = read_config(HERE / "company-supervisord.conf")
        self.assertEqual(
            company["include"]["files"], "/company/services/supervisor/*.conf"
        )
        for section in company.sections():
            if section.startswith("program:"):
                self.assertNotIn("user", company[section])

        launcher = (HERE / "start-company-supervisor.sh").read_text()
        self.assertIn("expected_uid=2000", launcher)
        self.assertIn("expected_gid=2000", launcher)
        image = (HERE / "Dockerfile").read_text()
        self.assertIn("useradd --uid 2000 --gid 2000", image)

    def test_company_validator_accepts_programs_and_rejects_user_selection(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            directory = Path(raw_directory)
            config = directory / "service.conf"
            config.write_text(
                "[program:preview]\n"
                "command=/usr/bin/python3 -m http.server 4321\n"
                "autorestart=true\n"
            )
            VALIDATOR.validate(
                directory,
                expected_directory=directory,
                expected_uid=os.getuid(),
            )

            config.write_text(
                "[program:preview]\ncommand=/bin/true\nuser=root\n"
            )
            with self.assertRaisesRegex(SystemExit, "may not select a Unix user"):
                VALIDATOR.validate(
                    directory,
                    expected_directory=directory,
                    expected_uid=os.getuid(),
                )

    def test_company_validator_rejects_control_sections_and_parse_dos(self) -> None:
        with tempfile.TemporaryDirectory() as raw_directory:
            directory = Path(raw_directory)
            config = directory / "service.conf"
            config.write_text("[unix_http_server]\nfile=/tmp/forged.sock\n")
            with self.assertRaisesRegex(SystemExit, "unsupported section"):
                VALIDATOR.validate(
                    directory,
                    expected_directory=directory,
                    expected_uid=os.getuid(),
                )

            config.write_bytes(b"x" * (VALIDATOR.MAX_CONFIG_BYTES + 1))
            with self.assertRaisesRegex(SystemExit, "exceeds .* bytes"):
                VALIDATOR.validate(
                    directory,
                    expected_directory=directory,
                    expected_uid=os.getuid(),
                )

            config.unlink()
            for index in range(VALIDATOR.MAX_CONFIG_FILES + 1):
                (directory / f"service-{index}.conf").write_text(
                    f"[program:service-{index}]\ncommand=/bin/true\n"
                )
            with self.assertRaisesRegex(SystemExit, "exceeds .* files"):
                VALIDATOR.validate(
                    directory,
                    expected_directory=directory,
                    expected_uid=os.getuid(),
                )

    def test_bridge_secret_and_trusted_control_state_cannot_use_company_volume(self) -> None:
        entrypoint = (HERE / "entrypoint.sh").read_text()
        self.assertIn("/company|/company/*)", entrypoint)
        self.assertIn("Runtime bridge capability must not live", entrypoint)
        self.assertIn("/run/restless/trusted-supervisor", entrypoint)


if __name__ == "__main__":
    unittest.main()
