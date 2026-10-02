/** The shared local/CI implementation of Core qualification and construction. */
import { dag, Container, Directory, Platform, argument, object, func } from '@dagger.io/dagger';

const NODE_IMAGE = 'node:24.18.1-alpine3.23@sha256:c2cc26d8f991c2db236ad51a61efee843c482372d6d22570787309d511694110';
const RUST_IMAGE = 'rust:1-bookworm@sha256:59037199c44290f2befcdd58dcc540164763fc296950255aaefeef096a1866b0';
const EXCLUDES = ['**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/target/**', '**/.git/**', '**/.env', '**/.env.*'];
const ACCOUNT_INPUTS = ['infra/account-plane/Dockerfile', 'Cargo.toml', 'Cargo.lock', 'crates/**',
  'tools/codex-runner/**', 'tools/custom-harness/**', 'tools/harness-auth/**', 'tools/schedule-test-proxy.py',
  'docs/COMPANY_OPERATING_RULES.md', 'services/native-sheets/package.json', 'services/native-sheets/package-lock.json',
  'services/native-sheets/src/**', 'services/native-sheets/NOTICE'];

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
