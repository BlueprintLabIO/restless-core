/** One offline protocol smoke for the pinned OMP Responses gateway patch. */
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import {
	encodeResponse,
	encodeStream,
	parseRequest,
} from "./node_modules/@oh-my-pi/pi-ai/src/providers/openai-responses-server.ts";
import {
	convertCodexResponsesMessages,
	convertOpenAICodexResponsesTools,
	normalizeCodexToolChoice,
} from "./node_modules/@oh-my-pi/pi-ai/src/providers/openai-codex-responses.ts";
import { convertTools, mapOpenAIResponsesToolChoiceForTools } from "./node_modules/@oh-my-pi/pi-ai/src/providers/openai-responses.ts";
import { AssistantMessageEventStream } from "./node_modules/@oh-my-pi/pi-ai/src/utils/event-stream.ts";

const sourceCli = spawnSync(fileURLToPath(new URL("./omp-source", import.meta.url)), ["--version"], { encoding: "utf8" });
assert.equal(sourceCli.status, 0, sourceCli.stderr);
assert.match(sourceCli.stdout, /^omp\/18\.3\.2\s*$/);

const definition = (namespace: string, deferred = false) => ({
	type: "namespace" as const,
	name: namespace,
	description: `Tools in ${namespace}`,
	tools: [{
		type: "function" as const,
		name: "lookup",
		description: "Find an entry",
		parameters: { type: "object", properties: { id: { type: "string" } }, required: ["id"] },
		strict: false,
		...(deferred ? { defer_loading: true } : {}),
	}],
});

const request = parseRequest({
	model: "gpt-6-sol",
	input: [
		{ type: "additional_tools", role: "developer", tools: [definition("mcp__beta", true)] },
		{ type: "function_call", id: "fc_a", call_id: "call_a", namespace: "mcp__alpha", name: "lookup", arguments: '{"id":"a"}' },
		{ type: "function_call_output", call_id: "call_a", output: "A" },
		{ type: "function_call", id: "fc_b", call_id: "call_b", namespace: "mcp__beta", name: "lookup", arguments: '{"id":"b"}' },
		{ type: "function_call_output", call_id: "call_b", output: "B" },
	],
	tools: [{ type: "function", name: "lookup", parameters: { type: "object", properties: {} } }, definition("mcp__alpha")],
	tool_choice: { type: "function", name: "lookup", namespace: "mcp__beta" },
});

assert.deepEqual(request.context.tools?.map(tool => [tool.namespace ?? "", tool.name]), [
	["", "lookup"], ["mcp__alpha", "lookup"], ["mcp__beta", "lookup"],
]);
assert.deepEqual(request.options.toolChoice, { name: "lookup", namespace: "mcp__beta" });
const alternateChildList = parseRequest({
	model: "gpt-6-sol",
	input: "hi",
	tools: [{ type: "namespace", name: "mcp__alternate", functions: definition("mcp__alternate").tools }],
});
assert.equal(alternateChildList.context.tools?.[0]?.namespace, "mcp__alternate");
assert.deepEqual(
	request.context.messages.filter(message => message.role === "assistant").flatMap(message => message.content)
		.filter(part => part.type === "toolCall").map(call => [call.namespace, call.name, call.id]),
	[["mcp__alpha", "lookup", "call_a"], ["mcp__beta", "lookup", "call_b"]],
);

const codexModel = { applyPatchToolType: "freeform", supportsComputerUse: false } as any;
const codexTools = convertOpenAICodexResponsesTools(request.context.tools!, codexModel);
assert.deepEqual(normalizeCodexToolChoice({ type: "tool", name: "lookup", namespace: "mcp__beta" }, request.context.tools!, codexModel),
	{ type: "function", name: "lookup", namespace: "mcp__beta" });
assert.deepEqual(codexTools.map(tool => [tool.type, tool.name]), [
	["function", "lookup"], ["namespace", "mcp__alpha"], ["namespace", "mcp__beta"],
]);
assert.equal((codexTools[2] as any).tools[0].defer_loading, undefined);
const replay = convertCodexResponsesMessages({
	...codexModel,
	id: "gpt-6-sol",
	provider: "openai",
	api: "openai-codex-responses",
	input: ["text"],
	identity: { class: "other" },
	compat: { supportsImageDetailOriginal: true },
}, request.context);
assert.deepEqual(replay.filter(item => item.type === "function_call").map(item => [item.namespace, item.name]), [
	["mcp__alpha", "lookup"], ["mcp__beta", "lookup"],
]);

const apiModel = {
	applyPatchToolType: "freeform",
	supportsComputerUse: false,
	compat: { rejectRootObjectUnion: false, supportsToolChoice: true, supportsForcedToolChoice: true },
} as any;
const apiTools = convertTools(request.context.tools!, false, apiModel);
assert.deepEqual(mapOpenAIResponsesToolChoiceForTools({ type: "tool", name: "lookup", namespace: "mcp__beta" }, request.context.tools!, apiModel),
	{ type: "function", name: "lookup", namespace: "mcp__beta" });
assert.deepEqual(apiTools.map(tool => [tool.type, "name" in tool ? tool.name : ""]), [
	["function", "lookup"], ["namespace", "mcp__alpha"], ["namespace", "mcp__beta"],
]);
assert.equal((apiTools[2] as any).tools[0].defer_loading, undefined);

const usage = {
	input: 0, output: 0, cacheRead: 0, cacheWrite: 0, totalTokens: 0,
	cost: { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, total: 0 },
};
const message = {
	role: "assistant" as const,
	content: [{ type: "toolCall" as const, id: "call_c", namespace: "mcp__beta", name: "lookup", arguments: { id: "c" } }],
	api: "openai-responses" as const,
	provider: "openai",
	model: "gpt-6-sol",
	usage,
	stopReason: "toolUse" as const,
	timestamp: Date.now(),
};
const output = encodeResponse(message, "gpt-6-sol").output as Array<Record<string, unknown>>;
assert.deepEqual([output[0]?.namespace, output[0]?.name], ["mcp__beta", "lookup"]);

const events = new AssistantMessageEventStream();
events.push({ type: "start", partial: message });
events.push({ type: "toolcall_start", contentIndex: 0, partial: message });
events.push({ type: "toolcall_delta", contentIndex: 0, delta: '{"id":"c"}', partial: message });
events.push({ type: "toolcall_end", contentIndex: 0, toolCall: message.content[0], partial: message });
events.push({ type: "done", reason: "toolUse", message });
const wire = await new Response(encodeStream(events, "gpt-6-sol")).text();
const items = wire.split("\n").filter(line => line.startsWith("data: {")).map(line => JSON.parse(line.slice(6)))
	.filter(event => event.type === "response.output_item.added" || event.type === "response.output_item.done");
assert.deepEqual(items.map(event => [event.item.namespace, event.item.name]), [
	["mcp__beta", "lookup"], ["mcp__beta", "lookup"],
]);

console.log("OMP namespace protocol smoke passed: definitions, history replay, forced choice, JSON, and SSE");
