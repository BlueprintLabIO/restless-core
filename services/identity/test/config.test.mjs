import test from "node:test";
import assert from "node:assert/strict";
import { origin } from "../src/config.mjs";
test("identity origins require HTTPS except on loopback", () => {
  for (const value of [
    "https://accounts.example.com",
    "http://localhost:6689",
    "http://accounts.localhost:6689",
    "http://127.0.0.1:6689",
  ])
    assert.equal(origin(value).origin, value);
  for (const value of [
    "http://accounts.example.com",
    "https://accounts.example.com/login",
    "https://accounts.example.com/",
    "https://user:pass@accounts.example.com",
    "https://accounts.example.com?return=elsewhere",
  ])
    assert.throws(() => origin(value));
});

import { generateKeyPairSync, randomUUID } from "node:crypto";
import { chmod, mkdtemp, rm, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { readConfig, validateConfig } from "../src/config.mjs";
function fixture() {
  const { privateKey } = generateKeyPairSync("ed25519");
  return {
    origin: "http://accounts.localhost:6689",
    coreOrigin: "http://plane.localhost:7788",
    secret: "test-secret-".repeat(6),
    signingKey: { ...privateKey.export({ format: "jwk" }), kid: "test" },
    ownerId: randomUUID(),
    planeId: randomUUID(),
    companyId: randomUUID(),
    cellId: randomUUID(),
    companyName: "Local test",
    ownerEmail: "owner@example.test",
    databaseUrl: "postgresql://test:test@localhost/accounts",
  };
}
test("setup rejects destinations Core cannot admit and invalid listener ports", () => {
  const config = fixture();
  for (const coreOrigin of ["http://127.0.0.1:7788", "http://localhost:7788"])
    assert.throws(
      () => validateConfig({ ...config, coreOrigin }),
      /DNS hostname/,
    );
  for (const port of [0, 65536, 3.5, "6689"])
    assert.throws(() => validateConfig({ ...config, port }), /listening port/);
  assert.equal(validateConfig({ ...config, port: 6689 }).port, 6689);
});
test("configuration must be a private file and cannot be replaced by a symlink", async () => {
  const directory = await mkdtemp(join(tmpdir(), "restless-identity-config-"));
  try {
    const path = join(directory, "config.json");
    const config = fixture();
    await writeFile(path, JSON.stringify(config), { mode: 0o600 });
    assert.equal((await readConfig(path)).companyId, config.companyId);
    await chmod(path, 0o644);
    await assert.rejects(readConfig(path), /private regular file/);
    await chmod(path, 0o600);
    const link = join(directory, "link.json");
    await symlink(path, link);
    await assert.rejects(readConfig(link));
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});
test("secure account and Core origins must share a parent domain", () => {
  const config = fixture();
  const secure = {
    ...config,
    origin: "https://accounts.example.com",
    coreOrigin: "https://work.example.com",
  };
  assert.equal(validateConfig(secure).coreHost, "work.example.com");
  for (const coreOrigin of [
    "https://work.example.org",
    "https://example.com",
    "https://work.other.com",
  ])
    assert.throws(
      () => validateConfig({ ...secure, coreOrigin }),
      /share a parent domain/,
    );
  assert.equal(
    validateConfig({ ...secure, coreOrigin: "https://acme.planes.example.com" })
      .coreHost,
    "acme.planes.example.com",
  );
});
