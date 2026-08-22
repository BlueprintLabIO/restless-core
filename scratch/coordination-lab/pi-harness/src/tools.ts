import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { lstat, readFile, readdir, realpath, stat, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, isAbsolute, relative, resolve } from "node:path";
import { Type } from "@earendil-works/pi-ai";
import type { AgentTool, AgentToolResult } from "@earendil-works/pi-agent-core";
import type { LaunchContract, ToolName } from "./launch.js";
import type { EventLog } from "./event-log.js";

const RUNTIME_BIN = resolve(dirname(fileURLToPath(import.meta.url)), "../runtime-bin");

function textResult(text: string, details: unknown = {}): AgentToolResult<unknown> {
  return { content: [{ type: "text", text }], details };
}

function bounded(value: string, maxBytes: number): string {
  const source = Buffer.from(value);
  if (source.byteLength <= maxBytes) return value;
  return `${source.subarray(0, maxBytes).toString("utf8")}\n[output truncated at ${maxBytes} bytes]`;
}

function textDigest(value: string) {
  return { bytes: Buffer.byteLength(value), sha256: createHash("sha256").update(value).digest("hex") };
}

function telemetryInput(name: string, params: any): unknown {
  if (name === "write") return { path: params.path, content: textDigest(String(params.content ?? "")) };
  if (name === "edit") return {
    path: params.path,
    oldText: textDigest(String(params.oldText ?? "")),
    newText: textDigest(String(params.newText ?? "")),
  };
  if (name === "run") return {
    command: bounded(String(params.command ?? ""), 2_000),
    timeoutMs: params.timeoutMs,
  };
  return params;
}

function telemetryDetails(name: string, details: any): unknown {
  if (name === "run") return {
    exitCode: details?.exitCode,
    stdout: textDigest(String(details?.stdout ?? "")),
    stderr: textDigest(String(details?.stderr ?? "")),
  };
  return details;
}

async function scopedPath(cwd: string, input: string, mustExist: boolean): Promise<string> {
  const candidate = isAbsolute(input) ? resolve(input) : resolve(cwd, input);
  let checked = candidate;
  if (mustExist) {
    checked = await realpath(candidate);
  } else {
    const parent = await realpath(resolve(candidate, ".."));
    checked = resolve(parent, candidate.split("/").at(-1)!);
  }
  const relation = relative(cwd, checked);
  if (relation.startsWith("..") || isAbsolute(relation)) throw new Error(`path escapes workspace: ${input}`);
  return checked;
}

function readTool(contract: LaunchContract): AgentTool<any> {
  return {
    name: "read",
    label: "Read file",
    description: "Read a UTF-8 file inside the session workspace. Use offset and limit for large files.",
    parameters: Type.Object({
      path: Type.String(),
      offset: Type.Optional(Type.Integer({ minimum: 0 })),
      limit: Type.Optional(Type.Integer({ minimum: 1, maximum: 2000 })),
    }),
    async execute(_id, input) {
      const params = input as { path: string; offset?: number; limit?: number };
      const path = await scopedPath(contract.cwd, params.path, true);
      const info = await stat(path);
      if (!info.isFile()) throw new Error("read target is not a file");
      const lines = (await readFile(path, "utf8")).split("\n");
      const offset = params.offset ?? 0;
      const limit = params.limit ?? 300;
      return textResult(lines.slice(offset, offset + limit).join("\n"), { path, offset, lines: Math.min(limit, lines.length - offset) });
    },
  };
}

function listTool(contract: LaunchContract): AgentTool<any> {
  return {
    name: "list",
    label: "List workspace",
    description: "List files and directories inside the session workspace. Symlinks are shown but never traversed.",
    parameters: Type.Object({
      path: Type.Optional(Type.String()),
      depth: Type.Optional(Type.Integer({ minimum: 0, maximum: 6 })),
      limit: Type.Optional(Type.Integer({ minimum: 1, maximum: 2_000 })),
    }),
    async execute(_id, input) {
      const params = input as { path?: string; depth?: number; limit?: number };
      const target = await scopedPath(contract.cwd, params.path ?? ".", true);
      const info = await stat(target);
      if (!info.isDirectory()) throw new Error("list target is not a directory");
      const maximumDepth = params.depth ?? 2;
      const limit = params.limit ?? 500;
      const entries: string[] = [];
      let truncated = false;
      const visit = async (directory: string, depth: number): Promise<void> => {
        const children = await readdir(directory, { withFileTypes: true });
        children.sort((left, right) => left.name.localeCompare(right.name));
        for (const child of children) {
          if (child.name === ".git") continue;
          if (entries.length >= limit) {
            truncated = true;
            return;
          }
          const childPath = resolve(directory, child.name);
          const childInfo = await lstat(childPath);
          const name = relative(contract.cwd, childPath) || ".";
          entries.push(`${name}${childInfo.isDirectory() ? "/" : childInfo.isSymbolicLink() ? " [symlink]" : ""}`);
          if (childInfo.isDirectory() && !childInfo.isSymbolicLink() && depth < maximumDepth) {
            await visit(childPath, depth + 1);
            if (truncated) return;
          }
        }
      };
      await visit(target, 0);
      return textResult(`${entries.join("\n")}${truncated ? `\n[listing truncated at ${limit} entries]` : ""}`,
        { path: target, entries: entries.length, truncated });
    },
  };
}

function searchTool(contract: LaunchContract): AgentTool<any> {
  return {
    name: "search",
    label: "Search workspace",
    description: "Search text inside a scoped workspace path using ripgrep. Literal matching is the default.",
    parameters: Type.Object({
      query: Type.String({ minLength: 1 }),
      path: Type.Optional(Type.String()),
      glob: Type.Optional(Type.String()),
      regex: Type.Optional(Type.Boolean()),
      limit: Type.Optional(Type.Integer({ minimum: 1, maximum: 2_000 })),
    }),
    async execute(_id, input, signal) {
      const params = input as { query: string; path?: string; glob?: string; regex?: boolean; limit?: number };
      const target = await scopedPath(contract.cwd, params.path ?? ".", true);
      const targetArg = relative(contract.cwd, target) || ".";
      const args = ["--line-number", "--no-heading", "--color", "never", "--max-filesize", "2M"];
      if (!params.regex) args.push("--fixed-strings");
      if (params.glob) args.push("--glob", params.glob);
      args.push("--", params.query, targetArg);
      const output = await new Promise<string>((accept, reject) => {
        const child = spawn("rg", args, {
          cwd: contract.cwd,
          env: { PATH: process.env.PATH ?? "/usr/bin:/bin", LANG: process.env.LANG ?? "C.UTF-8", NO_COLOR: "1" },
          stdio: ["ignore", "pipe", "pipe"],
        });
        let stdout = "";
        let stderr = "";
        const abort = () => child.kill("SIGTERM");
        signal?.addEventListener("abort", abort, { once: true });
        child.stdout.on("data", (chunk: Buffer) => { stdout = bounded(stdout + chunk.toString("utf8"), contract.limits.maxOutputBytes); });
        child.stderr.on("data", (chunk: Buffer) => { stderr = bounded(stderr + chunk.toString("utf8"), 4_000); });
        child.once("error", reject);
        child.once("close", (code) => {
          signal?.removeEventListener("abort", abort);
          if (signal?.aborted) return reject(new Error("search cancelled"));
          if (code !== 0 && code !== 1) return reject(new Error(`search failed with ${code}: ${stderr}`));
          accept(stdout);
        });
      });
      const limit = params.limit ?? 200;
      const lines = output.split("\n").filter(Boolean);
      const truncated = lines.length > limit;
      const selected = lines.slice(0, limit);
      return textResult(`${selected.join("\n")}${truncated ? `\n[search truncated at ${limit} matches]` : ""}` || "no matches",
        { path: target, matches: selected.length, truncated });
    },
  };
}

function writeTool(contract: LaunchContract): AgentTool<any> {
  return {
    name: "write",
    label: "Write file",
    description: "Write a complete UTF-8 file inside the session workspace.",
    parameters: Type.Object({ path: Type.String(), content: Type.String() }),
    async execute(_id, input) {
      const params = input as { path: string; content: string };
      const path = await scopedPath(contract.cwd, params.path, false);
      await writeFile(path, params.content, "utf8");
      return textResult(`wrote ${Buffer.byteLength(params.content)} bytes to ${relative(contract.cwd, path)}`, { path });
    },
  };
}

function editTool(contract: LaunchContract): AgentTool<any> {
  return {
    name: "edit",
    label: "Edit file",
    description: "Replace one exact occurrence of oldText with newText in a UTF-8 workspace file.",
    parameters: Type.Object({ path: Type.String(), oldText: Type.String(), newText: Type.String() }),
    async execute(_id, input) {
      const params = input as { path: string; oldText: string; newText: string };
      const path = await scopedPath(contract.cwd, params.path, true);
      const source = await readFile(path, "utf8");
      const first = source.indexOf(params.oldText);
      if (first < 0) throw new Error("oldText was not found");
      if (source.indexOf(params.oldText, first + params.oldText.length) >= 0) throw new Error("oldText is not unique");
      const output = `${source.slice(0, first)}${params.newText}${source.slice(first + params.oldText.length)}`;
      await writeFile(path, output, "utf8");
      return textResult(`edited ${relative(contract.cwd, path)}`, { path });
    },
  };
}

function runTool(contract: LaunchContract): AgentTool<any> {
  return {
    name: "run",
    label: "Run command",
    description: "Run one command in the session workspace. Shell syntax is supported. The child receives no model-provider credentials.",
    parameters: Type.Object({ command: Type.String(), timeoutMs: Type.Optional(Type.Integer({ minimum: 100, maximum: contract.limits.commandTimeoutMs })) }),
    executionMode: "sequential",
    async execute(_id, input, signal, onUpdate) {
      const params = input as { command: string; timeoutMs?: number };
      const timeoutMs = Math.min(params.timeoutMs ?? contract.limits.commandTimeoutMs, contract.limits.commandTimeoutMs);
      return await new Promise<AgentToolResult<unknown>>((accept, reject) => {
        const child = spawn("/bin/zsh", ["-lc", params.command], {
          cwd: contract.cwd,
          detached: process.platform !== "win32",
          env: {
            PATH: `${RUNTIME_BIN}:${process.env.PATH ?? "/usr/bin:/bin"}`,
            LANG: process.env.LANG ?? "C.UTF-8",
            LC_ALL: process.env.LC_ALL ?? "C.UTF-8",
            HOME: contract.cwd,
            TERM: "dumb",
            NO_COLOR: "1",
          },
          stdio: ["ignore", "pipe", "pipe"],
        });
        let stdout = "";
        let stderr = "";
        let settled = false;
        const finish = (error?: Error) => {
          if (settled) return;
          settled = true;
          clearTimeout(timer);
          signal?.removeEventListener("abort", abort);
          if (error) return reject(error);
          const output = bounded(`${stdout}${stderr ? `\n[stderr]\n${stderr}` : ""}`, contract.limits.maxOutputBytes);
          accept(textResult(output || `(exit ${child.exitCode ?? 0}, no output)`, { exitCode: child.exitCode, stdout, stderr }));
        };
        const abort = () => {
          if (child.pid && process.platform !== "win32") {
            try { process.kill(-child.pid, "SIGTERM"); } catch { child.kill("SIGTERM"); }
            setTimeout(() => {
              try { process.kill(-child.pid!, "SIGKILL"); } catch { child.kill("SIGKILL"); }
            }, 1000).unref();
          } else {
            child.kill("SIGTERM");
            setTimeout(() => child.kill("SIGKILL"), 1000).unref();
          }
        };
        const timer = setTimeout(() => {
          abort();
          finish(new Error(`command timed out after ${timeoutMs}ms`));
        }, timeoutMs);
        signal?.addEventListener("abort", abort, { once: true });
        if (signal?.aborted) abort();
        child.once("spawn", () => {
          if (signal?.aborted) abort();
        });
        child.stdout.on("data", (chunk: Buffer) => {
          stdout = bounded(stdout + chunk.toString("utf8"), contract.limits.maxOutputBytes);
          onUpdate?.(textResult(bounded(chunk.toString("utf8"), 4_000), { stream: "stdout" }));
        });
        child.stderr.on("data", (chunk: Buffer) => {
          stderr = bounded(stderr + chunk.toString("utf8"), contract.limits.maxOutputBytes);
          onUpdate?.(textResult(bounded(chunk.toString("utf8"), 4_000), { stream: "stderr" }));
        });
        child.once("error", (error) => finish(error));
        child.once("close", (code, killedBy) => {
          if (signal?.aborted) return finish(new Error("command cancelled"));
          if (code !== 0) return finish(new Error(`command failed with ${code ?? killedBy}: ${bounded(stderr || stdout, 4000)}`));
          finish();
        });
      });
    },
  };
}

export function buildTools(contract: LaunchContract, events: EventLog): AgentTool<any>[] {
  const factories: Record<ToolName, () => AgentTool<any>> = {
    read: () => readTool(contract),
    list: () => listTool(contract),
    search: () => searchTool(contract),
    write: () => writeTool(contract),
    edit: () => editTool(contract),
    run: () => runTool(contract),
  };
  return contract.tools.map((name) => {
    const tool = factories[name]();
    const execute = tool.execute.bind(tool);
    return {
      ...tool,
      async execute(id, params, signal, update) {
        await events.write("tool_execute_enter", {
          id,
          name,
          params: telemetryInput(name, params),
          hasSignal: Boolean(signal),
          signalAborted: signal?.aborted ?? null,
        });
        try {
          const result = await execute(id, params, signal, update);
          await events.write("tool_execute_exit", { id, name, isError: false, details: telemetryDetails(name, result.details) });
          return result;
        } catch (error) {
          await events.write("tool_execute_exit", { id, name, isError: true, error: String(error) });
          throw error;
        }
      },
    };
  });
}
