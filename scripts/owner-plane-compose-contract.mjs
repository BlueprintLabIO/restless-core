import { createHash } from "node:crypto";

export const OWNER_PLANE_COMPOSE_PATH =
  "infra/account-plane/cloud-compose.template.yaml";

export const OWNER_PLANE_TEMPLATE_TOKENS = [
  "ACCOUNT_PLANE_IMAGE",
  "COMPANY_RUNTIME_IMAGE",
  "CORE_RELEASE_MANIFEST_DIGEST",
  "DESIRED_REVISION",
  "FLEET_ENTRY_ISSUER",
  "FLEET_ENTRY_JWKS_URL",
  "HOSTNAME",
  "OWNER_ID",
  "PLANE_ID",
];

export const OWNER_PLANE_RUNTIME_ENVIRONMENT = [
  "RESTLESS_ENTRY_ISSUER",
  "RESTLESS_ENTRY_JWKS_URL",
  "RESTLESS_ENTRY_OWNER_ID",
  "RESTLESS_ENTRY_PLANE_ID",
  "RESTLESS_ENTRY_HOST",
  "RESTLESS_DESIRED_REVISION",
  "RESTLESS_ACCOUNT_PLANE_IMAGE",
  "RESTLESS_COMPANY_IMAGE",
  "RESTLESS_RELEASE_MANIFEST_DIGEST",
  "RESTLESS_RUNTIME_BOOTSTRAP_TOKEN_FILE",
];

const DIGEST_IMAGE =
  /^[a-z0-9][a-z0-9._/-]*(?::[a-z0-9._-]+)?@sha256:[0-9a-f]{64}$/;

function exactMembers(observed, expected) {
  const left = [...new Set(observed)].sort();
  const right = [...expected].sort();
  return (
    left.length === right.length &&
    left.every((value, index) => value === right[index])
  );
}

function occurrenceCount(value, fragment) {
  return value.split(fragment).length - 1;
}

export function ownerPlaneComposeSha256(template) {
  return createHash("sha256").update(template).digest("hex");
}

export function assertOwnerPlaneComposeTemplate(template) {
  if (
    typeof template !== "string" ||
    template.length < 80 ||
    template.length > 65_536
  ) {
    throw new Error("owner-plane Compose template has invalid size");
  }

  const tokenPattern = /\{\{([^{}\r\n]+)\}\}/g;
  const tokenMatches = [...template.matchAll(tokenPattern)];
  const tokens = tokenMatches.map((match) => match[1]);
  const withoutTokens = template.replace(tokenPattern, "");
  if (
    withoutTokens.includes("{{") ||
    withoutTokens.includes("}}") ||
    tokenMatches.some((match) => !/^[A-Z0-9_]+$/.test(match[1])) ||
    !exactMembers(tokens, OWNER_PLANE_TEMPLATE_TOKENS)
  ) {
    throw new Error(
      "owner-plane Compose template has the wrong provider tokens",
    );
  }
  if (
    occurrenceCount(template, "{{ACCOUNT_PLANE_IMAGE}}") !== 2 ||
    !/^\s*image\s*:\s*["']?\{\{ACCOUNT_PLANE_IMAGE\}\}["']?\s*$/m.test(
      template,
    ) ||
    occurrenceCount(template, "{{COMPANY_RUNTIME_IMAGE}}") !== 1
  ) {
    throw new Error(
      "owner-plane Compose template does not bind its image roles and reported plane identity exactly",
    );
  }

  const forbidden = [
    /(^|\n)\s*build\s*:/,
    /(^|\n)\s*ports\s*:/,
    /docker\.sock/i,
    /(^|\n)\s*privileged\s*:\s*["']?true["']?\s*$/im,
    /(^|\n)\s*network_mode\s*:\s*["']?host["']?\s*$/im,
    /(^|\n)\s*pid\s*:\s*["']?host["']?\s*$/im,
  ];
  if (forbidden.some((pattern) => pattern.test(template))) {
    throw new Error(
      "owner-plane Compose template contains a host-control feature",
    );
  }

  const imageLines = [
    ...template.matchAll(/^\s*image\s*:\s*["']?([^\s"']+)["']?\s*$/gm),
  ].map((match) => match[1]);
  if (
    imageLines.length !== 2 ||
    imageLines.filter((image) => image === "{{ACCOUNT_PLANE_IMAGE}}").length !==
      1 ||
    imageLines
      .filter((image) => image !== "{{ACCOUNT_PLANE_IMAGE}}")
      .some((image) => !DIGEST_IMAGE.test(image))
  ) {
    throw new Error("owner-plane Compose template contains a mutable image");
  }

  for (const fragment of [
    "read_only: true",
    "cap_drop:",
    "no-new-privileges:true",
    "healthcheck:",
    "volumes:",
    "plane-state:/state",
    "plane-database:/var/lib/postgresql/data",
    "RESTLESS_ENTRY_MODE: network",
    "RESTLESS_RUNTIME_BOOTSTRAP_TOKEN_FILE: /run/secrets/runtime_bootstrap_token",
    "environment: RESTLESS_RUNTIME_BOOTSTRAP_TOKEN",
  ]) {
    if (!template.includes(fragment)) {
      throw new Error(`owner-plane Compose template is missing ${fragment}`);
    }
  }
  for (const name of OWNER_PLANE_RUNTIME_ENVIRONMENT) {
    if (!new RegExp(`^\\s*${name}:`, "m").test(template)) {
      throw new Error(`owner-plane Compose template is missing ${name}`);
    }
  }

  return template;
}

export function validOwnerPlaneComposeTemplate(template) {
  try {
    assertOwnerPlaneComposeTemplate(template);
    return true;
  } catch {
    return false;
  }
}
