import { constants } from "node:fs";
import { mkdir, open, rm, writeFile } from "node:fs/promises";
import { generateKeyPairSync, randomBytes, randomUUID } from "node:crypto";
import { isAbsolute, join } from "node:path";
import { parseArgs } from "node:util";
import { pathToFileURL } from "node:url";
import pg from "pg";
import { validateConfig } from "../src/config.mjs";

async function privateText(path) {
  if (!isAbsolute(path))
    throw Error("Credential files must have absolute paths");
  const file = await open(path, constants.O_RDONLY | constants.O_NOFOLLOW);
  try {
    const stat = await file.stat();
    if (
      !stat.isFile() ||
      stat.nlink !== 1 ||
      stat.mode & 0o077 ||
      stat.size > 32768
    ) {
      throw Error("Credential files must be private regular files (mode 0600)");
    }
    return await file.readFile("utf8");
  } finally {
    await file.close();
  }
}
const shellQuote = (value) =>
  "'" + String(value).replaceAll("'", "'\\''") + "'";

export async function configure(options) {
  if (!isAbsolute(options.coreHome) || !isAbsolute(options.output))
    throw Error("Core home and output must be absolute paths");
  if (!/^[a-z][a-z0-9_]{0,62}$/.test(options.company))
    throw Error(
      "Use the existing company handle, containing lowercase letters, digits and underscores",
    );
  if (!/^\S+@sha256:[a-f0-9]{64}$/.test(options.companyImage))
    throw Error("The company image must be pinned as repository@sha256:digest");
  const databaseUrl = (await privateText(options.databaseUrlFile)).trim();
  const smtp = JSON.parse(await privateText(options.smtpFile));
  if (!smtp.host || !smtp.from)
    throw Error("SMTP host and from address are required");
  const sourceUrl = (
    await privateText(
      join(options.coreHome, "cells", options.company, "database.url"),
    )
  ).trim();
  if (sourceUrl === databaseUrl)
    throw Error("Accounts need a separate database from company data");
  const source = new pg.Client({
    connectionString: sourceUrl,
    connectionTimeoutMillis: 5000,
    statement_timeout: 10000,
  });
  let identity;
  try {
    await source.connect();
    const result = await source.query(
      `SELECT company_id, cell_id FROM "${options.company}".company_access_identity`,
    );
    if (result.rows.length !== 1)
      throw Error("Core must have initialized exactly one company identity");
    identity = result.rows[0];
  } finally {
    await source.end().catch(() => {});
  }
  const accounts = new pg.Client({
    connectionString: databaseUrl,
    connectionTimeoutMillis: 5000,
    statement_timeout: 10000,
  });
  try {
    await accounts.connect();
    // Catalog rows reveal an accidental reuse even when this role cannot read
    // Core's table and the URLs spell the same database differently.
    const result = await accounts.query(
      `SELECT EXISTS (
         SELECT 1
         FROM pg_catalog.pg_namespace namespace
         JOIN pg_catalog.pg_class relation
           ON relation.relnamespace = namespace.oid
         WHERE namespace.nspname = $1
           AND relation.relname = 'company_access_identity'
           AND relation.relkind IN ('r', 'p')
       ) AS contains_company_data`,
      [options.company],
    );
    if (result.rows[0]?.contains_company_data)
      throw Error("Accounts need a separate database from company data");
  } finally {
    await accounts.end().catch(() => {});
  }
  const { privateKey } = generateKeyPairSync("ed25519");
  const config = validateConfig({
    origin: options.origin,
    coreOrigin: options.coreOrigin,
    companyName: options.companyName,
    ownerEmail: options.ownerEmail,
    ownerId: randomUUID(),
    planeId: randomUUID(),
    companyId: identity.company_id,
    cellId: identity.cell_id,
    secret: randomBytes(48).toString("hex"),
    signingKey: { ...privateKey.export({ format: "jwk" }), kid: randomUUID() },
    databaseUrl,
    smtp,
    address: "127.0.0.1",
    port: Number(options.port ?? 6689),
  });
  // Refuse existing output, so a second run cannot replace active signing keys.
  await mkdir(options.output, { mode: 0o700 });
  try {
    await writeFile(
      join(options.output, "identity.json"),
      JSON.stringify(config, null, 2) + "\n",
      { flag: "wx", mode: 0o600 },
    );
    const env = {
      RESTLESS_ENTRY_MODE: "network",
      RESTLESS_RUNTIME_MODE: "local",
      RESTLESS_ENTRY_ISSUER: config.origin,
      RESTLESS_ENTRY_JWKS_URL: config.origin + "/.well-known/jwks.json",
      RESTLESS_ENTRY_OWNER_ID: config.ownerId,
      RESTLESS_ENTRY_PLANE_ID: config.planeId,
      RESTLESS_ENTRY_HOST: config.coreHost,
      RESTLESS_COMPANY_IMAGE: options.companyImage,
      ...(config.origin.startsWith("http:")
        ? { RESTLESS_ENTRY_ALLOW_INSECURE_HTTP: "1" }
        : {}),
    };
    await writeFile(
      join(options.output, "core-entry.env"),
      Object.entries(env)
        .map(([key, value]) => `export ${key}=${shellQuote(value)}`)
        .join("\n") + "\n",
      { flag: "wx", mode: 0o600 },
    );
    return {
      configPath: join(options.output, "identity.json"),
      coreEnvironment: join(options.output, "core-entry.env"),
    };
  } catch (error) {
    await rm(options.output, { recursive: true, force: true });
    throw error;
  }
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(process.argv[1]).href
) {
  try {
    const names = [
      "core-home",
      "company",
      "company-name",
      "company-image",
      "origin",
      "core-origin",
      "owner-email",
      "database-url-file",
      "smtp-file",
      "output",
      "port",
    ];
    const { values } = parseArgs({
      options: Object.fromEntries(
        names.map((name) => [name, { type: "string" }]),
      ),
    });
    for (const name of names.filter((name) => name !== "port"))
      if (!values[name])
        throw Error(`Missing --${name}; see this service's README`);
    const options = Object.fromEntries(
      Object.entries(values).map(([name, value]) => [
        name.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase()),
        value,
      ]),
    );
    const result = await configure(options);
    console.log(
      `Private account configuration: ${result.configPath}\nCore entry environment: ${result.coreEnvironment}`,
    );
  } catch (error) {
    console.error(`Account setup failed: ${error.message}`);
    process.exitCode = 1;
  }
}
