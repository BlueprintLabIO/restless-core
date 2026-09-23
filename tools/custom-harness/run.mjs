#!/usr/bin/env node
// One scoped Restless launch. Provider setup is private to the selected harness; actor grants are per launch.
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { spawn } from "node:child_process";
import { StringDecoder } from "node:string_decoder";
import { harnessRoot, environment } from "./manage.mjs";
import { LegacyModels } from "./legacy-models.mjs";
import { openGateway } from "./openclaw.mjs";
export async function run({ root, systemPromptPath, config }) {
  if (!["acp", "hermes-acp", "openclaw-gateway"].includes(config.adapter))
    throw new Error("This harness requires a dedicated adapter");
  if (!Array.isArray(config.command) || !config.command.length)
    throw new Error("Missing custom ACP command");
  let command = config.command.map((arg) =>
    arg
      .replaceAll("${HARNESS_DIR}", root)
      .replaceAll("${SYSTEM_PROMPT_FILE}", systemPromptPath),
  );
  let env = { ...process.env, ...environment(root, config.env) };
  // Keep only the host-issued launch identity. Custom profile settings cannot substitute these names.
  for (const name of [
    "RESTLESS_ACTOR",
    "RESTLESS_COORDINATOR",
    "RESTLESS_SESSION_CAPABILITY",
    "RESTLESS_COORDINATION_WAKE",
  ]) {
    if (process.env[name] !== undefined) env[name] = process.env[name];
    else delete env[name];
  }
  if (
    config.adapter === "acp" &&
    !config.command.some((arg) => arg.includes("${SYSTEM_PROMPT_FILE}"))
  )
    throw new Error(
      "Configure the custom command to accept ${SYSTEM_PROMPT_FILE} as its system-instruction file",
    );
  const context = fs.readFileSync(systemPromptPath, "utf8");
  if (config.adapter === "hermes-acp") {
    const project = path.join(root, "home/.hermes/hermes-agent");
    command = [
      path.join(project, "venv/bin/python"),
      path.join(path.dirname(fileURLToPath(import.meta.url)), "hermes.py"),
      project,
      systemPromptPath,
    ];
  }
  const gateway =
    config.adapter === "openclaw-gateway"
      ? await openGateway({ root, command, env, context })
      : null;
  if (gateway) {
    command = gateway.command;
    env = gateway.env;
  }
  const child = spawn(command[0], command.slice(1), {
    env,
    cwd: process.cwd(),
    stdio: ["pipe", "pipe", "inherit"],
  });
  const bridge = gateway?.bridge ?? new LegacyModels();
  const write = (stream, message) =>
    stream.write(JSON.stringify(message) + "\n");
  let failed = false;
  function fail() {
    if (failed) return;
    failed = true;
    child.kill("SIGTERM");
    process.exitCode = 1;
    process.stdin.destroy();
  }
  function read(stream, receive) {
    const decoder = new StringDecoder("utf8");
    let buffer = "";
    stream.on("data", (chunk) => {
      buffer += decoder.write(chunk);
      let end;
      try {
        while ((end = buffer.indexOf("\n")) >= 0) {
          const line = buffer.slice(0, end);
          buffer = buffer.slice(end + 1);
          if (Buffer.byteLength(line) > 16 * 1024 * 1024)
            throw new Error("ACP frame too large");
          if (line.trim()) receive(JSON.parse(line));
        }
        if (Buffer.byteLength(buffer) > 16 * 1024 * 1024)
          throw new Error("ACP frame too large");
      } catch {
        fail();
      }
    });
    stream.on("error", fail);
    stream.on("end", () => {
      buffer += decoder.end();
      if (buffer.trim()) fail();
    });
  }
  let requests = Promise.resolve();
  read(process.stdin, (message) => {
    requests = requests
      .then(async () => {
        const adapted = await bridge.request(message);
        if (adapted.rejected) write(process.stdout, adapted.rejected);
        else write(child.stdin, adapted);
      })
      .catch(fail);
  });
  read(child.stdout, (message) =>
    write(process.stdout, bridge.response(message)),
  );
  process.stdin.on("end", () => child.stdin.end());
  child.stdin.on("error", fail);
  child.on("error", fail);
  const done = new Promise((resolve) =>
    child.on("close", (code) => {
      if (code) process.exitCode = code;
      process.stdin.destroy();
      resolve();
    }),
  );
  for (const signal of ["SIGTERM", "SIGINT"])
    process.once(signal, () => {
      child.kill(signal);
      process.stdin.destroy();
    });

  try {
    await done;
  } finally {
    await gateway?.close();
  }
}
if (
  process.argv[1] &&
  path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  const [id, systemPromptPath] = process.argv.slice(2);
  const config = JSON.parse(process.env.RESTLESS_CUSTOM_CONFIG ?? "{}");
  delete process.env.RESTLESS_CUSTOM_CONFIG;
  await run({ root: harnessRoot(id), systemPromptPath, config });
}
