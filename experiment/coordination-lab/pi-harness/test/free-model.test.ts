import assert from "node:assert/strict";
import test from "node:test";
import { proveFreeToolModel } from "../src/free-model.js";

const originalFetch = globalThis.fetch;

test.afterEach(() => {
  globalThis.fetch = originalFetch;
});

test("accepts only a live zero-price text tool model", async () => {
  globalThis.fetch = async () => new Response(JSON.stringify({
    data: [{
      id: "test/free",
      name: "Free",
      context_length: 1000,
      pricing: { prompt: "0", completion: "0" },
      supported_parameters: ["tools", "tool_choice"],
      architecture: { input_modalities: ["text"] },
    }],
  })) as typeof fetch;
  const proof = await proveFreeToolModel("test/free");
  assert.equal(proof.id, "test/free");
  assert.equal(proof.promptPrice, "0");
  assert.equal(proof.completionPrice, "0");
});

test("rejects a model when either side has a price", async () => {
  globalThis.fetch = async () => new Response(JSON.stringify({
    data: [{
      id: "test/paid",
      pricing: { prompt: "0", completion: "0.000001" },
      supported_parameters: ["tools"],
      architecture: { input_modalities: ["text"] },
    }],
  })) as typeof fetch;
  await assert.rejects(proveFreeToolModel("test/paid"), /is not free/);
});
