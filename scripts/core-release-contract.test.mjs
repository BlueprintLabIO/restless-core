import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import test from "node:test";
import {
  CORE_RELEASE_CONTRACT,
  coreReleaseSha256,
  createCoreReleaseManifest,
  validCoreReleaseManifest,
} from "./core-release-contract.mjs";
import {
  OWNER_PLANE_COMPOSE_PATH,
  OWNER_PLANE_TEMPLATE_TOKENS,
  assertOwnerPlaneComposeTemplate,
  ownerPlaneComposeSha256,
  validOwnerPlaneComposeTemplate,
} from "./owner-plane-compose-contract.mjs";

const repository = resolve(import.meta.dirname, "..");
const digest = (character) => `sha256:${character.repeat(64)}`;
const image = (name, character) =>
  `ghcr.io/blueprintlabio/${name}@${digest(character)}`;
const input = {
  sourceRevision: "1".repeat(40),
  coreVersion: "0.4.2",
  uiCarrierImage: image("restless-core-ui", "2"),
  accountPlaneImage: image("restless-core-account-plane", "3"),
  companyRuntimeImage: image("restless-core-company-runtime", "4"),
  uiArtifactDigest: digest("5"),
  uiPayloadDigest: digest("6"),
  uiRouteManifestDigest: digest("7"),
  uiRouteCount: 15,
  productContractVersion: 1,
  capabilityContractVersion: 1,
  apiContractVersion: 1,
  identityAssertionContractVersion: 1,
  schemaVersion: 34,
  ownerPlaneComposePath: OWNER_PLANE_COMPOSE_PATH,
  ownerPlaneComposeSha256: "8".repeat(64),
};

test("Core release manifest binds three images, UI identities and source-owned contracts", () => {
  const manifest = createCoreReleaseManifest(input);
  assert.equal(manifest.contract, CORE_RELEASE_CONTRACT);
  assert.equal(validCoreReleaseManifest(manifest), true);
  assert.match(coreReleaseSha256(manifest), /^sha256:[a-f0-9]{64}$/);
  assert.deepEqual(Object.keys(manifest.images).sort(), [
    "accountPlane",
    "companyRuntime",
    "uiCarrier",
  ]);
  assert.deepEqual(manifest.contracts, {
    product: 1,
    capability: 1,
    api: 1,
    identityAssertion: 1,
    schema: 34,
  });
  assert.deepEqual(manifest.deployment, {
    ownerPlaneCompose: {
      path: OWNER_PLANE_COMPOSE_PATH,
      sha256: "8".repeat(64),
    },
  });
});

test("Core release manifest rejects mutable, missing, duplicated and malformed identity", () => {
  const manifest = createCoreReleaseManifest(input);
  assert.equal(
    validCoreReleaseManifest({
      ...manifest,
      images: {
        ...manifest.images,
        uiCarrier: "ghcr.io/example/core-ui:latest",
      },
    }),
    false,
  );
  assert.equal(
    validCoreReleaseManifest({
      ...manifest,
      deployment: {
        ownerPlaneCompose: {
          ...manifest.deployment.ownerPlaneCompose,
          path: "compose.yaml",
        },
      },
    }),
    false,
  );
  assert.equal(
    validCoreReleaseManifest({
      ...manifest,
      deployment: {
        ownerPlaneCompose: {
          ...manifest.deployment.ownerPlaneCompose,
          sha256: digest("8"),
        },
      },
    }),
    false,
    "the Compose checksum uses the provider's raw SHA-256 representation",
  );
  assert.equal(
    validCoreReleaseManifest({
      ...manifest,
      images: { ...manifest.images, uiCarrier: manifest.images.accountPlane },
    }),
    false,
  );
  assert.equal(
    validCoreReleaseManifest({
      ...manifest,
      images: {
        ...manifest.images,
        uiCarrier: `ghcr.io/blueprintlabio/restless-core-ui@${digest("3")}`,
      },
    }),
    false,
    "three differently named images must not reuse one content digest",
  );
  for (const field of [
    "artifactDigest",
    "payloadDigest",
    "routeManifestDigest",
  ]) {
    assert.equal(
      validCoreReleaseManifest({
        ...manifest,
        ui: { ...manifest.ui, [field]: "sha256:short" },
      }),
      false,
    );
  }
  assert.equal(
    validCoreReleaseManifest({
      ...manifest,
      contracts: { ...manifest.contracts, product: "1" },
    }),
    false,
  );
  assert.equal(validCoreReleaseManifest({ ...manifest, extra: true }), false);
});

test("Core workflow builds one UI and publishes three pinned, attested linux/amd64 images", async () => {
  const workflow = await readFile(
    resolve(repository, ".github/workflows/core-release.yml"),
    "utf8",
  );
  const uses = [...workflow.matchAll(/uses:\s+([^\s#]+)/g)].map(
    (match) => match[1],
  );
  assert.ok(uses.length > 0);
  for (const action of uses) {
    assert.match(
      action,
      /^[^@]+@[a-f0-9]{40}$/,
      `${action} must be pinned to a commit`,
    );
  }

  assert.match(workflow, /tags: \["restless-core-release-\*"\]/);
  assert.doesNotMatch(
    workflow,
    /branches:/,
    "ordinary branch pushes must not publish Core packages",
  );

  assert.equal((workflow.match(/npm --prefix web run build/g) ?? []).length, 1);
  assert.equal((workflow.match(/docker\/build-push-action@/g) ?? []).length, 3);
  assert.equal(
    (workflow.match(/actions\/attest-build-provenance@/g) ?? []).length,
    3,
  );
  assert.equal((workflow.match(/platforms: linux\/amd64/g) ?? []).length, 3);
  for (const dockerfile of [
    "infra/core-ui-artifact/Dockerfile",
    "infra/account-plane/Dockerfile",
    "infra/company-image/Dockerfile",
  ]) {
    assert.match(
      workflow,
      new RegExp(`file: ${dockerfile.replaceAll("/", "\\/")}`),
    );
  }
  for (const image of [
    "CORE_UI_IMAGE",
    "CORE_ACCOUNT_PLANE_IMAGE",
    "CORE_COMPANY_RUNTIME_IMAGE",
  ]) {
    assert.match(workflow, new RegExp(`${image}: .*@\\$\\{\\{ steps\\.`));
  }
  assert.match(
    workflow,
    /node scripts\/create-core-release-manifest\.mjs --verify/,
  );
  assert.match(workflow, /web\/build\/core-ui-manifest\.json/);
  assert.match(workflow, /infra\/account-plane\/cloud-compose\.template\.yaml/);
  assert.match(workflow, /owner-plane-compose\.sha256/);
  assert.match(workflow, /owner_plane_compose_sha256/);
  assert.match(workflow, /docker compose .* config --no-interpolate --quiet/);

  for (const dockerfile of [
    "infra/account-plane/Dockerfile",
    "infra/company-image/Dockerfile",
  ]) {
    const source = await readFile(resolve(repository, dockerfile), "utf8");
    const bases = source
      .split("\n")
      .filter((line) => line.startsWith("FROM "));
    assert.ok(bases.length > 0);
    for (const base of bases) {
      assert.match(
        base,
        /@sha256:[a-f0-9]{64}(?:\s+AS\s+\S+)?$/i,
        `${dockerfile} must pin every external base image`,
      );
    }
  }
});

test("owner-plane Compose is provider-complete, hardened and digest-only after rendering", async () => {
  const template = await readFile(
    resolve(repository, OWNER_PLANE_COMPOSE_PATH),
    "utf8",
  );
  assert.doesNotThrow(() => assertOwnerPlaneComposeTemplate(template));
  assert.match(ownerPlaneComposeSha256(template), /^[a-f0-9]{64}$/);

  const replacements = {
    ACCOUNT_PLANE_IMAGE: image("restless-core-account-plane", "a"),
    COMPANY_RUNTIME_IMAGE: image("restless-core-company-runtime", "b"),
    CORE_RELEASE_MANIFEST_DIGEST: digest("c"),
    DESIRED_REVISION: "d".repeat(40),
    FLEET_ENTRY_ISSUER: "https://fleet.example.test",
    FLEET_ENTRY_JWKS_URL: "https://fleet.example.test/.well-known/jwks.json",
    HOSTNAME: "plane.example.test",
    OWNER_ID: "owner_test",
    PLANE_ID: "plane_test",
    RUNTIME_BOOTSTRAP_SECRET_FILE:
      "runtime-bootstrap-token-11111111-1111-7111-8111-111111111111",
  };
  let rendered = template;
  for (const token of OWNER_PLANE_TEMPLATE_TOKENS) {
    rendered = rendered.replaceAll(`{{${token}}}`, replacements[token]);
  }
  assert.doesNotMatch(rendered, /\{\{[A-Z0-9_]+\}\}/);
  const imageLines = [
    ...rendered.matchAll(/^\s*image:\s*["']?([^\s"']+)["']?\s*$/gm),
  ].map((match) => match[1]);
  assert.equal(imageLines.length, 2);
  for (const reference of imageLines) {
    assert.match(reference, /@sha256:[a-f0-9]{64}$/);
  }
  assert.match(
    rendered,
    /file: "\.\/secrets\/runtime-bootstrap-token-11111111-1111-7111-8111-111111111111"/,
  );
  assert.doesNotMatch(rendered, /RESTLESS_RUNTIME_BOOTSTRAP_TOKEN\s*:/);

  for (const mutation of [
    `${template}\nservices:\n  escape:\n    build: .\n`,
    template.replace('    expose: ["7788"]', '    ports: ["7788:7788"]'),
    template.replace("    read_only: true", "    privileged: true"),
    template.replace("    read_only: true", "    privileged: True"),
    template.replace(
      "    networks: [database, public-proxy]",
      "    network_mode: host",
    ),
    template.replace(
      "      - plane-state:/state",
      "      - /var/run/docker.sock:/var/run/docker.sock",
    ),
    template.replace("postgres@sha256:4", "postgres:latest"),
    template.replace("      RESTLESS_ENTRY_MODE: network\n", ""),
    template.replace("      - plane-state:/state\n", ""),
    template.replace("      - plane-database:/var/lib/postgresql\n", ""),
    template.replace(
      "{{FLEET_ENTRY_JWKS_URL}}",
      "https://fleet.example.test/jwks",
    ),
    template.replace(
      'file: "./secrets/{{RUNTIME_BOOTSTRAP_SECRET_FILE}}"',
      "environment: RESTLESS_RUNTIME_BOOTSTRAP_TOKEN",
    ),
    template.replace(
      "      RESTLESS_ENTRY_MODE: network",
      "      RESTLESS_RUNTIME_BOOTSTRAP_TOKEN: leaked\n      RESTLESS_ENTRY_MODE: network",
    ),
    template.replace("{{RUNTIME_BOOTSTRAP_SECRET_FILE}}", "missing-token"),
    `${template}\n# {{RUNTIME_BOOTSTRAP_SECRET_FILE}}\n`,
    `${template}\n# {{unknown_token}}\n`,
    `${template}\n# {{UNFINISHED\n`,
  ]) {
    assert.equal(validOwnerPlaneComposeTemplate(mutation), false);
  }
});

test("release metadata is derived from canonical source constants", async () => {
  const generator = await readFile(
    resolve(repository, "scripts/create-core-release-manifest.mjs"),
    "utf8",
  );
  for (const source of [
    "web/src/lib/product/contracts.ts",
    "web/src/lib/platform/contracts.ts",
    "crates/restlessd/src/release.rs",
    "crates/restlessd/src/entry.rs",
  ]) {
    assert.match(generator, new RegExp(source.replaceAll("/", "\\/")));
  }
  assert.doesNotMatch(generator, /productContractVersion:\s*1[,\n]/);
  assert.doesNotMatch(generator, /capabilityContractVersion:\s*1[,\n]/);
  assert.match(generator, /OWNER_PLANE_COMPOSE_PATH/);
  assert.match(generator, /ownerPlaneComposeSha256/);
});
