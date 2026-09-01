#!/usr/bin/env node

import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";
import { execFileSync } from "node:child_process";

const manifestPath = process.argv[2];
if (!manifestPath || process.argv.length !== 3) {
  console.error("usage: node scripts/core-release-check.mjs <core-release.json>");
  process.exit(2);
}

const fail = (message) => {
  throw new Error(`invalid Core release manifest: ${message}`);
};
const exactKeys = (value, expected, where) => {
  const actual = Object.keys(value ?? {}).sort();
  const wanted = [...expected].sort();
  if (JSON.stringify(actual) !== JSON.stringify(wanted)) {
    fail(`${where} keys are ${actual.join(", ")}; expected ${wanted.join(", ")}`);
  }
};
const immutableImage = /^ghcr\.io\/[a-z0-9_.-]+\/[a-z0-9_.-]+@sha256:[0-9a-f]{64}$/;
const sha256 = /^[0-9a-f]{64}$/;
const gitRevision = /^[0-9a-f]{40}$/;

const raw = await readFile(manifestPath);
const manifest = JSON.parse(raw);
exactKeys(
  manifest,
  [
    "manifest_version",
    "core_version",
    "source_revision",
    "images",
    "contracts",
    "identity_handoff",
    "deployment",
    "compatibility",
    "contracts_artifact",
    "support",
    "health",
  ],
  "manifest",
);
if (manifest.manifest_version !== 1) fail("manifest_version must be 1");
if (!/^\d+\.\d+\.\d+$/.test(manifest.core_version)) fail("core_version is not semver");
if (!gitRevision.test(manifest.source_revision)) fail("source_revision is not an exact Git revision");
const revision = execFileSync("git", ["rev-parse", "HEAD"], { encoding: "utf8" }).trim();
if (manifest.source_revision !== revision) fail("source_revision does not match the checked-out commit");

exactKeys(manifest.images, ["account_plane", "company_runtime"], "images");
for (const [name, reference] of Object.entries(manifest.images)) {
  if (!immutableImage.test(reference)) fail(`${name} is not an immutable GHCR digest`);
}
const expectedPlane = process.env.RESTLESS_ACCOUNT_PLANE_IMAGE_DIGEST;
const expectedRuntime = process.env.RESTLESS_COMPANY_IMAGE_DIGEST;
if (expectedPlane && manifest.images.account_plane !== expectedPlane) fail("account_plane digest differs from the published image");
if (expectedRuntime && manifest.images.company_runtime !== expectedRuntime) fail("company_runtime digest differs from the published image");

exactKeys(manifest.contracts, ["api", "identity_assertion", "schema"], "contracts");
for (const [name, version] of Object.entries(manifest.contracts)) {
  if (!Number.isSafeInteger(version) || version < 1) fail(`${name} contract version must be a positive integer`);
}
if (
  manifest.identity_handoff?.algorithm !== "Ed25519" ||
  manifest.identity_handoff?.jws_alg !== "EdDSA" ||
  manifest.identity_handoff?.audience !== "restless-core-account-plane" ||
  manifest.identity_handoff?.replay !== "atomic durable PostgreSQL jti consumption before host-only session"
) fail("identity handoff is not the hosted Ed25519/durable-replay contract");
if (manifest.deployment?.host_control !== "none") fail("account plane must not have host control");
if (manifest.deployment?.account_plane_compose !== "infra/account-plane/compose.yml") fail("unexpected account-plane Compose path");
if (!sha256.test(manifest.deployment?.account_plane_compose_sha256 ?? "")) fail("Compose digest is not sha256");
const compose = await readFile(manifest.deployment.account_plane_compose);
const composeDigest = createHash("sha256").update(compose).digest("hex");
if (manifest.deployment.account_plane_compose_sha256 !== composeDigest) fail("Compose digest does not match the published template");
if (manifest.support?.status !== "candidate") fail("new Core releases must begin as candidates");

console.log(JSON.stringify({
  status: "verified",
  revision,
  manifest_sha256: createHash("sha256").update(raw).digest("hex"),
  compose_sha256: composeDigest,
  images: manifest.images,
}));
