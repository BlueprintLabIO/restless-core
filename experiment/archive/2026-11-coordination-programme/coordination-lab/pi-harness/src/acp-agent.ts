#!/usr/bin/env node
import { Readable, Writable } from "node:stream";
import * as acp from "@agentclientprotocol/sdk";
import { loadLaunch } from "./launch.js";
import { PiRuntime } from "./runtime.js";

const launchPath = process.env.RESTLESS_LAUNCH;
if (!launchPath) throw new Error("RESTLESS_LAUNCH is required");
const launch = await loadLaunch(launchPath);
const runtime = new PiRuntime(launch);
await runtime.initialize();
let sessionCreated = false;

const stream = acp.ndJsonStream(
  Writable.toWeb(process.stdout) as WritableStream<Uint8Array>,
  Readable.toWeb(process.stdin) as unknown as ReadableStream<Uint8Array>,
);
acp.agent({ name: "restless-pi-harness" })
  .onRequest("initialize", async (ctx) => ({
    protocolVersion: ctx.params.protocolVersion === acp.PROTOCOL_VERSION ? ctx.params.protocolVersion : acp.PROTOCOL_VERSION,
    agentCapabilities: {
      loadSession: false,
      promptCapabilities: { image: false, audio: false, embeddedContext: true },
      mcpCapabilities: { http: false, sse: false },
    },
    agentInfo: { name: "restless-pi-harness", version: "0.1.0" },
    _meta: {
      "restless.dev/launch": {
        schemaVersion: launch.contract.schemaVersion,
        contractSha256: launch.contractSha256,
        systemPromptSha256: launch.systemPromptSha256,
      },
    },
  }))
  .onRequest("authenticate", async () => ({}))
  .onRequest("session/new", async (ctx) => {
    if (ctx.params.cwd !== launch.cwd) throw new Error(`cwd mismatch: launch=${launch.cwd} request=${ctx.params.cwd}`);
    if (sessionCreated) throw new Error("this bounded harness process accepts one session");
    await runtime.attachMcpServers(ctx.params.mcpServers);
    sessionCreated = true;
    return {
      sessionId: launch.contract.sessionId,
      _meta: { "restless.dev/actor": launch.contract.actor },
    };
  })
  .onRequest("session/prompt", async (ctx) => {
    if (ctx.params.sessionId !== launch.contract.sessionId || !sessionCreated) throw new Error("unknown session");
    const result = await runtime.prompt(ctx.params.prompt, ctx.client);
    return {
      stopReason: result.stopReason,
      _meta: { "restless.dev/result": result },
    };
  })
  .onRequest("session/close", async (ctx) => {
    if (ctx.params.sessionId !== launch.contract.sessionId) throw new Error("unknown session");
    await runtime.close();
    return {};
  })
  .onNotification("session/cancel", async (ctx) => {
    if (ctx.params.sessionId === launch.contract.sessionId) runtime.cancel();
  })
  .connect(stream);
