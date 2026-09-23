import test from "node:test";
import assert from "node:assert/strict";
import { GatewayModels } from "./openclaw.mjs";

test("Gateway model selection confirms the exact route and scoped session before acknowledging", async () => {
  const calls = [];
  const bridge = new GatewayModels({
    models: [{ id: "custom/vendor/model:v2", name: "Model" }],
    sessionKey: "agent:restless-a:restless",
    call: async (method, params) => {
      calls.push({ method, params });
      return {
        key: params.key,
        resolved: { modelProvider: "custom", model: "vendor/model:v2" },
      };
    },
  });
  await bridge.request({ id: 1, method: "session/new", params: {} });
  const response = bridge.response({
    id: 1,
    result: { sessionId: "native-session", configOptions: [{ id: "thought" }] },
  });
  const option = response.result.configOptions.find(
    (option) => option.category === "model",
  );
  const select = (params) =>
    bridge.request({
      id: 2,
      method: "session/set_config_option",
      params: { sessionId: "native-session", configId: option.id, ...params },
    });
  assert(
    (
      await select({
        sessionId: "other-session",
        value: "custom/vendor/model:v2",
      })
    ).rejected.error,
  );
  assert((await select({ value: "other/model" })).rejected.error);
  assert.equal(calls.length, 0);
  const selected = await select({ value: "custom/vendor/model:v2" });
  assert.equal(
    selected.rejected.result.configOptions[0].currentValue,
    "custom/vendor/model:v2",
  );
  assert.deepEqual(calls, [
    {
      method: "sessions.patch",
      params: {
        key: "agent:restless-a:restless",
        model: "custom/vendor/model:v2",
      },
    },
  ]);
  bridge.call = async () => ({
    key: "agent:restless-b:restless",
    resolved: { modelProvider: "custom", model: "vendor/model:v2" },
  });
  assert((await select({ value: "custom/vendor/model:v2" })).rejected.error);
  bridge.call = async () => ({
    key: "agent:restless-a:restless",
    resolved: { modelProvider: "custom", model: "different" },
  });
  assert((await select({ value: "custom/vendor/model:v2" })).rejected.error);
  bridge.call = async () => {
    throw new Error("route unavailable");
  };
  assert.equal(
    (await select({ value: "custom/vendor/model:v2" })).rejected.error.message,
    "route unavailable",
  );
});
