import type * as acp from "@agentclientprotocol/sdk";
import type { AgentTool } from "@earendil-works/pi-agent-core";
import type { ImageContent, TextContent } from "@earendil-works/pi-ai";
import { Client } from "@modelcontextprotocol/sdk/client/index.js";
import { StdioClientTransport } from "@modelcontextprotocol/sdk/client/stdio.js";
import type { EventLog } from "./event-log.js";
import type { LaunchContract } from "./launch.js";

interface ConnectedServer {
  name: string;
  client: Client;
  transport: StdioClientTransport;
}

function stdio(server: acp.McpServer): server is acp.McpServerStdio {
  return "command" in server && !("type" in server);
}

function exactServer(contract: LaunchContract, server: acp.McpServerStdio) {
  const expected = contract.mcpServers?.find((item) => item.name === server.name);
  if (!expected) throw new Error(`MCP server ${server.name} is not in the launch contract`);
  const envNames = server.env.map((item) => item.name).sort();
  if (server.command !== expected.command || JSON.stringify(server.args) !== JSON.stringify(expected.args)) {
    throw new Error(`MCP server ${server.name} command or args differ from the launch contract`);
  }
  if (JSON.stringify(envNames) !== JSON.stringify(expected.envNames)) {
    throw new Error(`MCP server ${server.name} environment names differ from the launch contract`);
  }
  return expected;
}

export class McpConnections {
  private readonly servers: ConnectedServer[] = [];

  constructor(private readonly contract: LaunchContract, private readonly events: EventLog) {}

  async connect(configurations: acp.McpServer[]): Promise<AgentTool<any>[]> {
    const expectedCount = this.contract.mcpServers?.length ?? 0;
    if (configurations.length !== expectedCount) {
      throw new Error(`MCP server count mismatch: expected ${expectedCount}, got ${configurations.length}`);
    }
    const tools: AgentTool<any>[] = [];
    const names = new Set<string>();
    for (const configuration of configurations) {
      if (!stdio(configuration)) throw new Error("first-party harness currently supports ACP stdio MCP servers only");
      exactServer(this.contract, configuration);
      const env = Object.fromEntries(configuration.env.map((item) => [item.name, item.value]));
      const transport = new StdioClientTransport({
        command: configuration.command,
        args: configuration.args,
        env,
        cwd: this.contract.cwd,
        stderr: "pipe",
      });
      const client = new Client({ name: `restless-${configuration.name}`, version: "0.1.0" });
      await client.connect(transport);
      this.servers.push({ name: configuration.name, client, transport });
      const listed = await client.listTools();
      await this.events.write("mcp_connected", {
        name: configuration.name,
        command: configuration.command,
        args: configuration.args,
        envNames: configuration.env.map((item) => item.name).sort(),
        tools: listed.tools.map((tool) => tool.name),
      });
      for (const tool of listed.tools) {
        if (names.has(tool.name)) throw new Error(`duplicate MCP tool name ${tool.name}`);
        names.add(tool.name);
        tools.push({
          name: tool.name,
          label: tool.title ?? tool.name,
          description: tool.description ?? `MCP tool from ${configuration.name}`,
          parameters: tool.inputSchema as any,
          async execute(_id, params, signal) {
            const result = await client.callTool(
              { name: tool.name, arguments: params as Record<string, unknown> },
              undefined,
              { signal },
            ) as { content: Array<{ type: string; text?: string; data?: string; mimeType?: string }>; isError?: boolean };
            const text = result.content.map((item) => item.type === "text" ? item.text : JSON.stringify(item)).join("\n");
            if (result.isError) throw new Error(text || `MCP tool ${tool.name} failed`);
            const content: Array<TextContent | ImageContent> = [];
            for (const item of result.content) {
              if (item.type === "text") content.push({ type: "text", text: item.text ?? "" });
              else if (item.type === "image" && item.data && item.mimeType) content.push({ type: "image", data: item.data, mimeType: item.mimeType });
              else content.push({ type: "text", text: JSON.stringify(item) });
            }
            return { content, details: { server: configuration.name, result } };
          },
        });
      }
    }
    return tools;
  }

  async close(): Promise<void> {
    await Promise.allSettled(this.servers.map(async (server) => {
      await server.client.close();
    }));
    this.servers.length = 0;
  }
}
