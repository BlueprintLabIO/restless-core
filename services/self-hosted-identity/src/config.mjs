import { open } from "node:fs/promises";
import { constants } from "node:fs";
import { isAbsolute } from "node:path";
import { isIP } from "node:net";
const UUID =
  /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
export function origin(value) {
  const url = new URL(value);
  const local =
    url.hostname === "localhost" ||
    url.hostname.endsWith(".localhost") ||
    url.hostname === "127.0.0.1" ||
    url.hostname === "[::1]";
  if (
    url.origin !== value ||
    url.username ||
    url.password ||
    !(url.protocol === "https:" || (local && url.protocol === "http:"))
  )
    throw Error(
      "Origins must use HTTPS, or HTTP on a loopback development host",
    );
  return url;
}
export function validateConfig(value) {
  const account = origin(value.origin),
    core = origin(value.coreOrigin);
  if (account.origin === core.origin)
    throw Error("The identity service and Core need separate origins");
  if (isIP(core.hostname) || !core.hostname.includes("."))
    throw Error(
      "Core needs a DNS hostname, such as work.example.com or plane.localhost",
    );
  if (typeof value.secret !== "string" || value.secret.length < 48)
    throw Error("Identity secret must contain at least 48 characters");
  if (
    !value.signingKey?.d ||
    value.signingKey.kty !== "OKP" ||
    value.signingKey.crv !== "Ed25519" ||
    !value.signingKey.kid
  )
    throw Error("An Ed25519 private signing JWK with a key id is required");
  for (const key of ["ownerId", "planeId", "companyId", "cellId"])
    if (!UUID.test(value[key])) throw Error(`${key} must be a non-nil UUID`);
  if (
    typeof value.companyName !== "string" ||
    !value.companyName.trim() ||
    value.companyName.length > 120
  )
    throw Error("A company name is required");
  if (
    typeof value.ownerEmail !== "string" ||
    !/^\S+@\S+\.\S+$/.test(value.ownerEmail)
  )
    throw Error("An owner email is required");
  const db = new URL(value.databaseUrl);
  if (
    !["postgres:", "postgresql:"].includes(db.protocol) ||
    !db.hostname ||
    !db.username ||
    !db.password ||
    db.pathname === "/"
  )
    throw Error(
      "A password-authenticated identity PostgreSQL database is required",
    );
  const port = value.port ?? Number(account.port || 6689);
  if (!Number.isInteger(port) || port < 1 || port > 65535)
    throw Error("The listening port must be an integer from 1 to 65535");
  return {
    ...value,
    ownerEmail: value.ownerEmail.toLowerCase(),
    host: account.host,
    coreHost: core.hostname,
    port,
    address: value.address ?? "127.0.0.1",
  };
}
export async function readConfig(path) {
  if (!path || !isAbsolute(path))
    throw Error(
      "RESTLESS_IDENTITY_CONFIG must name an absolute private JSON file",
    );
  const file = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const meta = await file.stat();
    if (
      !meta.isFile() ||
      meta.nlink !== 1 ||
      (meta.mode & 0o077) !== 0 ||
      meta.size > 32768
    )
      throw Error(
        "Identity configuration must be one private regular file (mode 0600)",
      );
    return validateConfig(JSON.parse(await file.readFile("utf8")));
  } finally {
    await file.close();
  }
}
