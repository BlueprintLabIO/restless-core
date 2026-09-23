#!/usr/bin/env node
import fs from "node:fs";
import { spawn, spawnSync } from "node:child_process";
import { harnessRoot, environment } from "./manage.mjs";
const [id, inputPath] = process.argv.slice(2);
const root = harnessRoot(id);
const config = JSON.parse(fs.readFileSync(inputPath, "utf8"));
fs.unlinkSync(inputPath);
if (!Array.isArray(config.setup_command) || !config.setup_command.length)
  throw new Error("No setup command configured");
const command = config.setup_command.map((arg) =>
  arg.replaceAll("${HARNESS_DIR}", root),
);
const env = {
  ...environment(root, config.environment),
  DISPLAY: process.env.DISPLAY ?? ":1",
};
const child = spawn("xterm", ["-T", `${config.name} setup`, "-e", ...command], {
  cwd: root,
  env,
  detached: true,
  stdio: "ignore",
});
let error = null;
child.on("error", (e) => {
  error = e;
});
let opened = false;
for (let i = 0; i < 25; i++) {
  await new Promise((resolve) => setTimeout(resolve, 200));
  if (error || child.exitCode !== null) break;
  const check = spawnSync(
    "xdotool",
    ["search", "--onlyvisible", "--pid", String(child.pid)],
    { env, timeout: 1000, encoding: "utf8" },
  );
  if (check.status === 0 && check.stdout.trim()) {
    opened = true;
    break;
  }
}
if (!opened) {
  try {
    process.kill(-child.pid, "SIGTERM");
  } catch {}
  throw new Error(
    "Could not open the native setup terminal. Check the company desktop and retry.",
  );
}
child.unref();
console.log(JSON.stringify({ state: "setup_opened" }));
