import { createHash } from "node:crypto";
import { OWNER_PLANE_COMPOSE_PATH } from "./owner-plane-compose-contract.mjs";

const REVISION = /^[a-f0-9]{40}$/;
const DIGEST = /^sha256:[a-f0-9]{64}$/;
const IMAGE = /^[a-z0-9.-]+\/[a-z0-9._/-]+@sha256:[a-f0-9]{64}$/;
const RAW_SHA256 = /^[a-f0-9]{64}$/;
const VERSION = /^\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.-]+)?$/;

export const CORE_RELEASE_CONTRACT = "restless-core-release.v1";

function exactKeys(value, expected) {
  if (!value || Array.isArray(value) || typeof value !== "object") return false;
  const keys = Object.keys(value).sort();
  const wanted = [...expected].sort();
  return (
    keys.length === wanted.length &&
    keys.every((key, index) => key === wanted[index])
  );
}

export function validSourceRevision(value) {
  return typeof value === "string" && REVISION.test(value);
}

export function validDigest(value) {
  return typeof value === "string" && DIGEST.test(value);
}

export function validImageReference(value) {
  return (
    typeof value === "string" &&
    IMAGE.test(value) &&
    !value.includes(":latest@")
  );
}

export function validContractVersion(value) {
  return Number.isSafeInteger(value) && value > 0;
}

export function canonicalCoreReleaseJson(manifest) {
  return `${JSON.stringify(manifest, null, 2)}\n`;
}

export function coreReleaseSha256(manifest) {
  return `sha256:${createHash("sha256").update(canonicalCoreReleaseJson(manifest)).digest("hex")}`;
}

export function validCoreReleaseManifest(manifest) {
  if (
    !exactKeys(manifest, [
      "contract",
      "schemaVersion",
      "sourceRevision",
      "coreVersion",
      "images",
      "ui",
      "contracts",
      "deployment",
    ]) ||
    !exactKeys(manifest.images, [
      "uiCarrier",
      "accountPlane",
      "companyRuntime",
    ]) ||
    !exactKeys(manifest.ui, [
      "artifactDigest",
      "payloadDigest",
      "routeManifestDigest",
      "manifestPath",
      "routeCount",
    ]) ||
    !exactKeys(manifest.contracts, [
      "product",
      "capability",
      "api",
      "identityAssertion",
      "schema",
    ]) ||
    !exactKeys(manifest.deployment, ["ownerPlaneCompose"]) ||
    !exactKeys(manifest.deployment.ownerPlaneCompose, ["path", "sha256"])
  ) {
    return false;
  }

  const images = Object.values(manifest.images);
  const imageDigests = images.map((image) => image.split("@").at(-1));
  return (
    manifest.contract === CORE_RELEASE_CONTRACT &&
    manifest.schemaVersion === 1 &&
    validSourceRevision(manifest.sourceRevision) &&
    typeof manifest.coreVersion === "string" &&
    VERSION.test(manifest.coreVersion) &&
    images.every(validImageReference) &&
    new Set(images).size === images.length &&
    new Set(imageDigests).size === imageDigests.length &&
    validDigest(manifest.ui.artifactDigest) &&
    validDigest(manifest.ui.payloadDigest) &&
    validDigest(manifest.ui.routeManifestDigest) &&
    manifest.ui.manifestPath === "/core-ui-manifest.json" &&
    validContractVersion(manifest.ui.routeCount) &&
    Object.values(manifest.contracts).every(validContractVersion) &&
    manifest.deployment.ownerPlaneCompose.path === OWNER_PLANE_COMPOSE_PATH &&
    typeof manifest.deployment.ownerPlaneCompose.sha256 === "string" &&
    RAW_SHA256.test(manifest.deployment.ownerPlaneCompose.sha256)
  );
}

export function createCoreReleaseManifest(input) {
  const manifest = {
    contract: CORE_RELEASE_CONTRACT,
    schemaVersion: 1,
    sourceRevision: input.sourceRevision,
    coreVersion: input.coreVersion,
    images: {
      uiCarrier: input.uiCarrierImage,
      accountPlane: input.accountPlaneImage,
      companyRuntime: input.companyRuntimeImage,
    },
    ui: {
      artifactDigest: input.uiArtifactDigest,
      payloadDigest: input.uiPayloadDigest,
      routeManifestDigest: input.uiRouteManifestDigest,
      manifestPath: "/core-ui-manifest.json",
      routeCount: input.uiRouteCount,
    },
    contracts: {
      product: input.productContractVersion,
      capability: input.capabilityContractVersion,
      api: input.apiContractVersion,
      identityAssertion: input.identityAssertionContractVersion,
      schema: input.schemaVersion,
    },
    deployment: {
      ownerPlaneCompose: {
        path: input.ownerPlaneComposePath,
        sha256: input.ownerPlaneComposeSha256,
      },
    },
  };
  if (!validCoreReleaseManifest(manifest)) {
    throw new Error("Core release manifest violates its contract");
  }
  return manifest;
}
