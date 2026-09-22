#!/usr/bin/env python3
"""Provision a real isolated vault, store a value, restart and read it back."""
import json
import os
from pathlib import Path
import socket
import subprocess
import tempfile
import urllib.parse
import urllib.request
import uuid

ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.run(args, check=True, capture_output=True, text=True, **kwargs)


with tempfile.TemporaryDirectory(prefix='restless-vault-test-') as directory:
    state = Path(directory)
    namespace = 'launchvault' + uuid.uuid4().hex[:10] + '_test'
    project = f'restless-{namespace}-vault'
    with socket.socket() as reservation:
        reservation.bind(('127.0.0.1', 0))
        port = reservation.getsockname()[1]
    env = dict(os.environ, RESTLESS_HOME=directory,
               RESTLESS_RESOURCE_NAMESPACE=namespace,
               RESTLESS_PORT_OFFSET=str(port - 7793), STACK_REPO_ROOT=str(ROOT))
    env.pop('INFISICAL_API_URL', None)
    compose = ['docker', 'compose', '--project-name', project, '--file',
               str(ROOT / 'infra/infisical/compose.yml'), '--env-file',
               str(state / 'infisical/runtime.env')]
    env['RESTLESS_INFISICAL_RUNTIME_ENV'] = str(state / 'infisical/runtime.env')
    env['RESTLESS_INFISICAL_PORT'] = str(port)

    def provision():
        run('bash', '-c', 'source "$STACK_REPO_ROOT/scripts/lib/dev-vault.sh"; dev_vault_ensure',
            env=env, timeout=400)

    def api(method, path, body=None, token=None):
        headers = {'Content-Type': 'application/json'}
        if token:
            headers['Authorization'] = 'Bearer ' + token
        request = urllib.request.Request(f'http://127.0.0.1:{port}' + path,
            data=json.dumps(body).encode() if body is not None else None,
            headers=headers, method=method)
        with urllib.request.urlopen(request, timeout=15) as response:
            return json.load(response)

    try:
        provision()
        authority = state / 'infisical/authority.env'
        assert authority.stat().st_mode & 0o777 == 0o600
        saved = authority.read_text()
        settings = dict(line.split('=', 1) for line in saved.splitlines() if '=' in line)

        def login():
            return api('POST', '/api/v1/auth/universal-auth/login', {
                'clientId': settings['INFISICAL_UNIVERSAL_AUTH_CLIENT_ID'],
                'clientSecret': settings['INFISICAL_UNIVERSAL_AUTH_CLIENT_SECRET'],
                'organizationSlug': settings['INFISICAL_ORGANIZATION_SLUG'],
            })['accessToken']

        token = login()
        location = {'projectId': settings['INFISICAL_PROJECT_ID'],
                    'environment': 'prod', 'secretPath': '/'}
        value = uuid.uuid4().hex
        api('PATCH', '/api/v4/secrets/batch', {
            **location, 'mode': 'upsert',
            'secrets': [{'secretKey': 'BOOTSTRAP_TEST', 'secretValue': value}],
        }, token)
        print('PASS fresh vault is ready for authenticated writes on return', flush=True)
        run(*compose, 'stop', env=env)
        provision()
        assert authority.read_text() == saved, 'Restart replaced the Authority identity'
        query = urllib.parse.urlencode({**location, 'type': 'shared', 'viewSecretValue': 'true'})
        assert api('GET', '/api/v4/secrets/BOOTSTRAP_TEST?' + query,
                   token=login())['secret']['secretValue'] == value
        print('PASS restart preserves the Authority connection and stored value', flush=True)
    finally:
        run(*compose, 'down', '--volumes', env=env)
        for kind, field in [('container', 'ID'), ('volume', 'Name'), ('network', 'ID')]:
            remaining = run('docker', kind, 'ls', '--filter',
                f'label=com.docker.compose.project={project}', '--format', '{{.' + field + '}}')
            assert not remaining.stdout.strip(), f'{kind} cleanup incomplete'
        print('PASS disposable vault containers, volumes and network removed', flush=True)
