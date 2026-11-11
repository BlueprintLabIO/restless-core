#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod/v4";

const server = new McpServer({ name: "restless-coordination-fixture", version: "0.1.0" });
server.registerTool("coord_probe", {
  title: "Coordination probe",
  description: "Return an exact coordination marker for the supplied Work identifier.",
  inputSchema: { workId: z.string() },
  annotations: { readOnlyHint: true, idempotentHint: true, openWorldHint: false },
}, async ({ workId }) => ({
  content: [{ type: "text", text: `WORK-CALLBACK:${workId}:MCP-OK` }],
}));
await server.connect(new StdioServerTransport());
