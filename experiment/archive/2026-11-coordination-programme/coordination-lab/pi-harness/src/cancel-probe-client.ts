#!/usr/bin/env node
import { spawn } from "node:child_process";
import { readFile } from "node:fs/promises";
import { Readable, Writable } from "node:stream";
import * as acp from "@agentclientprotocol/sdk";

const launchPath = process.argv[2];
if (!launchPath) throw new Error("usage: cancel-probe-client <absolute-launch.json>");
const launch = JSON.parse(await readFile(launchPath, "utf8")) as { cwd: string };
const child = spawn(process.execPath, [new URL("./acp-agent.js", import.meta.url).pathname], {
  env: { ...process.env, RESTLESS_LAUNCH: launchPath },
  stdio: ["pipe", "pipe", "inherit"],
});
const stream = acp.ndJsonStream(
  Writable.toWeb(child.stdin) as WritableStream<Uint8Array>,
  Readable.toWeb(child.stdout) as unknown as ReadableStream<Uint8Array>,
);
const observed: string[] = [];
const startedAt = Date.now();
try {
  const result = await acp.client({ name: "restless-cancel-probe" })
    .onRequest("session/request_permission", async () => ({ outcome: { outcome: "cancelled" } }))
    .onRequest("fs/read_text_file", async () => { throw new Error("client filesystem is disabled"); })
    .onRequest("fs/write_text_file", async () => { throw new Error("client filesystem is disabled"); })
    .connectWith(stream, async (ctx) => {
      await ctx.request("initialize", {
        protocolVersion: acp.PROTOCOL_VERSION,
        clientCapabilities: { fs: { readTextFile: false, writeTextFile: false } },
        clientInfo: { name: "restless-cancel-probe", version: "0.1.0" },
      });
      return await ctx.buildSession({ cwd: launch.cwd, mcpServers: [] }).withSession(async (session) => {
        const promptResult = session.prompt("Start the exact cancellation probe now.");
        let cancellationSentAt: number | undefined;
        for (;;) {
          const message = await session.nextUpdate();
          if (message.kind === "stop") {
            return {
              response: message.response,
              observed,
              elapsedMs: Date.now() - startedAt,
              cancelToStopMs: cancellationSentAt ? Date.now() - cancellationSentAt : null,
            };
          }
          observed.push(message.update.sessionUpdate);
          if (message.update.sessionUpdate === "tool_call" && !cancellationSentAt) {
            cancellationSentAt = Date.now();
            await ctx.notify("session/cancel", { sessionId: session.sessionId });
            const response = await promptResult;
            return {
              response,
              observed,
              elapsedMs: Date.now() - startedAt,
              cancelToStopMs: Date.now() - cancellationSentAt,
            };
          }
        }
      });
    });
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
} finally {
  child.kill("SIGTERM");
}
