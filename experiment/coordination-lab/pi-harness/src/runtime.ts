import { Agent, type AgentEvent, type AgentTool } from "@earendil-works/pi-agent-core";
import { createModels, type AssistantMessage, type Model } from "@earendil-works/pi-ai";
import { openrouterProvider } from "@earendil-works/pi-ai/providers/openrouter";
import { createHash } from "node:crypto";
import type * as acp from "@agentclientprotocol/sdk";
import type { AgentContext } from "@agentclientprotocol/sdk";
import type { LoadedLaunch } from "./launch.js";
import { EventLog } from "./event-log.js";
import { proveFreeToolModel, type FreeModelProof } from "./free-model.js";
import { buildTools } from "./tools.js";
import { McpConnections } from "./mcp.js";

export interface RuntimeResult {
  outcome: "completed" | "cancelled" | "error" | "max_tokens" | "max_turns";
  stopReason: "end_turn" | "cancelled" | "refusal" | "max_tokens" | "max_turn_requests";
  usage: { input: number; output: number; cacheRead: number; cacheWrite: number; cost: number };
  model: string;
  turns: number;
  freeModelProof: FreeModelProof;
  error?: string;
}

function promptText(blocks: acp.ContentBlock[]): string {
  return blocks
    .map((block) => {
      if (block.type === "text") return block.text;
      if (block.type === "resource_link") return `[resource: ${block.name} ${block.uri}]`;
      if (block.type === "resource") return `[embedded resource: ${block.resource.uri}]`;
      return `[unsupported prompt content: ${block.type}]`;
    })
    .join("\n");
}

function usageOf(messages: AssistantMessage[]) {
  return messages.reduce((total, message) => ({
    input: total.input + message.usage.input,
    output: total.output + message.usage.output,
    cacheRead: total.cacheRead + message.usage.cacheRead,
    cacheWrite: total.cacheWrite + message.usage.cacheWrite,
    cost: total.cost + message.usage.cost.total,
  }), { input: 0, output: 0, cacheRead: 0, cacheWrite: 0, cost: 0 });
}

function stringSummary(value: string): string | { bytes: number; sha256: string; preview: string } {
  if (Buffer.byteLength(value) <= 512) return value;
  return {
    bytes: Buffer.byteLength(value),
    sha256: createHash("sha256").update(value).digest("hex"),
    preview: value.slice(0, 256),
  };
}

function valueSummary(value: unknown, depth = 0): unknown {
  if (typeof value === "string") return stringSummary(value);
  if (value === null || typeof value !== "object") return value;
  if (depth >= 4) return { type: Array.isArray(value) ? "array" : "object", truncated: true };
  if (Array.isArray(value)) {
    return value.slice(0, 50).map((item) => valueSummary(item, depth + 1));
  }
  return Object.fromEntries(Object.entries(value as Record<string, unknown>).slice(0, 50)
    .map(([key, item]) => [key, valueSummary(item, depth + 1)]));
}

export class PiRuntime {
  private readonly events: EventLog;
  private readonly models = createModels();
  private agent: Agent | undefined;
  private proof: FreeModelProof | undefined;
  private turns = 0;
  private activeContext: AgentContext | undefined;
  private cancelled = false;
  private turnLimitReached = false;
  private readonly mcp: McpConnections;
  private readonly nativeTools: AgentTool<any>[];
  private bufferedDelta: { type: "text_delta" | "thinking_delta"; text: string } | undefined;

  constructor(private readonly launch: LoadedLaunch) {
    this.models.setProvider(openrouterProvider());
    this.events = new EventLog(launch.contract.eventLog, launch.contract.sessionId, launch.contract.actor.id);
    this.nativeTools = buildTools(launch.contract, this.events);
    this.mcp = new McpConnections(launch.contract, this.events);
  }

  async initialize(): Promise<FreeModelProof> {
    this.proof = await proveFreeToolModel(this.launch.contract.model.id);
    const model = this.models.getModel("openrouter", this.launch.contract.model.id) as Model<any> | undefined;
    if (!model) throw new Error(`Pi OpenRouter catalogue does not contain ${this.launch.contract.model.id}`);
    if (model.cost.input !== 0 || model.cost.output !== 0) throw new Error("Pi catalogue does not mark selected model free");
    this.agent = new Agent({
      initialState: {
        systemPrompt: this.launch.systemPrompt,
        model,
        thinkingLevel: this.launch.contract.model.reasoning ?? "medium",
        tools: this.nativeTools,
        messages: [],
      },
      streamFn: this.models.streamSimple.bind(this.models),
      sessionId: this.launch.contract.sessionId,
      toolExecution: "parallel",
      shouldStopAfterTurn: ({ message }) => {
        this.turns += 1;
        const needsContinuation = message.content.some((item) => item.type === "toolCall");
        if (needsContinuation && this.turns >= this.launch.contract.limits.maxTurns) {
          this.turnLimitReached = true;
          return true;
        }
        return false;
      },
    });
    this.agent.subscribe((event) => this.onEvent(event));
    await this.events.write("launch_verified", {
      contractSha256: this.launch.contractSha256,
      systemPromptSha256: this.launch.systemPromptSha256,
      cwd: this.launch.cwd,
      actor: this.launch.contract.actor,
      tools: this.launch.contract.tools,
      writeScope: this.launch.contract.writeScope,
      model: this.proof,
      limits: this.launch.contract.limits,
    });
    return this.proof;
  }

  async attachMcpServers(servers: acp.McpServer[]): Promise<void> {
    if (!this.agent) throw new Error("runtime is not initialized");
    const tools = await this.mcp.connect(servers);
    const all = [...this.nativeTools, ...tools];
    if (new Set(all.map((tool) => tool.name)).size !== all.length) throw new Error("native and MCP tool names collide");
    this.agent.state.tools = all;
    await this.events.write("tools_materialized", { names: all.map((tool) => tool.name) });
  }

  private async notify(update: acp.SessionUpdate): Promise<void> {
    if (!this.activeContext) return;
    await this.activeContext.notify("session/update", {
      sessionId: this.launch.contract.sessionId,
      update,
    });
  }

  private async onEvent(event: AgentEvent): Promise<void> {
    await this.recordPiEvent(event);
    if (event.type === "message_update") {
      const delta = event.assistantMessageEvent;
      if (delta.type === "text_delta") {
        await this.notify({ sessionUpdate: "agent_message_chunk", content: { type: "text", text: delta.delta } });
      } else if (delta.type === "thinking_delta") {
        await this.notify({ sessionUpdate: "agent_thought_chunk", content: { type: "text", text: delta.delta } });
      }
    } else if (event.type === "tool_execution_start") {
      await this.notify({
        sessionUpdate: "tool_call",
        toolCallId: event.toolCallId,
        title: `${event.toolName}`,
        kind: ["read", "list", "search"].includes(event.toolName) ? "read" : event.toolName === "run" ? "execute" : "edit",
        status: "in_progress",
        rawInput: event.args,
      });
    } else if (event.type === "tool_execution_update") {
      await this.notify({
        sessionUpdate: "tool_call_update",
        toolCallId: event.toolCallId,
        status: "in_progress",
        rawOutput: event.partialResult.details,
      });
    } else if (event.type === "tool_execution_end") {
      await this.notify({
        sessionUpdate: "tool_call_update",
        toolCallId: event.toolCallId,
        status: event.isError ? "failed" : "completed",
        rawOutput: event.result.details,
      });
    }
  }

  private async recordPiEvent(event: AgentEvent): Promise<void> {
    if (event.type === "message_update") {
      const update = event.assistantMessageEvent;
      if (update.type === "text_delta" || update.type === "thinking_delta") {
        if (this.bufferedDelta?.type === update.type && Buffer.byteLength(this.bufferedDelta.text) < 4_096) {
          this.bufferedDelta.text += update.delta;
          return;
        }
        await this.flushPiDelta();
        this.bufferedDelta = { type: update.type, text: update.delta };
        return;
      }
      // Completed tool lifecycle events below already contain the validated
      // arguments. Persisting every growing partialArgs snapshot is quadratic.
      if (update.type === "toolcall_delta") return;
    }
    await this.flushPiDelta();
    await this.events.write(`pi.${event.type}`, this.eventSummary(event));
  }

  private async flushPiDelta(): Promise<void> {
    if (!this.bufferedDelta) return;
    const delta = this.bufferedDelta;
    this.bufferedDelta = undefined;
    await this.events.write("pi.message_delta", delta);
  }

  private eventSummary(event: AgentEvent): unknown {
    const summarizeContent = (content: unknown): unknown[] => {
      if (typeof content === "string") return [{ type: "text", bytes: Buffer.byteLength(content) }];
      if (!Array.isArray(content)) return [];
      return content.map((item: any) => item?.type === "text"
        ? { type: "text", bytes: Buffer.byteLength(String(item.text ?? "")) }
        : { type: String(item?.type ?? "unknown") });
    };
    if (event.type === "message_update") {
      const update = event.assistantMessageEvent as any;
      if (update.type === "text_end" || update.type === "thinking_end") {
        return {
          type: update.type,
          contentIndex: update.contentIndex,
          content: typeof update.content === "string" ? stringSummary(update.content) : undefined,
        };
      }
      if (update.type === "toolcall_end") {
        return {
          type: update.type,
          contentIndex: update.contentIndex,
          toolCall: update.toolCall ? {
            id: update.toolCall.id,
            name: update.toolCall.name,
            arguments: valueSummary(update.toolCall.arguments),
          } : undefined,
        };
      }
      return { type: update.type, contentIndex: update.contentIndex };
    }
    if (event.type === "message_start" || event.type === "message_end") {
      const message = event.message;
      return {
        role: message.role,
        ...(message.role === "assistant" ? {
          model: message.model,
          stopReason: message.stopReason,
          error: message.errorMessage,
          usage: message.usage,
        } : {}),
        content: summarizeContent("content" in message ? message.content : undefined),
      };
    }
    if (event.type === "agent_end") return { messageCount: event.messages.length };
    if (event.type === "turn_end") {
      return {
        stopReason: event.message.role === "assistant" ? event.message.stopReason : undefined,
        toolResults: event.toolResults.map((item) => ({ toolName: item.toolName, isError: item.isError })),
      };
    }
    if (event.type === "tool_execution_start") {
      return {
        type: event.type,
        toolCallId: event.toolCallId,
        toolName: event.toolName,
        args: valueSummary(event.args),
      };
    }
    if (event.type === "tool_execution_update") {
      return {
        type: event.type,
        toolCallId: event.toolCallId,
        toolName: event.toolName,
        partial: valueSummary(event.partialResult.details),
      };
    }
    if (event.type === "tool_execution_end") {
      return {
        type: event.type,
        toolCallId: event.toolCallId,
        toolName: event.toolName,
        isError: event.isError,
        content: (event.result.content as Array<any>).map((item: any) => item.type === "text"
          ? { type: "text", text: item.text.slice(0, 4_000), truncated: item.text.length > 4_000 }
          : { type: item.type }),
      };
    }
    return event;
  }

  async prompt(blocks: acp.ContentBlock[], context: AgentContext): Promise<RuntimeResult> {
    if (!this.agent || !this.proof) throw new Error("runtime is not initialized");
    this.activeContext = context;
    this.turns = 0;
    this.cancelled = false;
    this.turnLimitReached = false;
    const messageOffset = this.agent.state.messages.length;
    const timeout = setTimeout(() => {
      this.cancelled = true;
      this.agent?.abort();
    }, this.launch.contract.limits.timeoutMs);
    try {
      await this.events.write("prompt_start", {
        blocks: blocks.map((block) => block.type === "text"
          ? { type: "text", text: stringSummary(block.text) }
          : valueSummary(block)),
      });
      await this.agent.prompt(promptText(blocks));
      const messages = this.agent.state.messages.slice(messageOffset);
      const last = [...messages].reverse().find((message): message is AssistantMessage => message.role === "assistant");
      const assistants = messages.filter((message): message is AssistantMessage => message.role === "assistant");
      const usage = usageOf(assistants);
      const stopReason = this.cancelled || last?.stopReason === "aborted"
        ? "cancelled"
        : this.turnLimitReached
          ? "max_turn_requests"
        : last?.stopReason === "length"
          ? "max_tokens"
          : last?.stopReason === "error"
            ? "refusal"
            : "end_turn";
      const outcome = stopReason === "cancelled"
        ? "cancelled"
        : stopReason === "max_turn_requests"
          ? "max_turns"
        : stopReason === "max_tokens"
          ? "max_tokens"
          : last?.stopReason === "error" || last?.errorMessage
            ? "error"
            : "completed";
      const result: RuntimeResult = {
        outcome,
        stopReason,
        usage,
        model: this.proof.id,
        turns: this.turns,
        freeModelProof: this.proof,
        ...(last?.errorMessage ? { error: last.errorMessage } : {}),
      };
      await this.events.write("prompt_end", result);
      return result;
    } finally {
      clearTimeout(timeout);
      this.activeContext = undefined;
      await this.flushPiDelta();
      await this.events.flush();
    }
  }

  cancel(): void {
    void this.events.write("cancel_requested", {});
    this.cancelled = true;
    this.agent?.abort();
  }

  async close(): Promise<void> {
    this.cancel();
    await this.mcp.close();
    await this.flushPiDelta();
    await this.events.write("runtime_closed", {});
    await this.events.flush();
  }
}
