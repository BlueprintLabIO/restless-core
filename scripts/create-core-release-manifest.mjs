#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { appendFile, readFile, writeFile } from "node:fs/promises";
import { basename, resolve } from "node:path";
import {
  canonicalCoreReleaseJson,
  coreReleaseSha256,
  createCoreReleaseManifest,
  validCoreReleaseManifest,
} from "./core-release-contract.mjs";
import {
  coreUiReleaseMetadata,
  verifyCoreUiArtifact,
} from "../web/scripts/core-ui-artifact.mjs";
import {
  OWNER_PLANE_COMPOSE_PATH,
  assertOwnerPlaneComposeTemplate,
  ownerPlaneComposeSha256,
} from "./owner-plane-compose-contract.mjs";

const projectRoot = resolve(import.meta.dirname, "..");
const webRoot = resolve(projectRoot, "web");
const buildRoot = resolve(webRoot, "build");

function required(name) {
  const value = process.env[name]?.trim();
  if (!value) throw new Error(`${name} is required`);
  return value;
}

function git(args) {
  return execFileSync(process.env.GIT_BINARY?.trim() || "git", args, {
    cwd: projectRoot,
    encoding: "utf8",
    stdio: ["ignore", "pipe", "pipe"],
  }).trim();
}

async function sourceMatch(path, pattern, label) {
  const source = await readFile(resolve(projectRoot, path), "utf8");
  const value = source.match(pattern)?.[1];
  if (!value) throw new Error(`could not read ${label} from ${path}`);
  return value;
}

function positiveInteger(value, label) {
  if (!/^[1-9]\d*$/.test(value))
    throw new Error(`${label} must be a positive integer`);
  const parsed = Number(value);
  if (!Number.isSafeInteger(parsed))
    throw new Error(`${label} must be a safe integer`);
  return parsed;
}

function prefixedDigest(value, label) {
  if (!/^[a-f0-9]{64}$/.test(value))
    throw new Error(`${label} must be a sha256 hex digest`);
  return `sha256:${value}`;
}

async function ownerPlaneComposeMetadata() {
  const bytes = await readFile(resolve(projectRoot, OWNER_PLANE_COMPOSE_PATH));
  assertOwnerPlaneComposeTemplate(bytes.toString("utf8"));
  return {
    ownerPlaneComposePath: OWNER_PLANE_COMPOSE_PATH,
    ownerPlaneComposeSha256: ownerPlaneComposeSha256(bytes),
  };
}

async function releaseMetadata() {
  const sourceRevision = required("SOURCE_REVISION");
  const checkoutRevision = git(["rev-parse", "HEAD"]);
  if (sourceRevision !== checkoutRevision) {
    throw new Error(
      "SOURCE_REVISION does not match the checked-out Core commit",
    );
  }
  if (git(["status", "--porcelain", "--untracked-files=no"])) {
    throw new Error("Core release creation requires a clean tracked checkout");
  }

  const [
    coreVersion,
    productContract,
    capabilityContract,
    apiContract,
    identityAssertionContract,
    schemaVersion,
    uiManifest,
    ownerPlaneCompose,
  ] = await Promise.all([
    sourceMatch("Cargo.toml", /^version\s*=\s*"([^"]+)"/m, "Core version"),
    sourceMatch(
      "web/src/lib/product/contracts.ts",
      /PRODUCT_CONTRACT_VERSION\s*=\s*(\d+)\s+as const/,
      "product contract version",
    ),
    sourceMatch(
      "web/src/lib/platform/contracts.ts",
      /PLATFORM_SCHEMA_VERSION\s*=\s*(\d+)\s+as const/,
      "capability contract version",
    ),
    sourceMatch(
      "crates/restlessd/src/release.rs",
      /API_CONTRACT_VERSION:\s*u32\s*=\s*(\d+)/,
      "API contract version",
    ),
    sourceMatch(
      "crates/restlessd/src/entry.rs",
      /ASSERTION_CONTRACT_VERSION:\s*u32\s*=\s*(\d+)/,
      "identity assertion contract version",
    ),
    sourceMatch(
      "crates/restlessd/src/release.rs",
      /SCHEMA_VERSION:\s*u32\s*=\s*(\d+)/,
      "schema version",
    ),
    verifyCoreUiArtifact({ projectRoot, webRoot, buildRoot }),
    ownerPlaneComposeMetadata(),
  ]);
  const ui = coreUiReleaseMetadata(uiManifest);
  return {
    sourceRevision,
    coreVersion,
    productContractVersion: positiveInteger(
      productContract,
      "product contract version",
    ),
    capabilityContractVersion: positiveInteger(
      capabilityContract,
      "capability contract version",
    ),
    apiContractVersion: positiveInteger(apiContract, "API contract version"),
    identityAssertionContractVersion: positiveInteger(
      identityAssertionContract,
      "identity assertion contract version",
    ),
    schemaVersion: positiveInteger(schemaVersion, "schema version"),
    uiArtifactDigest: prefixedDigest(
      ui.artifact_sha256,
      "Core UI artifact digest",
    ),
    uiPayloadDigest: prefixedDigest(
      ui.payload_sha256,
      "Core UI payload digest",
    ),
    uiRouteManifestDigest: prefixedDigest(
      ui.route_manifest_sha256,
      "Core UI route-manifest digest",
    ),
    uiRouteCount: positiveInteger(
      String(ui.route_count),
      "Core UI route count",
    ),
    ...ownerPlaneCompose,
  };
}

if (process.argv[2] === "--verify") {
  const path = process.argv[3];
  if (!path)
    throw new Error(
      "usage: create-core-release-manifest.mjs --verify <manifest.json>",
    );
  const manifest = JSON.parse(await readFile(path, "utf8"));
  if (!validCoreReleaseManifest(manifest)) {
    throw new Error("Core release manifest violates its contract");
  }
  console.log(
    JSON.stringify({
      file: basename(path),
      digest: coreReleaseSha256(manifest),
      status: "valid",
    }),
  );
  process.exit(0);
}

const metadata = await releaseMetadata();
if (process.argv[2] === "--github-output") {
  const output = required("GITHUB_OUTPUT");
  await appendFile(
    output,
    [
      `core_version=${metadata.coreVersion}`,
      `product_contract_version=${metadata.productContractVersion}`,
      `capability_contract_version=${metadata.capabilityContractVersion}`,
      `api_contract_version=${metadata.apiContractVersion}`,
      `identity_assertion_contract_version=${metadata.identityAssertionContractVersion}`,
      `schema_version=${metadata.schemaVersion}`,
      `ui_artifact_digest=${metadata.uiArtifactDigest}`,
      `ui_payload_digest=${metadata.uiPayloadDigest}`,
      `ui_route_manifest_digest=${metadata.uiRouteManifestDigest}`,
      `ui_route_count=${metadata.uiRouteCount}`,
      `owner_plane_compose_path=${metadata.ownerPlaneComposePath}`,
      `owner_plane_compose_sha256=${metadata.ownerPlaneComposeSha256}`,
      "",
    ].join("\n"),
    "utf8",
  );
  process.exit(0);
}
if (process.argv[2] === "--metadata") {
  console.log(JSON.stringify(metadata));
  process.exit(0);
}

const manifest = createCoreReleaseManifest({
  ...metadata,
  uiCarrierImage: required("CORE_UI_IMAGE"),
  accountPlaneImage: required("CORE_ACCOUNT_PLANE_IMAGE"),
  companyRuntimeImage: required("CORE_COMPANY_RUNTIME_IMAGE"),
});
const output =
  process.env.CORE_RELEASE_MANIFEST_PATH?.trim() ||
  "core-release-manifest.json";
await writeFile(output, canonicalCoreReleaseJson(manifest), {
  encoding: "utf8",
  flag: "wx",
  mode: 0o600,
});
console.log(
  JSON.stringify({
    file: output,
    digest: coreReleaseSha256(manifest),
    status: "created",
  }),
);
