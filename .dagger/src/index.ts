/** The shared local/CI implementation of Core qualification and construction. */
import { availableParallelism } from 'node:os';
import { dag, Container, Directory, Platform, Secret, argument, object, func } from '@dagger.io/dagger';

import { verifyRuntimeToolsImage, verifyCompanyRuntimeImage, verifyNativeDocumentsImage, verifyAccountPlaneImage } from './verify.js';
import { publishImage, node, timed, timingsFile, verifyBuildInputs, verifyImageInspection, type Timing } from './publish.js';
import { sealRelease } from './release.js';
import { library, publishLibrary, reuseLibrary, sealLibrary } from './libraries.js';

const NODE_IMAGE = 'node:24.18.1-alpine3.23@sha256:c2cc26d8f991c2db236ad51a61efee843c482372d6d22570787309d511694110';
const RUST_IMAGE = 'rust:1-bookworm@sha256:59037199c44290f2befcdd58dcc540164763fc296950255aaefeef096a1866b0';
const EXCLUDES = ['**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/target/**', '**/__pycache__/**', '**/.git/**', '**/.env', '**/.env.*'];
const ACCOUNT_INPUTS = ['infra/account-plane/Dockerfile', 'Cargo.toml', 'Cargo.lock', 'crates/**',
  'tools/codex-runner/**', 'tools/custom-harness/**', 'tools/harness-auth/**', 'tools/schedule-test-proxy.py',
  'docs/COMPANY_OPERATING_RULES.md', 'services/native-sheets/package.json', 'services/native-sheets/package-lock.json',
  'services/native-sheets/src/**', 'services/native-sheets/NOTICE'];
const RUNTIME_INPUTS = ['infra/company-image/**', 'Cargo.toml', 'Cargo.lock', 'crates/**',
  'tools/scenario/restless-scenario.mjs', 'tools/web-review/restless-web-review.mjs', 'tools/codex-runner/**'];
const RUST_INPUTS = ['infra/rust-binaries/Dockerfile', 'Cargo.toml', 'Cargo.lock', 'crates/**',
  'tools/codex-runner/**', 'tools/custom-harness/**', 'tools/harness-auth/**', 'tools/schedule-test-proxy.py',
  'docs/COMPANY_OPERATING_RULES.md'];
// One Rust compile at a time now uses every x64 builder thread: a release builds its binaries once
// (rustBinaries) instead of two image builds sharing the machine. arm64 keeps the old bound.
const cargoJobs = (platform: string = 'linux/amd64') => platform === 'linux/amd64' ? String(availableParallelism()) : '2';

function project(source: Directory, path: string, base: Container = dag.container().from(NODE_IMAGE)): Container {
  const files = source.directory(path);
  return base
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
    .withEnvVariable('CARGO_BUILD_JOBS', cargoJobs()).withEnvVariable('CARGO_INCREMENTAL', '0')
    .withMountedCache('/usr/local/cargo/registry', dag.cacheVolume('restless-core-cargo-registry-v1'))
    .withMountedCache('/usr/local/cargo/git', dag.cacheVolume('restless-core-cargo-git-v1'))
    .withMountedCache('/src/target', dag.cacheVolume('restless-core-cargo-check-v1'))
    .withDirectory('/src', source, { include: ['Cargo.toml', 'Cargo.lock', 'crates/**', 'contracts/**', 'docs/COMPANY_OPERATING_RULES.md',
      'docs/sprints/sprint-36/contract/v1/*.json',
      'tools/codex-runner/**', 'tools/custom-harness/**', 'tools/harness-auth/**', 'tools/schedule-test-proxy.py'], exclude: EXCLUDES })
    .withWorkdir('/src');
}

function checkPlatform(platform: string): asserts platform is Platform {
  if (!['linux/amd64', 'linux/arm64'].includes(platform)) throw new Error('Core artifacts support linux/amd64 and linux/arm64');
}

@object()
export class RestlessCore {
  /** One bounded local/CI delivery call for the complete product library set. */
  @func()
  async deliverLibraries(
    @argument({ ignore: ['**', '!LICENSE', '!web/**', '!services/identity/**', '!scripts/release/**',
      '!dagger.json', '!.dagger/package.json', '!.dagger/tsconfig.json', '!.dagger/src/index.ts',
      '!.dagger/src/libraries.ts', '!.dagger/src/publish.ts', '!.github/workflows/ui-artifact-release.yml',
      '**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/.git/**', '**/.env', '**/.env.*'] })
    source: Directory, revision: string, epoch: string, scanPeriod: string, username: string, password: Secret,
    oidcRequestUrl: string, oidcRequestToken: Secret, workflowRef: string, publish: boolean = true, concurrency: number = 2,
  ): Promise<Directory> {
    if (!/^[0-9a-f]{40}$/.test(revision) || !/^\d+$/.test(epoch)) throw new Error('delivery requires exact checkout revision/epoch');
    if (!Number.isInteger(concurrency) || concurrency < 1 || concurrency > 3) throw new Error('library concurrency must be between 1 and 3');
    await this.verifyLibraryInputs(source).sync();
    const pending = ['ui', 'office', 'issuer'];
    const results: Array<{kind: string; payload: Directory; receipt: any}> = [];
    await Promise.all(Array.from({length: concurrency}, async () => {
      for (;;) {
        const kind = pending.shift();
        if (!kind) break;
        if (!publish) {
          const payload = await this.library(source, kind, revision, epoch, scanPeriod);
          results.push({kind, payload, receipt: {component: kind, source_revision: revision, qualification_only: true}});
          continue;
        }
        let payload = await this.publishLibrary(source, kind, revision, epoch, scanPeriod, username, password);
        const receipt = JSON.parse(await payload.file('library-receipt.json').contents());
        // Reuse already admitted the signed OCI payload and exact input/day.
        // It performs no mutation or duplicate signing of the existing artifact.
        if (!receipt.reused) payload = await this.sealLibrary(source, payload, username, password, oidcRequestUrl, oidcRequestToken, workflowRef);
        results.push({kind, payload, receipt});
      }
    }));
    results.sort((left, right) => left.kind.localeCompare(right.kind));
    return results.reduce((out, result) => out.withDirectory(result.kind, result.payload), dag.directory())
      .withNewFile('libraries-receipt.json', JSON.stringify({source_revision: revision, scan_period: scanPeriod,
        libraries: results.map(result => result.receipt)}, null, 2) + '\n');
  }

  /** Prove actual library dependency boundaries used for artifact reuse. */
  @func()
  verifyLibraryInputs(
    @argument({ ignore: ['**', '!LICENSE', '!web/**', '!services/identity/**', '!scripts/release/**',
      '!.dagger/**', '!dagger.json', '!.github/workflows/ui-artifact-release.yml',
      '**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/.git/**', '**/.env', '**/.env.*'] })
    source: Directory,
  ): Container {
    return dag.container().from(NODE_IMAGE).withDirectory('/src', source).withWorkdir('/src')
      .withExec(['node', '--test', 'scripts/release/library-inputs.test.mjs']);
  }

  /** Qualify a small product package independently of Runtime/account images. */
  @func()
  library(
    @argument({ ignore: ['**', '!LICENSE', '!web/**', '!services/identity/**', '!scripts/release/**',
      '!dagger.json', '!.dagger/package.json', '!.dagger/tsconfig.json', '!.dagger/src/index.ts',
      '!.dagger/src/libraries.ts', '!.dagger/src/publish.ts', '!.github/workflows/ui-artifact-release.yml',
      '**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/.git/**', '**/.env', '**/.env.*'] })
    source: Directory, kind: string, revision: string, epoch: string, scanPeriod: string,
  ): Promise<Directory> {
    return library(source, kind, revision, epoch, scanPeriod);
  }

  /** Publish a qualified product package through the existing Core OCI repository. */
  @func()
  async publishLibrary(
    @argument({ ignore: ['**', '!LICENSE', '!web/**', '!services/identity/**', '!scripts/release/**',
      '!dagger.json', '!.dagger/package.json', '!.dagger/tsconfig.json', '!.dagger/src/index.ts',
      '!.dagger/src/libraries.ts', '!.dagger/src/publish.ts', '!.github/workflows/ui-artifact-release.yml',
      '**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/.git/**', '**/.env', '**/.env.*'] })
    source: Directory, kind: string, revision: string, epoch: string, scanPeriod: string,
    username: string, password: Secret): Promise<Directory> {
    if (!/^[0-9a-f]{40}$/.test(revision) || !/^\d+$/.test(epoch)) throw new Error('publication requires exact checkout revision/epoch');
    const reused = await reuseLibrary(source, kind, scanPeriod, username, password);
    if (reused) return reused;
    const payload = await this.library(source, kind, revision, epoch, scanPeriod);
    return publishLibrary(payload, kind, revision, scanPeriod, username, password, source);
  }

  /** Sign a product package with the UI workflow's exact main identity. */
  @func()
  sealLibrary(
    @argument({ ignore: ['**', '!scripts/release/verify-library.mjs'] })
    source: Directory, artifacts: Directory, username: string, password: Secret,
    oidcRequestUrl: string, oidcRequestToken: Secret, workflowRef: string): Promise<Directory> {
    return sealLibrary(source, artifacts, username, password, oidcRequestUrl, oidcRequestToken, workflowRef);
  }

  /** Build the reusable desktop/agent tool base without importing Rust or UI. */
  @func()
  runtimeTools(
    @argument({ ignore: ['**', '!infra/company-image/Dockerfile', '!infra/company-image/gtk-settings.ini', '!infra/company-image/browser-sbom.mjs'] })
    source: Directory, platform: string = 'linux/amd64',
  ): Container {
    checkPlatform(platform);
    return dag.directory().withDirectory('/', source, {
      include: ['infra/company-image/Dockerfile', 'infra/company-image/gtk-settings.ini', 'infra/company-image/browser-sbom.mjs'],
    }).dockerBuild({ dockerfile: 'infra/company-image/Dockerfile', target: 'runtime-tools', platform })
      .withLabel('org.opencontainers.image.source', 'https://github.com/BlueprintLabIO/restless-core');
  }

  /** Exercise installed tools in the isolated builder; no appliance is mounted. */
  @func()
  async verifyRuntimeTools(
    @argument({ ignore: ['**', '!infra/company-image/Dockerfile', '!infra/company-image/gtk-settings.ini', '!infra/company-image/browser-sbom.mjs', '!infra/company-image/verify-desktop.mjs'] })
    source: Directory, platform: string = 'linux/amd64',
  ): Promise<string> {
    return verifyRuntimeToolsImage(this.runtimeTools(source, platform), source.file('infra/company-image/verify-desktop.mjs'));
  }

  /** Compile every release Rust binary once (restlessd, restless, restless-runtime-bridge) for all images. */
  @func()
  rustBinaries(
    @argument({ ignore: ['**', '!infra/rust-binaries/Dockerfile', '!Cargo.toml', '!Cargo.lock', '!crates/**',
      '!tools/codex-runner/**', '!tools/custom-harness/**', '!tools/harness-auth/**', '!tools/schedule-test-proxy.py',
      '!docs/COMPANY_OPERATING_RULES.md', '**/target/**', '**/.env', '**/.env.*'] })
    source: Directory, revision: string, platform: string = 'linux/amd64',
  ): Directory {
    checkPlatform(platform);
    if (!/^[0-9a-f]{40}$/.test(revision)) throw new Error('release binaries require exact source provenance');
    return dag.directory().withDirectory('/', source, { include: RUST_INPUTS, exclude: EXCLUDES })
      .dockerBuild({ dockerfile: 'infra/rust-binaries/Dockerfile', platform,
        buildArgs: [{ name: 'SOURCE_REVISION', value: revision }, { name: 'CARGO_BUILD_JOBS', value: cargoJobs(platform) }] })
      .directory('/out');
  }

  /** Construct the canonical Runtime, optionally reusing an admitted tool base. */
  @func()
  async companyRuntime(
    @argument({ ignore: ['**', '!Cargo.toml', '!Cargo.lock', '!crates/**', '!infra/company-image/**',
      '!tools/scenario/restless-scenario.mjs', '!tools/web-review/restless-web-review.mjs', '!tools/codex-runner/**',
      'infra/company-image/test_supervision_contract.py', '**/node_modules/**', '**/target/**', '**/__pycache__/**', '**/.env', '**/.env.*'] })
    source: Directory, revision: string, platform: string = 'linux/amd64', toolsImage: string = '', binaries?: Directory,
  ): Promise<Container> {
    checkPlatform(platform);
    if (!/^[0-9a-f]{40}$/.test(revision)) throw new Error('company Runtime requires exact source provenance');
    if (toolsImage && !/^ghcr\.io\/blueprintlabio\/restless-runtime-tools@sha256:[0-9a-f]{64}$/.test(toolsImage)) {
      throw new Error('Runtime tool base must be an exact Core OCI digest');
    }
    const [cargo, release, entry] = await Promise.all([
      source.file('Cargo.toml').contents(), source.file('crates/restless-engine/src/release.rs').contents(),
      source.file('crates/restless-engine/src/entry.rs').contents(),
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
    let context = dag.directory().withDirectory('/', source, {
      include: RUNTIME_INPUTS, exclude: [...EXCLUDES, 'infra/company-image/test_supervision_contract.py'],
    });
    if (binaries) context = context.withDirectory('prebuilt-rust', binaries);
    let image = context
      .dockerBuild({ dockerfile: 'infra/company-image/Dockerfile', platform,
        buildArgs: [{ name: 'CARGO_BUILD_JOBS', value: cargoJobs(platform) },
          { name: 'RUST_BINARIES', value: binaries ? 'prebuilt' : 'build' },
          ...(toolsImage ? [{ name: 'RUNTIME_TOOLS_IMAGE', value: toolsImage }] : [])] });
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
    source: Directory, revision: string, platform: string = 'linux/amd64', toolsImage: string = '', binaries?: Directory,
  ): Promise<string> {
    return verifyCompanyRuntimeImage(await this.companyRuntime(source, revision, platform, toolsImage, binaries), revision);
  }

  /** Run the current Core checks through the same functions used for builds. */
  @func()
  async qualify(
    @argument({ ignore: ['**/.git', '**/.git/**', '**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/target/**', '**/.env', '**/.env.*'] })
    source: Directory,
  ): Promise<string> {
    const timings: Timing[] = [];
    await Promise.all([
      timed(timings, 'rust check and boundary tests', () => rust(source).withExec(['cargo', 'check', '--workspace', '--locked'])
        .withExec(['cargo', 'test', '--locked', '-p', 'restlessd', 'company_projection::tests'])
        .withExec(['cargo', 'test', '--locked', '-p', 'restlessd', 'public_jwks_routes_are_method_and_path_exact']).sync()),
      timed(timings, 'cockpit check and build', () => cockpit(source).sync()),
      timed(timings, 'browser overlays', () => this.verifyOverlays(source).sync()),
      timed(timings, 'ui artifact consumer', () => this.verifyUiArtifact(source).sync()),
      timed(timings, 'native documents check and build', () => project(source, 'services/native-documents-collaboration')
        .withExec(['npm', 'run', 'check']).withExec(['npm', 'run', 'build']).sync()),
      timed(timings, 'native sheets tests', () => project(source, 'services/native-sheets').withExec(['npm', 'test']).sync()),
      timed(timings, 'workflow lint', () => this.verifyWorkflow(source).sync()),
      timed(timings, 'issuer artifact', () => this.issuer(source).sync()),
      timed(timings, 'release contracts', () => this.verifyRelease(source)),
    ]);
    return 'Core qualification passed: Rust workspace, projection contract and public-key boundary tests, cockpit check/build and browser overlays, clean-project UI artifact consumer, native Documents check/build, pinned native Sheets engine, issuer artifact imports, workflow lint and versioned release contracts\n'
      + `timings ${JSON.stringify(timings)}`;
  }

  /** Check workflow wiring with the same pinned tool used by Cloud. */
  @func()
  verifyWorkflow(source: Directory): Container {
    return dag.container().from('rhysd/actionlint:1.7.12@sha256:b1934ee5f1c509618f2508e6eb47ee0d3520686341fec936f3b79331f9315667')
      .withDirectory('/src/.github', source.directory('.github')).withWorkdir('/src')
      .withExec(['/bin/sh', '-ec',
        `if grep -RInE '(^|[[:space:]])("[^"]*/)?dagger"?[[:space:]]' .github/workflows | grep -v -- '--silent'; then
          printf '%s\\n' 'GitHub Actions Dagger calls must use --silent to keep runner inventory out of logs.' >&2
          exit 1
        fi`])
      .withExec(['actionlint', '-oneline', '-config-file', '.github/actionlint.yaml',
        '.github/workflows/immutable-core-release.yml', '.github/workflows/identity-image.yml',
        '.github/workflows/ui-artifact-release.yml']);
  }

  /** Build the architecture-independent cockpit once per actual UI inputs. */
  @func()
  cockpit(source: Directory): Directory {
    return cockpit(source).directory('/project/build');
  }

  /** Install and render the real packed UI in an empty project, without pretending it is a release. */
  @func()
  verifyUiArtifact(source: Directory): Container {
    return project(source, 'web').withExec(['node', 'scripts/smoke-ui-artifact.mjs', '--qualification']);
  }

  /** Run the real chrome's browser checks on example data in the isolated builder. */
  @func()
  verifyOverlays(source: Directory): Container {
    // Chromium sits below the web sources so it stays cached across commits; a stalled mirror is retried.
    const browser = dag.container().from(NODE_IMAGE).withExec(['/bin/sh', '-c',
      'for i in 1 2 3; do timeout 300 apk add --no-cache chromium && exit 0; sleep 5; done; exit 1']);
    return project(source, 'web', browser)
      .withEnvVariable('RESTLESS_BROWSER_EXECUTABLE', '/usr/bin/chromium')
      .withEnvVariable('RESTLESS_REVIEW_ORIGIN', 'http://127.0.0.1:5173')
      .withEnvVariable('RESTLESS_OVERLAY_PROOF_DIR', '/tmp/restless-overlay-proof')
      .withExec(['/bin/sh', '-ec',
        'node node_modules/vite/bin/vite.js --host 127.0.0.1 --port 5173 > /tmp/restless-overlay-vite.log 2>&1 & server_pid=$!; '
        + 'trap \'kill -TERM "$server_pid" 2>/dev/null || true; wait "$server_pid" 2>/dev/null || true\' EXIT; '
        + 'node --input-type=module -e \'let ready=false; for(let i=0;i<60;i++){try{const r=await fetch("http://127.0.0.1:5173/gallery/shell",{signal:AbortSignal.timeout(1000)}); await r.arrayBuffer(); if(r.ok){ready=true; break}}catch{} await new Promise(r=>setTimeout(r,250))} if(!ready) throw new Error("overlay fixture did not start")\' '
        + '|| { tail -60 /tmp/restless-overlay-vite.log; exit 1; }; '
        + 'node scripts/verify-overlays.mjs',
      ]);
  }

  /** Exercise the versioned release contracts before any registry mutation. */
  @func()
  async verifyRelease(source: Directory): Promise<string> {
    await verifyBuildInputs();
    await verifyImageInspection();
    await node(source).withExec(['node', '--test', 'scripts/company-collaboration-contract.test.mjs']).sync();
    return 'Core release contracts passed: real OCI image/config inspection, timestamp-independent inputs, legacy and platform-scoped manifests and signed bundle indexes';
  }

  /** Publish and qualify exact platform images, including a reusable tool base. */
  @func()
  async publish(
    @argument({ ignore: ['**/.git', '**/.git/**', '**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/target/**', '**/.env', '**/.env.*'] })
    source: Directory, revision: string, username: string, password: Secret, scanPeriod: string,
    platform: string = 'linux/amd64',
  ): Promise<Directory> {
    checkPlatform(platform);
    if (!/^[0-9a-f]{40}$/.test(revision)) throw new Error('Core publication requires exact source provenance');
    if (scanPeriod !== new Date().toISOString().slice(0, 10)) throw new Error('pass the current UTC scan day');
    const timings: Timing[] = [];
    await timed(timings, 'release contracts', () => this.verifyRelease(source));
    const toolsContext = source.filter({ include: ['infra/company-image/Dockerfile', 'infra/company-image/gtk-settings.ini', 'infra/company-image/browser-sbom.mjs'] });
    // The Rust binaries and the runtime tool base are independent: build them together.
    const binaries = this.rustBinaries(source, revision, platform);
    let [, artifacts] = await Promise.all([
      timed(timings, 'rust binaries (restlessd, restless, runtime bridge)', () => binaries.sync()),
      timed(timings, 'runtime-tools', () => publishImage('runtime-tools', revision, platform, toolsContext,
        async () => this.runtimeTools(source, platform), async image => { console.log(await verifyRuntimeToolsImage(image, source.file('infra/company-image/verify-desktop.mjs'))); },
        username, password, scanPeriod)),
    ]);
    const tools = JSON.parse(await artifacts.file('images/runtime-tools.json').contents());
    const nativeContext = source.directory('services/native-documents-collaboration').filter({
      include: ['Dockerfile', 'package.json', 'package-lock.json', 'tsconfig.json', 'tsconfig.build.json', 'src/**'], exclude: EXCLUDES,
    });
    const tasks = [
      () => publishImage('account-plane', revision, platform, source.filter({ include: [...ACCOUNT_INPUTS, 'web/**'], exclude: EXCLUDES }),
        async () => this.accountPlane(source, revision, platform, binaries), async image => { console.log(await verifyAccountPlaneImage(image)); },
        username, password, scanPeriod),
      () => publishImage('native-documents-collaboration', revision, platform, nativeContext,
        async () => this.nativeDocuments(source, revision, platform), async image => { console.log(await verifyNativeDocumentsImage(image)); },
        username, password, scanPeriod),
      () => publishImage('company-runtime', revision, platform, source.filter({
        include: RUNTIME_INPUTS, exclude: [...EXCLUDES, 'infra/company-image/test_supervision_contract.py'],
      }), async () => this.companyRuntime(source, revision, platform, tools.reference, binaries),
        async image => { console.log(await verifyCompanyRuntimeImage(image, revision)); }, username, password, scanPeriod),
    ];
    // With the Rust compiled once above, the images only assemble layers: build all three at once.
    const results = await timed(timings, 'images', () => Promise.all(tasks.map(task => task())));
    for (const result of results) artifacts = artifacts.withDirectory('/', result);
    return artifacts.withNewFile('timings/publish.json', timingsFile(timings));
  }

  /** Seal publication through the trusted Core main workflow. */
  @func()
  seal(source: Directory, artifacts: Directory, revision: string, username: string, password: Secret,
    oidcRequestUrl: string, oidcRequestToken: Secret, workflowRef: string,
    platform: string = 'linux/amd64',
  ): Promise<Directory> {
    checkPlatform(platform);
    return sealRelease(source, artifacts, revision, platform, username, password, oidcRequestUrl, oidcRequestToken, workflowRef);
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

  /** Build the canonical native Documents image using only its actual inputs. */
  @func()
  nativeDocuments(
    @argument({ ignore: ['**', '!services/native-documents-collaboration/Dockerfile',
      '!services/native-documents-collaboration/package.json', '!services/native-documents-collaboration/package-lock.json',
      '!services/native-documents-collaboration/tsconfig.json', '!services/native-documents-collaboration/tsconfig.build.json',
      '!services/native-documents-collaboration/src/**'] })
    source: Directory, revision: string, platform: string = 'linux/amd64',
  ): Container {
    checkPlatform(platform);
    if (!/^[0-9a-f]{40}$/.test(revision)) throw new Error('native Documents requires exact source provenance');
    const context = source.directory('services/native-documents-collaboration').filter({
      include: ['Dockerfile', 'package.json', 'package-lock.json', 'tsconfig.json', 'tsconfig.build.json', 'src/**'], exclude: EXCLUDES,
    });
    return context.dockerBuild({ dockerfile: 'Dockerfile', platform })
      .withLabel('org.opencontainers.image.revision', revision)
      .withLabel('org.opencontainers.image.source', 'https://github.com/BlueprintLabIO/restless-core');
  }

  /** Exercise the compiled server as its non-root image user with a store fixture. */
  @func()
  async verifyNativeDocuments(
    @argument({ ignore: ['**', '!services/native-documents-collaboration/Dockerfile',
      '!services/native-documents-collaboration/package.json', '!services/native-documents-collaboration/package-lock.json',
      '!services/native-documents-collaboration/tsconfig.json', '!services/native-documents-collaboration/tsconfig.build.json',
      '!services/native-documents-collaboration/src/**'] })
    source: Directory, revision: string, platform: string = 'linux/amd64',
  ): Promise<string> {
    return verifyNativeDocumentsImage(this.nativeDocuments(source, revision, platform));
  }

  /** Build the existing canonical account image with the shared cockpit output. */
  @func()
  accountPlane(
    @argument({ ignore: ['**', '!Cargo.toml', '!Cargo.lock', '!crates/**', '!tools/codex-runner/**',
      '!tools/custom-harness/**', '!tools/harness-auth/**', '!tools/schedule-test-proxy.py',
      '!docs/COMPANY_OPERATING_RULES.md', '!infra/account-plane/Dockerfile', '!services/native-sheets/package.json',
      '!services/native-sheets/package-lock.json', '!services/native-sheets/src/**', '!services/native-sheets/NOTICE',
      '!web/**', '**/node_modules/**', '**/.svelte-kit/**', '**/build/**', '**/dist/**', '**/target/**', '**/.env', '**/.env.*'] })
    source: Directory, revision: string, platform: string = 'linux/amd64', binaries?: Directory,
  ): Container {
    checkPlatform(platform);
    if (!/^[0-9a-f]{40}$/.test(revision)) throw new Error('account plane requires exact source provenance');
    let context = dag.directory().withDirectory('/', source, { include: ACCOUNT_INPUTS, exclude: EXCLUDES })
      .withDirectory('web/build', this.cockpit(source));
    if (binaries) context = context.withDirectory('prebuilt-rust', binaries);
    return context.dockerBuild({ dockerfile: 'infra/account-plane/Dockerfile', platform,
      buildArgs: [{ name: 'SOURCE_REVISION', value: revision }, { name: 'CARGO_BUILD_JOBS', value: cargoJobs(platform) },
        { name: 'RUST_BINARIES', value: binaries ? 'prebuilt' : 'build' }] })
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
    source: Directory, revision: string, platform: string = 'linux/amd64', binaries?: Directory,
  ): Promise<string> {
    return verifyAccountPlaneImage(this.accountPlane(source, revision, platform, binaries));
  }
}
