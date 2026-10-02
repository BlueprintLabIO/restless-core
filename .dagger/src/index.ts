/** The shared local/CI implementation of Core qualification and construction. */
import { dag, Container, Directory, Platform, argument, object, func } from '@dagger.io/dagger';

const NODE_IMAGE = 'node:24.18.1-alpine3.23@sha256:c2cc26d8f991c2db236ad51a61efee843c482372d6d22570787309d511694110';
const RUST_IMAGE = 'rust:1-bookworm@sha256:59037199c44290f2befcdd58dcc540164763fc296950255aaefeef096a1866b0';
const EXCLUDES = ['**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/target/**', '**/__pycache__/**', '**/.git/**', '**/.env', '**/.env.*'];
const ACCOUNT_INPUTS = ['infra/account-plane/Dockerfile', 'Cargo.toml', 'Cargo.lock', 'crates/**',
  'tools/codex-runner/**', 'tools/custom-harness/**', 'tools/harness-auth/**', 'tools/schedule-test-proxy.py',
  'docs/COMPANY_OPERATING_RULES.md', 'services/native-sheets/package.json', 'services/native-sheets/package-lock.json',
  'services/native-sheets/src/**', 'services/native-sheets/NOTICE'];
const RUNTIME_INPUTS = ['infra/company-image/**', 'Cargo.toml', 'Cargo.lock', 'crates/**',
  'tools/scenario/restless-scenario.mjs', 'tools/web-review/restless-web-review.mjs', 'tools/codex-runner/**'];

function project(source: Directory, path: string): Container {
  const files = source.directory(path);
  return dag.container().from(NODE_IMAGE)
    .withMountedCache('/root/.npm', dag.cacheVolume('restless-core-npm-v1'))
    .withFile('/project/package.json', files.file('package.json'))
    .withFile('/project/package-lock.json', files.file('package-lock.json'))
    .withWorkdir('/project').withExec(['npm', 'ci', '--no-audit', '--no-fund'])
    .withDirectory('/project', files, { exclude: EXCLUDES });
}

function cockpit(source: Directory): Container {
  return project(source, 'web').withExec(['npm', 'run', 'check']).withExec(['npm', 'run', 'build']);
}

function rust(source: Directory): Container {
  return dag.container().from(RUST_IMAGE)
    .withEnvVariable('CARGO_BUILD_JOBS', '2').withEnvVariable('CARGO_INCREMENTAL', '0')
    .withMountedCache('/usr/local/cargo/registry', dag.cacheVolume('restless-core-cargo-registry-v1'))
    .withMountedCache('/usr/local/cargo/git', dag.cacheVolume('restless-core-cargo-git-v1'))
    .withMountedCache('/src/target', dag.cacheVolume('restless-core-cargo-check-v1'))
    .withDirectory('/src', source, { include: ['Cargo.toml', 'Cargo.lock', 'crates/**', 'docs/COMPANY_OPERATING_RULES.md',
      'tools/codex-runner/**', 'tools/custom-harness/**', 'tools/harness-auth/**', 'tools/schedule-test-proxy.py'], exclude: EXCLUDES })
    .withWorkdir('/src');
}

function checkPlatform(platform: string): asserts platform is Platform {
  if (!['linux/amd64', 'linux/arm64'].includes(platform)) throw new Error('Core artifacts support linux/amd64 and linux/arm64');
}

@object()
export class RestlessCore {
  /** Build the reusable desktop/agent tool base without importing Rust or UI. */
  @func()
  runtimeTools(
    @argument({ ignore: ['**', '!infra/company-image/Dockerfile', '!infra/company-image/gtk-settings.ini'] })
    source: Directory, platform: string = 'linux/amd64',
  ): Container {
    checkPlatform(platform);
    return dag.directory().withDirectory('/', source, {
      include: ['infra/company-image/Dockerfile', 'infra/company-image/gtk-settings.ini'],
    }).dockerBuild({ dockerfile: 'infra/company-image/Dockerfile', target: 'runtime-tools', platform })
      .withLabel('org.opencontainers.image.source', 'https://github.com/BlueprintLabIO/restless-core');
  }

  /** Exercise installed tools in the isolated builder; no appliance is mounted. */
  @func()
  async verifyRuntimeTools(
    @argument({ ignore: ['**', '!infra/company-image/Dockerfile', '!infra/company-image/gtk-settings.ini'] })
    source: Directory, platform: string = 'linux/amd64',
  ): Promise<string> {
    return this.runtimeTools(source, platform).withExec(['/bin/sh', '-ec',
      'test ! -e /usr/local/bin/restless; test ! -e /usr/local/bin/restless-runtime-bridge; '
      + 'node --version; codex --version; omp --version; pnpm --version; '
      + 'godot --headless --version; chromium --version; '
      + 'node -e \'if (typeof require("/usr/local/lib/node_modules/ws") !== "function") throw new Error("ws is unavailable")\'; '
      + 'test "$(id -u company)" = 2000; test "$(id -u effect)" = 2001; '
      + 'test -s /opt/restless/godot/export_templates/4.7.2.stable/windows_release_x86_64.exe; '
      + 'printf "Runtime tool base is usable without Core binaries\\n"',
    ], { useEntrypoint: false }).stdout();
  }

  /** Construct the canonical Runtime, optionally reusing an admitted tool base. */
  @func()
  async companyRuntime(
    @argument({ ignore: ['**', '!Cargo.toml', '!Cargo.lock', '!crates/**', '!infra/company-image/**',
      '!tools/scenario/restless-scenario.mjs', '!tools/web-review/restless-web-review.mjs', '!tools/codex-runner/**',
      'infra/company-image/test_supervision_contract.py', '**/node_modules/**', '**/target/**', '**/__pycache__/**', '**/.env', '**/.env.*'] })
    source: Directory, revision: string, platform: string = 'linux/amd64', toolsImage: string = '',
  ): Promise<Container> {
    checkPlatform(platform);
    if (!/^[0-9a-f]{40}$/.test(revision)) throw new Error('company Runtime requires exact source provenance');
    if (toolsImage && !/^ghcr\.io\/blueprintlabio\/restless-runtime-tools@sha256:[0-9a-f]{64}$/.test(toolsImage)) {
      throw new Error('Runtime tool base must be an exact Core OCI digest');
    }
    const [cargo, release, entry] = await Promise.all([
      source.file('Cargo.toml').contents(), source.file('crates/restlessd/src/release.rs').contents(),
      source.file('crates/restlessd/src/entry.rs').contents(),
    ]);
    const values: [string, string | undefined][] = [
      ['RESTLESS_CORE_VERSION', cargo.match(/^version = "([^"]+)"/m)?.[1]],
      ['RESTLESS_API_CONTRACT_VERSION', release.match(/API_CONTRACT_VERSION: u32 = (\d+);/)?.[1]],
      ['RESTLESS_SCHEMA_VERSION', release.match(/SCHEMA_VERSION: u32 = (\d+);/)?.[1]],
      ['RESTLESS_ASSERTION_CONTRACT_VERSION', entry.match(/ASSERTION_CONTRACT_VERSION: u32 = (\d+);/)?.[1]],
      ['RESTLESS_SOURCE_REVISION', revision],
    ];
    // Composition metadata is configuration, not a toolchain or compiler input.
    // Keep it outside Dockerfile translation so a new revision reuses the image.
    let image = dag.directory().withDirectory('/', source, {
      include: RUNTIME_INPUTS, exclude: [...EXCLUDES, 'infra/company-image/test_supervision_contract.py'],
    })
      .dockerBuild({ dockerfile: 'infra/company-image/Dockerfile', platform,
        buildArgs: toolsImage ? [{ name: 'RUNTIME_TOOLS_IMAGE', value: toolsImage }] : [] });
    for (const [name, value] of values) {
      if (!value) throw new Error(`missing canonical release value: ${name}`);
      image = image.withEnvVariable(name, value);
    }
    return image
      .withLabel('org.opencontainers.image.revision', revision)
      .withLabel('org.opencontainers.image.source', 'https://github.com/BlueprintLabIO/restless-core');
  }

  /** Check the Runtime artifact's binary linkage and exact release metadata. */
  @func()
  async verifyCompanyRuntime(
    @argument({ ignore: ['**', '!Cargo.toml', '!Cargo.lock', '!crates/**', '!infra/company-image/**',
      '!tools/scenario/restless-scenario.mjs', '!tools/web-review/restless-web-review.mjs', '!tools/codex-runner/**',
      'infra/company-image/test_supervision_contract.py', '**/node_modules/**', '**/target/**', '**/__pycache__/**', '**/.env', '**/.env.*'] })
    source: Directory, revision: string, platform: string = 'linux/amd64', toolsImage: string = '',
  ): Promise<string> {
    const image = await this.companyRuntime(source, revision, platform, toolsImage);
    const checked = image.withEnvVariable('EXPECTED_REVISION', revision).withExec(['/bin/sh', '-ec',
      'restless --help; links="$(ldd /usr/local/bin/restless-runtime-bridge)"; '
      + 'printf "%s\\n" "$links"; ! printf "%s" "$links" | grep -q "not found"; '
      + 'test "$RESTLESS_SOURCE_REVISION" = "$EXPECTED_REVISION"; '
      + 'test -x /usr/local/bin/company-init; test -x /usr/local/bin/start-runtime-bridge; '
      + 'test -s /opt/restless/skills/scenario-evidence/SKILL.md; '
      + 'printf "Company Runtime binaries link and release metadata matches\\n"',
    ], { useEntrypoint: false });
    // Exercise the immutable launcher's actual uid/capability drop. The small
    // supervisor fixture avoids starting desktop services or a company daemon.
    return checked.withNewFile('/tmp/restless-privilege-probe',
      '#!/bin/sh\nset -eu\n/usr/local/bin/assert-runtime-privileges company\n'
      + 'printf passed > /tmp/restless-supervisor-probe/passed\n', { permissions: 0o555 })
      .withNewFile('/etc/supervisor/conf.d/company.conf',
        '[supervisord]\nnodaemon=true\nlogfile=/tmp/restless-supervisor-probe/supervisor.log\n'
        + 'logfile_maxbytes=0\npidfile=/tmp/restless-supervisor-probe/supervisor.pid\n'
        + 'childlogdir=/tmp/restless-supervisor-probe\n[program:privilege-probe]\n'
        + 'command=/tmp/restless-privilege-probe\nautostart=true\nautorestart=false\nstartsecs=0\n'
        + 'stdout_logfile=/tmp/restless-supervisor-probe/child.log\nstdout_logfile_maxbytes=0\nredirect_stderr=true\n')
      .withExec(['/bin/sh', '-ec',
        'mkdir -p /company/services/supervisor /tmp/restless-supervisor-probe; '
        + 'chown -R 2000:2000 /company /tmp/restless-supervisor-probe; '
        + '/usr/local/bin/start-company-supervisor > /tmp/restless-supervisor-probe/launcher.log 2>&1 & supervisor_pid=$!; '
        + 'trap \'kill -TERM "$supervisor_pid" 2>/dev/null || true; wait "$supervisor_pid" 2>/dev/null || true\' EXIT; '
        + 'for attempt in $(seq 1 100); do '
        + 'if test -f /tmp/restless-supervisor-probe/passed; then '
        + 'test "$(stat -c %u /tmp/restless-supervisor-probe/passed)" = 2000; '
        + 'printf "Company supervisor child has uid/gid 2000, no groups/capabilities, and NoNewPrivs\\n"; exit 0; fi; '
        + 'sleep 0.1; done; cat /tmp/restless-supervisor-probe/launcher.log; '
        + 'cat /tmp/restless-supervisor-probe/child.log 2>/dev/null || true; exit 1',
      ], { useEntrypoint: false }).stdout();
  }

  /** Run the current Core checks through the same functions used for builds. */
  @func()
  async qualify(
    @argument({ ignore: ['**/.git', '**/.git/**', '**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/target/**', '**/.env', '**/.env.*'] })
    source: Directory,
  ): Promise<string> {
    await Promise.all([
      rust(source).withExec(['cargo', 'check', '--workspace', '--locked']).sync(),
      cockpit(source).sync(),
      project(source, 'services/native-documents-collaboration')
        .withExec(['npm', 'run', 'check']).withExec(['npm', 'run', 'build']).sync(),
      project(source, 'services/native-sheets').withExec(['npm', 'test']).sync(),
      this.verifyWorkflow(source).sync(),
      this.issuer(source).sync(),
    ]);
    return 'Core qualification passed: Rust workspace, cockpit check/build, native Documents check/build, pinned native Sheets engine, issuer artifact imports, workflow lint';
  }

  /** Check workflow wiring with the same pinned tool used by Cloud. */
  @func()
  verifyWorkflow(source: Directory): Container {
    return dag.container().from('rhysd/actionlint:1.7.12@sha256:b1934ee5f1c509618f2508e6eb47ee0d3520686341fec936f3b79331f9315667')
      .withDirectory('/src/.github', source.directory('.github')).withWorkdir('/src')
      .withExec(['actionlint', '-oneline', '-config-file', '.github/actionlint.yaml',
        '.github/workflows/immutable-core-release.yml', '.github/workflows/identity-image.yml']);
  }

  /** Build the architecture-independent cockpit once per actual UI inputs. */
  @func()
  cockpit(source: Directory): Directory {
    return cockpit(source).directory('/project/build');
  }

  /** Exercise and export the bounded workbook engine independently of Rust. */
  @func()
  nativeSheets(source: Directory): Container {
    return project(source, 'services/native-sheets').withExec(['npm', 'test']);
  }

  /** Export the Core-owned issuer package without an entire identity image. */
  @func()
  issuer(source: Directory): Directory {
    const identity = source.directory('services/identity');
    const files = dag.directory().withDirectory('/', identity.directory('library'))
      .withFile('issuer.mjs', identity.file('src/issuer.mjs'))
      .withFile('membership-sql.mjs', identity.file('src/membership-sql.mjs'))
      .withFile('core-request.mjs', identity.file('src/core-request.mjs'));
    return dag.container().from(NODE_IMAGE)
      .withMountedCache('/root/.npm', dag.cacheVolume('restless-core-npm-v1'))
      .withDirectory('/issuer', files).withWorkdir('/issuer')
      .withExec(['npm', 'install', '--ignore-scripts', '--no-package-lock', '--no-audit', '--no-fund'])
      .withExec(['node', '--input-type=module', '-e',
        'import assert from "node:assert/strict"; '
        + 'import { createIssuer } from "@restless/issuer"; '
        + 'import { createSqlMembershipStore } from "@restless/issuer/membership-sql"; '
        + 'import { coreRequest } from "@restless/issuer/core-request"; '
        + 'for (const value of [createIssuer, createSqlMembershipStore, coreRequest]) assert.equal(typeof value, "function"); '
        + 'console.log("Core issuer artifact imports successfully")',
      ]).directory('/issuer').withoutDirectory('node_modules');
  }

  /** Build the existing canonical account image with the shared cockpit output. */
  @func()
  accountPlane(
    @argument({ ignore: ['**', '!Cargo.toml', '!Cargo.lock', '!crates/**', '!tools/codex-runner/**',
      '!tools/custom-harness/**', '!tools/harness-auth/**', '!tools/schedule-test-proxy.py',
      '!docs/COMPANY_OPERATING_RULES.md', '!infra/account-plane/Dockerfile', '!services/native-sheets/package.json',
      '!services/native-sheets/package-lock.json', '!services/native-sheets/src/**', '!services/native-sheets/NOTICE',
      '!web/**', '**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/target/**', '**/.env', '**/.env.*'] })
    source: Directory, revision: string, platform: string = 'linux/amd64',
  ): Container {
    checkPlatform(platform);
    if (!/^[0-9a-f]{40}$/.test(revision)) throw new Error('account plane requires exact source provenance');
    const context = dag.directory().withDirectory('/', source, { include: ACCOUNT_INPUTS, exclude: EXCLUDES })
      .withDirectory('web/build', this.cockpit(source));
    return context.dockerBuild({ dockerfile: 'infra/account-plane/Dockerfile', platform,
      buildArgs: [{ name: 'SOURCE_REVISION', value: revision }] })
      .withLabel('org.opencontainers.image.revision', revision)
      .withLabel('org.opencontainers.image.source', 'https://github.com/BlueprintLabIO/restless-core');
  }

  /** Exercise the built artifact without connecting to an appliance or company. */
  @func()
  async verifyAccountPlane(
    @argument({ ignore: ['**', '!Cargo.toml', '!Cargo.lock', '!crates/**', '!tools/codex-runner/**',
      '!tools/custom-harness/**', '!tools/harness-auth/**', '!tools/schedule-test-proxy.py',
      '!docs/COMPANY_OPERATING_RULES.md', '!infra/account-plane/Dockerfile', '!services/native-sheets/package.json',
      '!services/native-sheets/package-lock.json', '!services/native-sheets/src/**', '!services/native-sheets/NOTICE',
      '!web/**', '**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/target/**', '**/.env', '**/.env.*'] })
    source: Directory, revision: string, platform: string = 'linux/amd64',
  ): Promise<string> {
    return this.accountPlane(source, revision, platform).withExec(['/bin/sh', '-ec',
      '/usr/local/bin/restlessd --help; test -s /opt/restless/cockpit/index.html; '
      + 'node --input-type=module -e \'const { Model } = await import("/opt/restless/native-sheets/src/upstream.mjs"); '
      + 'if (typeof Model !== "function") throw new Error("Native Sheets engine is unavailable"); '
      + 'console.log("Account-plane binary links; bundled cockpit and native Sheets engine are usable")\'',
    ], { useEntrypoint: false }).stdout();
  }
}
