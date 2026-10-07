// The fixture as a local (stdio) MCP server, dependency-free so it runs in the
// company image unchanged: Restless's host-side local worker starts it as
// `node -e <this file>`. Newline-delimited JSON-RPC, per the MCP stdio
// transport. FIXTURE_TOKEN arrives from a credential reference; without it
// every tool refuses, so the smoke can see that the reference was resolved.
const readline = require('node:readline');
const fs = require('node:fs');
const path = require('node:path');
const http = require('node:http');
const notes = [{ id: 'n1', to: 'ada@example.test', body: 'Kickoff notes' }];
let sent = 0;
const tools = [
  { name: 'whoami', description: 'The signed-in Fixture Notes account.', inputSchema: { type: 'object', properties: {} }, annotations: { readOnlyHint: true } },
  { name: 'list_notes', description: 'List notes in the account.', inputSchema: { type: 'object', properties: {} }, annotations: { readOnlyHint: true } },
  { name: 'send_note', description: 'Send a note to someone. It reaches them.', inputSchema: { type: 'object', properties: { to: { type: 'string' }, body: { type: 'string' } }, required: ['to', 'body'] } },
  { name: 'delete_note', description: 'Permanently delete a note.', inputSchema: { type: 'object', properties: { id: { type: 'string' } }, required: ['id'] }, annotations: { destructiveHint: true } },
  // The tool's own data, kept across runs in RESTLESS_TOOL_DATA.
  { name: 'remember', description: 'Keep a line in this tool’s own data.', inputSchema: { type: 'object', properties: { text: { type: 'string' } }, required: ['text'] }, annotations: { readOnlyHint: true } },
  { name: 'recall', description: 'Read what this tool kept.', inputSchema: { type: 'object', properties: {} }, annotations: { readOnlyHint: true } },
  // The company browser, reachable only when the owner lets this tool use it.
  { name: 'browser_version', description: 'Ask the company browser for its version.', inputSchema: { type: 'object', properties: {} }, annotations: { readOnlyHint: true } },
];
const memory = () => path.join(process.env.RESTLESS_TOOL_DATA || '/nonexistent', 'memory.txt');
function browserVersion() {
  return new Promise((resolve) => {
    const base = process.env.RESTLESS_BROWSER_CDP;
    if (!base) return resolve(text({ error: 'no company browser: RESTLESS_BROWSER_CDP is unset' }, true));
    http.get(`${base}/json/version`, (response) => {
      let body = '';
      response.on('data', (chunk) => (body += chunk));
      response.on('end', () => {
        try { resolve(text({ browser: JSON.parse(body).Browser })); } catch { resolve(text({ error: 'unreadable', body }, true)); }
      });
    }).on('error', (error) => resolve(text({ error: error.message }, true)));
  });
}
function reply(id, result) { process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, result }) + '\n'); }
function fail(id, code, message) { process.stdout.write(JSON.stringify({ jsonrpc: '2.0', id, error: { code, message } }) + '\n'); }
function text(value, isError = false) { return { content: [{ type: 'text', text: JSON.stringify(value) }], isError }; }
function call(name, args) {
  if (!process.env.FIXTURE_TOKEN) return text({ error: 'no FIXTURE_TOKEN: the credential reference was not resolved' }, true);
  if (name === 'whoami') return text({ account: 'fixture-owner', transport: 'stdio' });
  if (name === 'list_notes') return text({ notes });
  if (name === 'send_note') { sent += 1; return text({ message_id: `s_${sent}`, to: args.to }); }
  if (name === 'delete_note') return text({ deleted: args.id });
  if (name === 'remember') {
    try { fs.writeFileSync(memory(), String(args.text)); return text({ kept: true }); } catch (error) { return text({ error: error.message }, true); }
  }
  if (name === 'recall') {
    try { return text({ text: fs.readFileSync(memory(), 'utf8') }); } catch (error) { return text({ error: error.message }, true); }
  }
  if (name === 'browser_version') return browserVersion();
  return null;
}
readline.createInterface({ input: process.stdin }).on('line', (line) => {
  let message;
  try { message = JSON.parse(line); } catch { return; }
  const { id, method, params } = message;
  if (id === undefined) return; // notifications
  if (method === 'initialize') return reply(id, { protocolVersion: params.protocolVersion, capabilities: { tools: {} }, serverInfo: { name: 'fixture-notes-stdio', version: '1.0.0' } });
  if (method === 'ping') return reply(id, {});
  if (method === 'tools/list') return reply(id, { tools });
  if (method === 'tools/call') {
    return Promise.resolve(call(params.name, params.arguments ?? {})).then((result) =>
      result ? reply(id, result) : fail(id, -32602, `unknown tool ${params.name}`));
  }
  fail(id, -32601, `method ${method} not found`);
});
