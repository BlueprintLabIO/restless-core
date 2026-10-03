import test from "node:test";
import assert from "node:assert/strict";
import { randomUUID, generateKeyPairSync } from "node:crypto";
import { validatePlan } from "../scripts/setup.mjs";
import { validateConfig } from "../src/config.mjs";
import { staticPlacement } from "../src/placement-static.mjs";

function plan() {
  return {
    version: 1,
    core_home: "/tmp/sharing_test",
    company: "one_test",
    access: "private",
    owner_email: "owner@example.test",
    account_origin: "https://accounts.example.test",
    core_origin: "https://work.example.test",
    companies: [
      {
        id: "one_test",
        name: "One",
        company_id: randomUUID(),
        cell_id: randomUUID(),
      },
      {
        id: "two_test",
        name: "Two",
        company_id: randomUUID(),
        cell_id: randomUUID(),
      },
    ],
  };
}

test("sharing setup cannot substitute a company or a credential-bearing address", () => {
  const source = plan();
  assert.equal(validatePlan(source), source);
  assert.throws(() => validatePlan({ ...source, company: "unlisted_test" }));
  assert.throws(() =>
    validatePlan({
      ...source,
      companies: [source.companies[0], source.companies[0]],
    }),
  );
  assert.throws(() =>
    validatePlan({
      ...source,
      account_origin: "https://user:secret@accounts.example.test",
    }),
  );
  assert.throws(() =>
    validatePlan({ ...source, account_origin: "https://accounts.other.test" }),
  );
  assert.throws(() =>
    validatePlan({
      ...source,
      companies: [{ ...source.companies[0], id: "../../elsewhere" }],
    }),
  );
});

test("each company on one local account plane keeps its immutable cell scope", async () => {
  const source = plan();
  const { privateKey } = generateKeyPairSync("ed25519");
  const config = validateConfig({
    origin: source.account_origin,
    coreOrigin: source.core_origin,
    ownerId: randomUUID(),
    planeId: randomUUID(),
    companyId: source.companies[0].company_id,
    cellId: source.companies[0].cell_id,
    companyName: "One",
    ownerEmail: source.owner_email,
    secret: "test-secret".repeat(6),
    databaseUrl: "postgresql://test:test@localhost/accounts_test",
    signingKey: {
      ...privateKey.export({ format: "jwk" }),
      kid: "sharing_test",
    },
    companies: source.companies.map((company) => ({
      companyId: company.company_id,
      cellId: company.cell_id,
      companyName: company.name,
    })),
  });
  const placement = staticPlacement(config);
  for (const company of source.companies) {
    const found = await placement.resolve(company.company_id);
    assert.equal(found.companyId, company.company_id);
    assert.equal(found.cellId, company.cell_id);
    assert.equal(found.planeId, config.planeId);
    assert.equal(
      found.ready,
      undefined,
      "readiness must be observed rather than assumed",
    );
  }
  assert.equal(await placement.resolve(randomUUID()), null);
  assert.throws(() =>
    validateConfig({
      ...config,
      companies: [config.companies[0], config.companies[0]],
    }),
  );
});

test("private HTTPS services may share one hostname on separate ports", () => {
  const source = plan();
  assert.equal(
    validatePlan({
      ...source,
      account_origin: "https://host.example.test:8443",
      core_origin: "https://host.example.test",
    }).version,
    1,
  );
});
