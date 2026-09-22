#!/usr/bin/env python3
"""First-run provider selection must be explicit and safe to serialize."""
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parent.parent


class DevelopmentCompanyTest(unittest.TestCase):
    def config(self, model=None, credential=None):
        env = dict(os.environ)
        for key in ('RESTLESS_DEV_MODEL', 'RESTLESS_DEV_CREDENTIAL_REFERENCE'):
            env.pop(key, None)
        if model is not None:
            env['RESTLESS_DEV_MODEL'] = model
        if credential is not None:
            env['RESTLESS_DEV_CREDENTIAL_REFERENCE'] = credential
        return subprocess.run(
            ['bash', '-c', 'source "$1"; dev_company_write bootstrap_test',
             'test', str(ROOT / 'scripts/lib/dev-company.sh')],
            env=env, text=True, capture_output=True, check=False)

    def test_model_required_before_any_config_is_written(self):
        for model in (None, '', 'glm-5.3', 'anthropic/model\nspend=999', 'x/"quoted"'):
            with self.subTest(model=model):
                result = self.config(model, 'env:UNRELATED_KEY')
                self.assertNotEqual(result.returncode, 0)
                self.assertEqual(result.stdout, '')
                self.assertIn('Choose a model', result.stderr)

    def test_selected_provider_and_reference_survive_serialization(self):
        for model in ('anthropic/claude-sonnet-4-6', 'openai/gpt-5.6-sol',
                      'openrouter/anthropic/claude-sonnet-4.6', 'zai/glm-5.3'):
            with self.subTest(model=model):
                result = self.config(model, 'env:CHOSEN_API_KEY')
                self.assertEqual(result.returncode, 0, result.stderr)
                config = tomllib.loads(result.stdout)
                self.assertEqual(config['model'], model)
                self.assertEqual(config['credentials'], {'model.inference': 'env:CHOSEN_API_KEY'})

    def test_native_setup_has_no_invented_credential(self):
        result = self.config('openai/gpt-5.6-sol')
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertNotIn('credentials', tomllib.loads(result.stdout))
        self.assertNotIn('ZAI', result.stdout)

    def test_first_run_can_wait_for_a_connection_without_selecting_a_vendor(self):
        result = self.config()
        self.assertEqual(result.returncode, 0, result.stderr)
        config = tomllib.loads(result.stdout)
        self.assertEqual(config['model'], 'unconfigured/pending')
        self.assertNotIn('credentials', config)

    def test_credential_reference_cannot_inject_toml(self):
        value = 'infisical:/company/a\\b"c'
        result = self.config('anthropic/claude-sonnet-4-6', value)
        self.assertEqual(tomllib.loads(result.stdout)['credentials']['model.inference'], value)
        self.assertNotEqual(self.config('anthropic/claude-sonnet-4-6', 'env:KEY\nmodel="other"').returncode, 0)

    def test_launcher_rejects_invalid_model_before_creating_state(self):
        with tempfile.TemporaryDirectory(prefix='restless-init-test-') as directory:
            state = Path(directory) / 'state'
            env = dict(os.environ, RESTLESS_HOME=str(state))
            env['RESTLESS_DEV_MODEL'] = 'missing-provider-prefix'
            result = subprocess.run(['bash', str(ROOT / 'scripts/restless-dev'), 'bootstrap_test'],
                                    env=env, text=True, capture_output=True, check=False)
            self.assertNotEqual(result.returncode, 0)
            self.assertIn('Choose a model', result.stderr)
            self.assertNotIn('PASS', result.stdout)
            self.assertFalse(state.exists())

    def test_invalid_namespace_fails_before_building_or_provisioning(self):
        with tempfile.TemporaryDirectory(prefix='restless-namespace-test-') as directory:
            state = Path(directory) / 'state'
            env = dict(os.environ, RESTLESS_HOME=str(state),
                       RESTLESS_RESOURCE_NAMESPACE='namespace_that_is_too_long_test')
            result = subprocess.run(['bash', str(ROOT / 'scripts/restless-dev'), 'bootstrap_test'],
                                    env=env, text=True, capture_output=True, check=False)
            self.assertEqual(result.returncode, 2)
            self.assertIn('1..24', result.stderr)
            self.assertNotIn('Building', result.stdout)
            self.assertFalse(state.exists())

    def test_invalid_port_profile_fails_before_building_or_provisioning(self):
        with tempfile.TemporaryDirectory(prefix='restless-offset-test-') as directory:
            state = Path(directory) / 'state'
            env = dict(os.environ, RESTLESS_HOME=str(state), RESTLESS_PORT_OFFSET='29000')
            result = subprocess.run(['bash', str(ROOT / 'scripts/restless-dev'), 'bootstrap_test'],
                                    env=env, text=True, capture_output=True, check=False)
            self.assertEqual(result.returncode, 2)
            self.assertIn('1000..19999', result.stderr)
            self.assertNotIn('Building', result.stdout)
            self.assertFalse(state.exists())

    def test_existing_company_keeps_configuration_without_model_environment(self):
        with tempfile.TemporaryDirectory(prefix='restless-existing-test-') as directory:
            root = Path(directory)
            company = root / 'state/companies/existing_test.toml'
            company.parent.mkdir(parents=True)
            saved = 'name = "existing_test"\nmodel = "custom/saved-model"\n'
            company.write_text(saved)
            commands = root / 'bin'
            commands.mkdir()
            for name, body in {
                'cargo': 'exit 37',  # Stop at the first build; never start infrastructure.
                'docker': '[ "$1" = info ] && exit 0; [ "$1 $2" = "compose version" ] && exit 0; exit 99',
                'npm': 'exit 99',
                'df': 'echo "Filesystem 1024-blocks Used Available Capacity Mounted"; echo "test 100000000 0 100000000 0% /"',
            }.items():
                command = commands / name
                command.write_text('#!/bin/sh\n' + body + '\n')
                command.chmod(0o755)
            env = dict(os.environ, RESTLESS_HOME=str(root / 'state'),
                       PATH=str(commands) + os.pathsep + os.environ['PATH'])
            env.pop('RESTLESS_DEV_MODEL', None)
            env.pop('RESTLESS_DEV_CREDENTIAL_REFERENCE', None)
            result = subprocess.run(['bash', str(ROOT / 'scripts/restless-dev'), 'existing_test'],
                                    env=env, text=True, capture_output=True, check=False)
            self.assertEqual(result.returncode, 37, result.stderr)
            self.assertEqual(company.read_text(), saved)


if __name__ == '__main__':
    unittest.main()
