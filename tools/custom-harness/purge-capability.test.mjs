import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

test("credential cleanup matches chunk boundaries without touching unrelated files or linked profiles", (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "restless-purge_test-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const profile = path.join(root, "profile");
  fs.mkdirSync(profile);
  const credential = "isolated-fixture-capability-731";
  const outside = path.join(root, "other-profile");
  fs.writeFileSync(outside, credential);
  fs.symlinkSync(outside, path.join(profile, "linked"));
  const captured = path.join(profile, "transcript");
  fs.writeFileSync(captured, "x".repeat(65530) + credential);
  const untouched = path.join(profile, "config");
  fs.writeFileSync(untouched, "another actor's settings");
  const run = () =>
    spawnSync(
      "python3",
      [
        fileURLToPath(
          new URL(
            "../../crates/restlessd/src/purge_capability.py",
            import.meta.url,
          ),
        ),
        profile,
      ],
      {
        env: { ...process.env, RESTLESS_PURGE_SECRET: credential },
        encoding: "utf8",
      },
    );
  assert.equal(run().status, 0);
  assert.equal(fs.readFileSync(captured, "utf8"), "");
  assert.equal(fs.readFileSync(untouched, "utf8"), "another actor's settings");
  assert.equal(fs.readFileSync(outside, "utf8"), credential);
  const large = path.join(profile, "large");
  fs.writeFileSync(large, "x".repeat(4 * 1024 * 1024) + credential);
  assert.notEqual(
    run().status,
    0,
    "Oversized captured state must not be silently cleared or reported clean",
  );
  assert.equal(fs.statSync(large).size, 4 * 1024 * 1024 + credential.length);
});

test("credential cleanup preserves absent-profile success and rejects the size boundary", (t) => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "restless-purge_test-"));
  t.after(() => fs.rmSync(root, { recursive: true, force: true }));
  const credential = "isolated-fixture-capability-947";
  const script = fileURLToPath(
    new URL("../../crates/restlessd/src/purge_capability.py", import.meta.url),
  );
  const run = (profile) =>
    spawnSync("python3", [script, profile], {
      env: { ...process.env, RESTLESS_PURGE_SECRET: credential },
      encoding: "utf8",
    });

  assert.equal(run(path.join(root, "absent-profile")).status, 0);

  const profile = path.join(root, "profile");
  fs.mkdirSync(profile);
  const boundary = path.join(profile, "boundary");
  fs.writeFileSync(
    boundary,
    "x".repeat(4 * 1024 * 1024 - credential.length) + credential,
  );
  assert.notEqual(
    run(profile).status,
    0,
    "A matching file exactly at the limit must be retained and reported",
  );
  assert.equal(fs.statSync(boundary).size, 4 * 1024 * 1024);
});
