import { createServer } from "node:http";
import { readFileSync } from "node:fs";

const packageVersion = (path) => JSON.parse(readFileSync(path, "utf8")).version;
const claudeAdapterRoot =
  "/usr/local/lib/node_modules/@agentclientprotocol/claude-agent-acp";
const claudeAdapterVersion = packageVersion(
  `${claudeAdapterRoot}/package.json`,
);
const claudeAgentSdkVersion = packageVersion(
  `${claudeAdapterRoot}/node_modules/@anthropic-ai/claude-agent-sdk/package.json`,
);
const claudeCodeVersion = packageVersion(
  `${claudeAdapterRoot}/node_modules/@anthropic-ai/claude-agent-sdk/manifest.json`,
);
const codexVersion = packageVersion(
  "/usr/local/lib/node_modules/@openai/codex/package.json",
);

const integer = (name) => {
  const value = Number.parseInt(process.env[name] ?? "", 10);
  if (!Number.isInteger(value)) throw new Error(`${name} must be an integer`);
  return value;
};

const release = Object.freeze({
  core_version: process.env.RESTLESS_CORE_VERSION,
  source_revision: process.env.RESTLESS_SOURCE_REVISION,
  api_contract_version: integer("RESTLESS_API_CONTRACT_VERSION"),
  assertion_contract_version: integer("RESTLESS_ASSERTION_CONTRACT_VERSION"),
  schema_version: integer("RESTLESS_SCHEMA_VERSION"),
  harnesses: Object.freeze({
    "restless-managed": "omp-18.0.10",
    codex: `codex-cli-${codexVersion}`,
    "claude-agent": `claude-agent-acp-${claudeAdapterVersion}`,
  }),
  harness_agents: Object.freeze({
    "claude-agent": `claude-code-${claudeCodeVersion}`,
  }),
  harness_dependencies: Object.freeze({
    "claude-agent-sdk": claudeAgentSdkVersion,
  }),
});

if (
  !release.core_version ||
  !release.source_revision ||
  release.source_revision === "unknown" ||
  release.harnesses["claude-agent"] !==
    process.env.RESTLESS_CLAUDE_AGENT_ACP_BUILD ||
  release.harness_agents["claude-agent"] !==
    process.env.RESTLESS_CLAUDE_CODE_BUILD ||
  release.harness_dependencies["claude-agent-sdk"] !==
    process.env.RESTLESS_CLAUDE_AGENT_SDK_BUILD
) {
  throw new Error("Runtime image is missing its build-baked release identity");
}

createServer((request, response) => {
  response.setHeader("Content-Type", "application/json");
  if (request.method === "GET" && request.url === "/health") {
    response.writeHead(200);
    response.end(JSON.stringify({ status: "ok", release }));
    return;
  }
  response.writeHead(404);
  response.end(JSON.stringify({ status: "not_found" }));
}).listen(7789, "0.0.0.0");
