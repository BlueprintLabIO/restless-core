// Exercises the patched pinned client against a local in-memory gateway.
import assert from "node:assert/strict";
import { streamPiNative } from "../node_modules/@oh-my-pi/pi-ai/src/providers/pi-native-client";

const runtime = { secret: "synthetic-only", buffer: "x".repeat(80_000) };
const declared = {
  name: "test_tool", description: "A test tool", parameters: { type: "object", properties: {} },
  namespace: "ns", namespaceDescription: "Namespace", strict: true, deferLoading: true,
  customFormat: { syntax: "lark", definition: "start: /x/" }, customWireName: "wire_tool",
  native: { type: "test-marker" }, examples: [{ type: "note", text: "test" }],
};
const tool = { ...declared, tool: runtime, runner: runtime, session: runtime };
const context = { systemPrompt: ["synthetic"], messages: [{ role: "user", content: [{ type: "text", text: "hello" }], timestamp: 0 }], tools: [tool], inactiveTools: [tool] };
const original = JSON.stringify(context);
let sent: any;
const fetch = async (_url: unknown, init: any) => {
  sent = JSON.parse(init.body);
  return new Response(JSON.stringify({ error: { message: "synthetic refusal" } }), { status: 402, headers: { "content-type": "application/json" } });
};
const model: any = { provider: "anthropic", id: "test-model", baseUrl: "http://127.0.0.1:1" };
const stream = streamPiNative(model, context as any, { apiKey: "synthetic", fetch } as any);
await stream.result().catch(() => {});
assert.ok(sent, "gateway received the request");
assert.deepEqual(sent.context.tools, [declared]);
assert.deepEqual(sent.context.inactiveTools, [declared]);
assert.deepEqual(sent.context.messages, context.messages);
assert.deepEqual(sent.context.systemPrompt, context.systemPrompt);
assert.equal(JSON.stringify(context), original, "caller context must remain unchanged");
assert.ok(JSON.stringify(sent).length < original.length / 20, "runtime objects should not cross the wire");
assert.equal(sent.modelId, "anthropic/test-model");
console.log("pi-native tool projection preserved declarations and removed runtime objects");
