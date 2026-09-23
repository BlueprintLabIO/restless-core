import test from "node:test";
import assert from "node:assert/strict";
import { LegacyModels } from "./legacy-models.mjs";
const response = (id, sessionId, values) => ({
  jsonrpc: "2.0",
  id,
  result: {
    sessionId,
    models: {
      currentModelId: values[0],
      availableModels: values.map((modelId) => ({ modelId })),
    },
  },
});
function create(bridge, id, sessionId, models) {
  bridge.request({ id, method: "session/new", params: {} });
  return bridge.response(response(id, sessionId, models));
}
function select(bridge, id, sessionId, value) {
  return bridge.request({
    id,
    method: "session/set_config_option",
    params: {
      sessionId,
      configId: LegacyModels.optionId,
      value,
    },
  });
}
test("legacy sessions select exact model IDs independently and preserve attribution", () => {
  const bridge = new LegacyModels();
  create(bridge, 1, "alice", ["custom:a:vendor/one", "custom:a:vendor/two"]);
  create(bridge, 2, "bart", ["provider:model"]);
  const request = select(bridge, 3, "alice", "custom:a:vendor/two");
  assert.equal(request.method, "session/set_model");
  assert.deepEqual(request.params, {
    sessionId: "alice",
    modelId: "custom:a:vendor/two",
  });
  const result = bridge.response({ id: 3, result: {} });
  assert.equal(
    result.result.configOptions[0].currentValue,
    "custom:a:vendor/two",
  );
  assert.equal(
    result.result._meta["restless/modelSelection"],
    "legacy-acknowledged",
  );
  assert.ok(select(bridge, 4, "bart", "custom:a:vendor/two").rejected);
});
test("an error acknowledgement cannot masquerade as successful model selection", () => {
  const bridge = new LegacyModels();
  create(bridge, 1, "s", ["one", "two"]);
  select(bridge, 2, "s", "two");
  const failure = {
    id: 2,
    error: { code: -32603, message: "Provider unavailable" },
  };
  assert.equal(bridge.response(failure), failure);
});
test("modern model options and tool notifications remain unchanged", () => {
  const bridge = new LegacyModels();
  bridge.request({ id: 1, method: "session/new", params: {} });
  const modern = {
    id: 1,
    result: {
      sessionId: "s",
      configOptions: [{ id: "vendor-model", category: "model" }],
    },
  };
  assert.equal(bridge.response(modern), modern);
  const tool = { id: 5, method: "session/request_permission", params: {} };
  assert.equal(bridge.response(tool), tool);
});
test("load without a response sessionId keeps its requested session scope", () => {
  const bridge = new LegacyModels();
  bridge.request({
    id: 1,
    method: "session/load",
    params: { sessionId: "persisted" },
  });
  const result = response(1, undefined, ["one", "two"]);
  assert.equal(
    bridge.response(result).result.configOptions[0].currentValue,
    "one",
  );
  assert.equal(
    select(bridge, 2, "persisted", "two").method,
    "session/set_model",
  );
});
