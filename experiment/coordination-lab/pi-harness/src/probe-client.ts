#!/usr/bin/env node
import { spawn } from "node:child_process";
import { Readable, Writable } from "node:stream";
import * as acp from "@agentclientprotocol/sdk";

const launchPath = process.argv[2];
let prompt = process.argv.slice(3).join(" ");
if (!launchPath || !prompt) throw new Error("usage: probe-client <absolute-launch.json> <prompt|@absolute-prompt-path>");
if (prompt.startsWith("@")) prompt = await (await import("node:fs/promises")).readFile(prompt.slice(1), "utf8");
const launch = JSON.parse(await (await import("node:fs/promises")).readFile(launchPath, "utf8")) as {
  cwd: string;
  mcpServers?: Array<{ name: string; command: string; args: string[]; envNames: string[] }>;
};
const child = spawn(process.execPath, [new URL("./acp-agent.js", import.meta.url).pathname], {
  env: { ...process.env, RESTLESS_LAUNCH: launchPath },
  stdio: ["pipe", "pipe", "inherit"],
});
const stream = acp.ndJsonStream(
  Writable.toWeb(child.stdin) as WritableStream<Uint8Array>,
  Readable.toWeb(child.stdout) as unknown as ReadableStream<Uint8Array>,
);
const updates: unknown[] = [];
try {
  const result = await acp.client({ name: "restless-probe" })
    .onRequest("session/request_permission", async () => ({ outcome: { outcome: "cancelled" } }))
    .onRequest("fs/read_text_file", async () => { throw new Error("client filesystem is disabled"); })
    .onRequest("fs/write_text_file", async () => { throw new Error("client filesystem is disabled"); })
    .connectWith(stream, async (ctx) => {
      const initialized = await ctx.request("initialize", {
        protocolVersion: acp.PROTOCOL_VERSION,
        clientCapabilities: { fs: { readTextFile: false, writeTextFile: false } },
        clientInfo: { name: "restless-probe", version: "0.1.0" },
      });
      const mcpServers = (launch.mcpServers ?? []).map((server) => ({
        name: server.name,
        command: server.command,
        args: server.args,
        env: server.envNames.map((name) => ({ name, value: process.env[name] ?? "" })),
      }));
      const response = await ctx.buildSession({ cwd: launch.cwd, mcpServers }).withSession(async (session) => {
        await session.prompt(prompt);
        for (;;) {
          const message = await session.nextUpdate();
          if (message.kind === "stop") return message.response;
          updates.push(message.update);
          if (process.env.PROBE_VERBOSE === "1") process.stderr.write(`${JSON.stringify(message.update)}\n`);
        }
      });
      return { initialized, response };
    });
  const typed = updates as Array<{ sessionUpdate?: string; content?: { type?: string; text?: string } }>;
  const counts = Object.fromEntries([...new Set(typed.map((item) => item.sessionUpdate ?? "unknown"))]
    .map((kind) => [kind, typed.filter((item) => (item.sessionUpdate ?? "unknown") === kind).length]));
  const text = typed
    .filter((item) => item.sessionUpdate === "agent_message_chunk" && item.content?.type === "text")
    .map((item) => item.content?.text ?? "")
    .join("");
  const summary = `${JSON.stringify({ ...result, updateCounts: counts, text }, null, 2)}\n`;
  if (process.env.PROBE_OUTPUT) await (await import("node:fs/promises")).writeFile(process.env.PROBE_OUTPUT, summary);
  process.stdout.write(summary);
} finally {
  child.kill("SIGTERM");
}
