import { createHash } from "node:crypto";
import { readFile, realpath } from "node:fs/promises";
import { isAbsolute, resolve } from "node:path";

export type ToolName = "read" | "list" | "search" | "write" | "edit" | "run";

export interface LaunchContract {
  schemaVersion: 1;
  sessionId: string;
  actor: {
    id: string;
    kind: "exec" | "staff";
    role: string;
  };
  cwd: string;
  systemPromptPath: string;
  model: {
    provider: "openrouter";
    id: string;
    reasoning?: "off" | "minimal" | "low" | "medium" | "high" | "xhigh" | "max";
  };
  tools: ToolName[];
  mcpServers?: Array<{
    name: string;
    command: string;
    args: string[];
    envNames: string[];
  }>;
  writeScope: "none" | "workspace";
  limits: {
    maxTurns: number;
    timeoutMs: number;
    commandTimeoutMs: number;
    maxOutputBytes: number;
  };
  eventLog?: string;
}

export interface LoadedLaunch {
  contract: LaunchContract;
  systemPrompt: string;
  systemPromptSha256: string;
  contractSha256: string;
  cwd: string;
}

function requireString(value: unknown, field: string): string {
  if (typeof value !== "string" || !value.trim()) throw new Error(`${field} must be a non-empty string`);
  return value;
}

function requireInteger(value: unknown, field: string, min: number, max: number): number {
  if (!Number.isInteger(value) || Number(value) < min || Number(value) > max) {
    throw new Error(`${field} must be an integer between ${min} and ${max}`);
  }
  return Number(value);
}

export async function loadLaunch(path: string): Promise<LoadedLaunch> {
  if (!isAbsolute(path)) throw new Error("RESTLESS_LAUNCH must be an absolute path");
  const raw = await readFile(path, "utf8");
  const value = JSON.parse(raw) as Partial<LaunchContract>;
  if (value.schemaVersion !== 1) throw new Error("unsupported launch schemaVersion");
  if (!value.actor || !value.model || !value.limits) throw new Error("launch is missing actor, model, or limits");
  if (value.actor.kind !== "exec" && value.actor.kind !== "staff") throw new Error("actor.kind must be exec or staff");
  if (value.model.provider !== "openrouter") throw new Error("scratch harness permits only OpenRouter");
  if (value.writeScope !== "none" && value.writeScope !== "workspace") throw new Error("invalid writeScope");
  const tools = value.tools;
  if (!Array.isArray(tools) || tools.some((tool) => !["read", "list", "search", "write", "edit", "run"].includes(tool))) {
    throw new Error("launch contains an unsupported tool");
  }
  if (new Set(tools).size !== tools.length) throw new Error("launch tools must be unique");
  if (value.writeScope === "none" && tools.some((tool) => tool === "write" || tool === "edit" || tool === "run")) {
    throw new Error("read-only launch cannot expose write, edit, or run");
  }
  const cwdInput = requireString(value.cwd, "cwd");
  const promptPath = requireString(value.systemPromptPath, "systemPromptPath");
  if (!isAbsolute(cwdInput) || !isAbsolute(promptPath)) throw new Error("cwd and systemPromptPath must be absolute");
  const cwd = await realpath(cwdInput);
  const mcpServers = value.mcpServers ?? [];
  if (!Array.isArray(mcpServers)) throw new Error("mcpServers must be an array");
  const normalizedMcp = mcpServers.map((server, index) => {
    if (!server || typeof server !== "object") throw new Error(`mcpServers[${index}] must be an object`);
    const candidate = server as { name?: unknown; command?: unknown; args?: unknown; envNames?: unknown };
    const command = requireString(candidate.command, `mcpServers[${index}].command`);
    if (!isAbsolute(command)) throw new Error(`mcpServers[${index}].command must be absolute`);
    if (!Array.isArray(candidate.args) || candidate.args.some((arg) => typeof arg !== "string")) {
      throw new Error(`mcpServers[${index}].args must contain strings`);
    }
    if (!Array.isArray(candidate.envNames) || candidate.envNames.some((name) => typeof name !== "string")) {
      throw new Error(`mcpServers[${index}].envNames must contain strings`);
    }
    return {
      name: requireString(candidate.name, `mcpServers[${index}].name`),
      command,
      args: [...candidate.args] as string[],
      envNames: [...candidate.envNames].sort() as string[],
    };
  });
  if (new Set(normalizedMcp.map((server) => server.name)).size !== normalizedMcp.length) {
    throw new Error("mcpServers names must be unique");
  }
  const systemPrompt = await readFile(promptPath, "utf8");
  if (!systemPrompt.trim()) throw new Error("system prompt is empty");
  const contract: LaunchContract = {
    schemaVersion: 1,
    sessionId: requireString(value.sessionId, "sessionId"),
    actor: {
      id: requireString(value.actor.id, "actor.id"),
      kind: value.actor.kind,
      role: requireString(value.actor.role, "actor.role"),
    },
    cwd,
    systemPromptPath: resolve(promptPath),
    model: {
      provider: "openrouter",
      id: requireString(value.model.id, "model.id"),
      reasoning: value.model.reasoning ?? "medium",
    },
    tools: [...tools],
    ...(normalizedMcp.length ? { mcpServers: normalizedMcp } : {}),
    writeScope: value.writeScope,
    limits: {
      maxTurns: requireInteger(value.limits.maxTurns, "limits.maxTurns", 1, 100),
      timeoutMs: requireInteger(value.limits.timeoutMs, "limits.timeoutMs", 1_000, 3_600_000),
      commandTimeoutMs: requireInteger(value.limits.commandTimeoutMs, "limits.commandTimeoutMs", 100, 600_000),
      maxOutputBytes: requireInteger(value.limits.maxOutputBytes, "limits.maxOutputBytes", 1_024, 10_000_000),
    },
    ...(value.eventLog ? { eventLog: resolve(requireString(value.eventLog, "eventLog")) } : {}),
  };
  return {
    contract,
    systemPrompt,
    cwd,
    systemPromptSha256: createHash("sha256").update(systemPrompt).digest("hex"),
    contractSha256: createHash("sha256").update(raw).digest("hex"),
  };
}
