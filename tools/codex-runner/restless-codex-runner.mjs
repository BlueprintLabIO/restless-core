#!/usr/bin/env node

// Restless's neutral first-party Codex session transport.
//
// stdin and stdout are newline-delimited JSON. The controller sends one
// `launch` operation, then `turn`, `steer`, `interrupt`, `ping`, or `shutdown`
// operations. The runner owns only documented Codex app-server mechanics; it
// does not plan, supervise, evaluate, or reinterpret task outcomes. That keeps
// the exact same byte-addressable runner usable by a solo benchmark controller
// and by a Restless-supervised worker.

import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { createInterface } from 'node:readline';

const PROTOCOL_VERSION = 1;
const CLIENT = {
  name: 'restless-codex-runner',
  title: 'Restless first-party Codex runner',
  version: String(PROTOCOL_VERSION),
};
const ALLOWED_OPS = new Set(['launch', 'turn', 'steer', 'interrupt', 'ping', 'shutdown']);
const ALLOWED_EFFORTS = new Set(['none', 'low', 'medium', 'high', 'xhigh', 'max', 'ultra']);
const MODEL_CAPABILITY_ENV = 'RESTLESS_MODEL_CAPABILITY';
const DISABLED_CODEX_FEATURES = [
  'multi_agent',
  'plugins',
  'remote_plugin',
  'plugin_sharing',
  'skill_search',
  'skill_mcp_dependency_install',
  'apps',
  'browser_use',
  'browser_use_external',
  'browser_use_full_cdp_access',
  'computer_use',
  'image_generation',
  'in_app_browser',
];
const DENIED_TASK_PROXY = 'http://127.0.0.1:9';
const MODEL_RELAY_NO_PROXY = 'host.docker.internal,127.0.0.1,localhost';

let appServer = null;
let appInput = null;
let nextRpcId = 1;
let launched = false;
let ready = false;
let threadId = null;
let activeTurnId = null;
let turnStarting = false;
let pendingTurnOperation = null;
let observed = null;
let stderrTail = '';
const pending = new Map();
const completedOperations = new Map();

function redactSensitive(value) {
  let redacted = String(value ?? '');
  const capability = process.env[MODEL_CAPABILITY_ENV];
  if (capability) redacted = redacted.split(capability).join('[REDACTED]');
  return redacted
    .replace(/(authorization\s*[:=]\s*bearer\s+)[^\s"']+/gi, '$1[REDACTED]')
    .replace(/([?&](?:api_?key|token)=)[^&\s]+/gi, '$1[REDACTED]');
}

function emit(value) {
  process.stdout.write(`${JSON.stringify(value)}\n`);
}

function fail(message, details = null) {
  emit({ type: 'runner_error', message, details });
}

function requireString(value, name) {
  if (typeof value !== 'string' || value.trim() === '') {
    throw new Error(`${name} must be a non-empty string`);
  }
  return value;
}

function providerModel(model) {
  const exact = requireString(model, 'model');
  const slash = exact.indexOf('/');
  return slash === -1 ? exact : exact.slice(slash + 1);
}

function providerBaseUrl(raw) {
  const url = new URL(requireString(raw, 'provider_base_url'));
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) {
    throw new Error('provider_base_url must be an HTTP(S) URL without embedded credentials');
  }
  if (url.search || url.hash) {
    throw new Error('provider_base_url must not contain a query or fragment');
  }
  if (url.pathname === '/' || url.pathname === '') {
    url.pathname = '/v1';
  }
  return url.toString().replace(/\/$/, '');
}

function mcpConfigArgs(rawServers) {
  if (rawServers == null) return { args: [], contract: [] };
  if (!Array.isArray(rawServers)) throw new Error('mcp_servers must be an array');
  const args = [];
  const contract = [];
  for (const raw of rawServers) {
    if (!raw || typeof raw !== 'object') throw new Error('MCP server contract must be an object');
    const name = requireString(raw.name, 'MCP server name');
    if (!/^[A-Za-z0-9_-]{1,64}$/.test(name)) throw new Error(`invalid MCP server name ${name}`);
    const prefix = `mcp_servers.${name}`;
    if (raw.transport === 'stdio') {
      const command = requireString(raw.command, `MCP ${name} command`);
      const serverArgs = Array.isArray(raw.args) && raw.args.every((value) => typeof value === 'string')
        ? raw.args
        : (() => { throw new Error(`MCP ${name} args must be strings`); })();
      const envVars = Array.isArray(raw.env_vars) && raw.env_vars.every((value) => /^[A-Za-z_][A-Za-z0-9_]*$/.test(value))
        ? raw.env_vars
        : (() => { throw new Error(`MCP ${name} env_vars are invalid`); })();
      for (const variable of envVars) {
        if (!(variable in process.env)) throw new Error(`MCP ${name} is missing scoped environment ${variable}`);
      }
      args.push('-c', `${prefix}.command=${JSON.stringify(command)}`);
      args.push('-c', `${prefix}.args=${JSON.stringify(serverArgs)}`);
      args.push('-c', `${prefix}.env_vars=${JSON.stringify(envVars)}`);
      contract.push({ name, transport: 'stdio', command, args: serverArgs, env_vars: envVars });
    } else if (raw.transport === 'http') {
      const url = new URL(requireString(raw.url, `MCP ${name} URL`));
      if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password) {
        throw new Error(`MCP ${name} URL must be credential-free HTTP(S)`);
      }
      const headers = raw.env_http_headers ?? {};
      if (!headers || typeof headers !== 'object' || Array.isArray(headers)) {
        throw new Error(`MCP ${name} env_http_headers must be an object`);
      }
      const entries = Object.entries(headers);
      for (const [header, variable] of entries) {
        if (!header || typeof variable !== 'string' || !/^[A-Za-z_][A-Za-z0-9_]*$/.test(variable)) {
          throw new Error(`MCP ${name} header environment mapping is invalid`);
        }
        if (!(variable in process.env)) throw new Error(`MCP ${name} is missing scoped header ${variable}`);
      }
      const inlineHeaders = `{${entries.map(([header, variable]) => `${JSON.stringify(header)} = ${JSON.stringify(variable)}`).join(', ')}}`;
      args.push('-c', `${prefix}.url=${JSON.stringify(url.toString())}`);
      if (entries.length > 0) args.push('-c', `${prefix}.env_http_headers=${inlineHeaders}`);
      contract.push({ name, transport: 'http', url: url.toString(), env_http_headers: headers });
    } else {
      throw new Error(`MCP ${name} has unsupported transport ${raw.transport}`);
    }
    args.push('-c', `${prefix}.required=true`);
  }
  return { args, contract };
}

function request(method, params) {
  if (!appInput || appInput.destroyed) {
    return Promise.reject(new Error('Codex app-server stdin is unavailable'));
  }
  const id = nextRpcId++;
  const promise = new Promise((resolve, reject) => pending.set(id, { resolve, reject, method }));
  appInput.write(`${JSON.stringify({ id, method, params })}\n`);
  return promise;
}

function notify(method, params = {}) {
  if (!appInput || appInput.destroyed) {
    throw new Error('Codex app-server stdin is unavailable');
  }
  appInput.write(`${JSON.stringify({ method, params })}\n`);
}

function rejectPending(message) {
  for (const { reject } of pending.values()) reject(new Error(message));
  pending.clear();
}

function compactItem(item) {
  if (!item || typeof item !== 'object') return null;
  return {
    id: item.id ?? null,
    type: item.type ?? null,
    status: item.status ?? null,
    command: item.command ?? null,
    path: item.path ?? null,
    tool: item.tool ?? item.toolName ?? null,
  };
}

function projectNotification(message) {
  const { method, params = {} } = message;
  switch (method) {
    case 'thread/started':
      emit({ type: 'thread_started', thread_id: params.thread?.id ?? null });
      return;
    case 'turn/started':
      activeTurnId = params.turn?.id ?? activeTurnId;
      emit({ type: 'turn_started', thread_id: params.threadId ?? threadId, turn_id: activeTurnId });
      if (pendingTurnOperation && activeTurnId) {
        emit({
          type: 'operation_complete',
          op: 'turn',
          request_id: pendingTurnOperation.requestId,
          thread_id: threadId,
          turn_id: activeTurnId,
        });
        pendingTurnOperation = null;
        turnStarting = false;
      }
      return;
    case 'item/agentMessage/delta':
      emit({
        type: 'agent_text_delta',
        thread_id: params.threadId ?? threadId,
        turn_id: params.turnId ?? activeTurnId,
        item_id: params.itemId ?? null,
        text: params.delta ?? '',
      });
      return;
    case 'item/reasoning/summaryTextDelta':
    case 'item/reasoning/summaryPartAdded':
    case 'item/reasoning/textDelta':
      // Hidden reasoning is neither requested nor persisted. Its occurrence is
      // only a liveness signal for event-driven supervision.
      emit({
        type: 'reasoning_activity',
        thread_id: params.threadId ?? threadId,
        turn_id: params.turnId ?? activeTurnId,
        item_id: params.itemId ?? null,
      });
      return;
    case 'item/started':
      emit({
        type: 'item_started',
        thread_id: params.threadId ?? threadId,
        turn_id: params.turnId ?? activeTurnId,
        item: compactItem(params.item),
        at_ms: params.startedAtMs ?? null,
      });
      return;
    case 'item/completed':
      emit({
        type: 'item_completed',
        thread_id: params.threadId ?? threadId,
        turn_id: params.turnId ?? activeTurnId,
        item: compactItem(params.item),
        at_ms: params.completedAtMs ?? null,
      });
      return;
    case 'item/commandExecution/outputDelta':
      // Output bytes may contain repository or secret material and remain in
      // Codex's own transcript. Their occurrence is enough for the controller
      // to observe tool liveness and coordinate cancellation event-first.
      emit({
        type: 'tool_activity',
        thread_id: params.threadId ?? threadId,
        turn_id: params.turnId ?? activeTurnId,
        item_id: params.itemId ?? null,
      });
      return;
    case 'thread/tokenUsage/updated':
      emit({
        type: 'usage',
        thread_id: params.threadId ?? threadId,
        turn_id: params.turnId ?? activeTurnId,
        token_usage: params.tokenUsage ?? null,
      });
      return;
    case 'model/rerouted':
      emit({
        type: 'model_rerouted',
        thread_id: params.threadId ?? threadId,
        turn_id: params.turnId ?? activeTurnId,
        details: params,
      });
      return;
    case 'turn/completed': {
      const turn = params.turn ?? {};
      emit({
        type: 'turn_completed',
        thread_id: params.threadId ?? threadId,
        turn_id: turn.id ?? activeTurnId,
        status: turn.status ?? 'unknown',
        error: turn.error ?? null,
        duration_ms: turn.durationMs ?? null,
      });
      if (!turn.id || turn.id === activeTurnId) activeTurnId = null;
      return;
    }
    case 'thread/compacted':
      emit({ type: 'thread_compacted', thread_id: params.threadId ?? threadId, details: params });
      return;
    case 'error':
    case 'warning':
    case 'configWarning':
      emit({ type: 'codex_notice', method, details: params });
      return;
    default:
      // Keep the transport observable without copying bulky raw response
      // items, command output, or model reasoning into a second transcript.
      if (method === 'remoteControl/status/changed' || method === 'thread/status/changed') {
        emit({ type: 'session_status', method, details: params });
      }
  }
}

function handleServerMessage(message) {
  const hasId = Object.hasOwn(message, 'id');
  const hasMethod = typeof message.method === 'string';
  if (hasId && !hasMethod) {
    const waiter = pending.get(message.id);
    if (!waiter) {
      emit({ type: 'orphan_response', id: message.id });
      return;
    }
    pending.delete(message.id);
    if (message.error) waiter.reject(new Error(`${waiter.method}: ${JSON.stringify(message.error)}`));
    else waiter.resolve(message.result);
    return;
  }
  if (hasId && hasMethod) {
    // Counted runs use approvalPolicy=never inside an already isolated Company
    // Runtime. An unexpected app-server callback must fail visibly, never hang
    // waiting for a human or silently grant broader authority.
    emit({ type: 'unexpected_server_request', id: message.id, method: message.method });
    appInput.write(`${JSON.stringify({
      id: message.id,
      error: { code: -32601, message: `runner does not service ${message.method}` },
    })}\n`);
    return;
  }
  if (hasMethod) projectNotification(message);
}

function attachAppServer(child) {
  appServer = child;
  appInput = child.stdin;
  createInterface({ input: child.stdout, crlfDelay: Infinity }).on('line', (line) => {
    if (!line.trim()) return;
    try {
      handleServerMessage(JSON.parse(line));
    } catch (error) {
      fail('unparseable Codex app-server output', { error: error.message, tail: line.slice(-1000) });
    }
  });
  child.stderr.setEncoding('utf8');
  child.stderr.on('data', (chunk) => {
    stderrTail = redactSensitive(`${stderrTail}${chunk}`).slice(-4000);
  });
  child.on('error', (error) => {
    rejectPending(`Codex app-server process error: ${error.message}`);
    fail('Codex app-server process error', { error: error.message });
  });
  child.on('exit', (code, signal) => {
    ready = false;
    rejectPending(`Codex app-server exited code=${code} signal=${signal}`);
    emit({ type: 'app_server_exited', code, signal, stderr_tail: stderrTail });
  });
}

async function launch(operation) {
  if (launched) throw new Error('launch may be sent only once');
  launched = true;
  const cwd = requireString(operation.cwd, 'cwd');
  const exactModel = requireString(operation.model, 'model');
  const model = providerModel(exactModel);
  const effort = requireString(operation.effort, 'effort');
  if (!ALLOWED_EFFORTS.has(effort)) throw new Error(`unsupported reasoning effort ${effort}`);
  const baseUrl = providerBaseUrl(operation.provider_base_url);
  if (!process.env[MODEL_CAPABILITY_ENV]) {
    throw new Error(`missing scoped ${MODEL_CAPABILITY_ENV}`);
  }
  const codexHome = requireString(process.env.CODEX_HOME, 'CODEX_HOME');
  const mcp = mcpConfigArgs(operation.mcp_servers);
  const disabledFeatureArgs = DISABLED_CODEX_FEATURES.flatMap((feature) => ['--disable', feature]);
  const args = [
    'app-server', '--stdio', '--strict-config', ...disabledFeatureArgs,
    '-c', 'model_provider="restless"',
    '-c', 'model_providers.restless.name="Restless scoped relay"',
    '-c', `model_providers.restless.base_url=${JSON.stringify(baseUrl)}`,
    '-c', `model_providers.restless.env_key=${JSON.stringify(MODEL_CAPABILITY_ENV)}`,
    '-c', 'model_providers.restless.wire_api="responses"',
    '-c', 'model_providers.restless.request_max_retries=0',
    '-c', 'model_providers.restless.stream_max_retries=0',
    '-c', 'model_providers.restless.stream_idle_timeout_ms=900000',
    '-c', `model_reasoning_effort=${JSON.stringify(effort)}`,
    ...mcp.args,
  ];
  const appServerEnv = {
    ...process.env,
    HTTP_PROXY: DENIED_TASK_PROXY,
    HTTPS_PROXY: DENIED_TASK_PROXY,
    ALL_PROXY: DENIED_TASK_PROXY,
    http_proxy: DENIED_TASK_PROXY,
    https_proxy: DENIED_TASK_PROXY,
    all_proxy: DENIED_TASK_PROXY,
    NO_PROXY: MODEL_RELAY_NO_PROXY,
    no_proxy: MODEL_RELAY_NO_PROXY,
  };
  attachAppServer(spawn(operation.codex_bin || 'codex', args, {
    cwd,
    env: appServerEnv,
    stdio: ['pipe', 'pipe', 'pipe'],
  }));
  const initialized = await request('initialize', { clientInfo: CLIENT, capabilities: null });
  notify('initialized');
  const common = {
    cwd,
    model,
    modelProvider: 'restless',
    allowProviderModelFallback: false,
    approvalPolicy: 'never',
    sandbox: 'danger-full-access',
    developerInstructions: requireString(operation.developer_instructions, 'developer_instructions'),
    ephemeral: false,
  };
  const prior = typeof operation.thread_id === 'string' && operation.thread_id ? operation.thread_id : null;
  const result = prior
    ? await request('thread/resume', { threadId: prior, ...common })
    : await request('thread/start', common);
  threadId = result.thread?.id;
  if (!threadId) throw new Error('Codex did not return a thread id');
  observed = {
    codex_version: initialized.userAgent ?? null,
    codex_home: initialized.codexHome ?? codexHome,
    protocol_version: PROTOCOL_VERSION,
    model_requested: exactModel,
    model_observed: result.model ?? null,
    provider_observed: result.modelProvider ?? null,
    effort_requested: effort,
    effort_observed: result.reasoningEffort ?? null,
    cwd_observed: result.cwd ?? null,
    approval_policy_observed: result.approvalPolicy ?? null,
    sandbox_observed: result.sandboxPolicy ?? result.sandbox ?? null,
    network_policy_observed: 'host-model-relay-only-v1',
    disabled_features_observed: DISABLED_CODEX_FEATURES,
    mcp_contract_digest: createHash('sha256').update(JSON.stringify(mcp.contract)).digest('hex'),
    runner_digest: createHash('sha256').update(readFileSync(new URL(import.meta.url))).digest('hex'),
  };
  if (observed.model_observed !== model || observed.provider_observed !== 'restless') {
    throw new Error(`exact model/provider admission failed: ${JSON.stringify(observed)}`);
  }
  if (observed.effort_observed !== effort) {
    throw new Error(`exact effort admission failed: ${JSON.stringify(observed)}`);
  }
  ready = true;
  emit({ type: 'session_ready', thread_id: threadId, resumed: Boolean(prior), observed });
}

function textInput(text) {
  return [{ type: 'text', text: requireString(text, 'text'), text_elements: [] }];
}

async function dispatch(operation) {
  if (!operation || typeof operation !== 'object' || !ALLOWED_OPS.has(operation.op)) {
    throw new Error('unknown runner operation');
  }
  if (operation.op === 'launch') return launch(operation);
  if (!ready) throw new Error('runner is not ready');
  switch (operation.op) {
    case 'turn': {
      if (activeTurnId || turnStarting) {
        throw new Error(activeTurnId ? `turn ${activeTurnId} is already active` : 'a turn is starting');
      }
      turnStarting = true;
      pendingTurnOperation = { requestId: operation.request_id ?? null };
      request('turn/start', {
        threadId,
        input: textInput(operation.text),
        model: providerModel(observed.model_requested),
        effort: observed.effort_requested,
        approvalPolicy: 'never',
      }).then((result) => {
        const returnedTurnId = result.turn?.id ?? null;
        if (pendingTurnOperation) {
          if (!returnedTurnId) throw new Error('Codex did not return a turn id');
          activeTurnId = returnedTurnId;
          emit({
            type: 'operation_complete',
            op: 'turn',
            request_id: pendingTurnOperation.requestId,
            thread_id: threadId,
            turn_id: activeTurnId,
          });
          pendingTurnOperation = null;
          turnStarting = false;
        } else if (returnedTurnId && activeTurnId && returnedTurnId !== activeTurnId) {
          throw new Error(`Codex returned turn ${returnedTurnId} after starting ${activeTurnId}`);
        }
      }).catch((error) => {
        pendingTurnOperation = null;
        turnStarting = false;
        fail(error.message, { op: 'turn', request_id: operation.request_id ?? null });
      });
      return;
    }
    case 'steer': {
      if (!activeTurnId) throw new Error('no active turn to steer');
      const result = await request('turn/steer', { threadId, input: textInput(operation.text) });
      emit({ type: 'operation_complete', op: 'steer', request_id: operation.request_id ?? null, thread_id: threadId, turn_id: result.turnId ?? activeTurnId });
      return;
    }
    case 'interrupt': {
      if (!activeTurnId) throw new Error('no active turn to interrupt');
      const interrupted = activeTurnId;
      await request('turn/interrupt', { threadId, turnId: interrupted });
      emit({ type: 'operation_complete', op: 'interrupt', request_id: operation.request_id ?? null, thread_id: threadId, turn_id: interrupted });
      return;
    }
    case 'ping':
      emit({ type: 'pong', request_id: operation.request_id ?? null, thread_id: threadId, turn_id: activeTurnId, ready });
      return;
    case 'shutdown':
      emit({ type: 'operation_complete', op: 'shutdown', request_id: operation.request_id ?? null, thread_id: threadId });
      appInput.end();
      return;
  }
}

const input = createInterface({ input: process.stdin, crlfDelay: Infinity });
let chain = Promise.resolve();
input.on('line', (line) => {
  if (!line.trim()) return;
  chain = chain.then(async () => {
    let operation = null;
    try {
      operation = JSON.parse(line);
      const requestId = typeof operation.request_id === 'string' && operation.request_id
        ? operation.request_id
        : null;
      if (requestId && completedOperations.has(requestId)) {
        emit({
          type: 'operation_duplicate',
          op: operation.op,
          request_id: requestId,
          original_op: completedOperations.get(requestId),
          thread_id: threadId,
          turn_id: activeTurnId,
        });
        return;
      }
      await dispatch(operation);
      if (requestId) completedOperations.set(requestId, operation.op);
    } catch (error) {
      fail(error.message, { op: operation?.op ?? null });
    }
  });
});
input.on('close', () => {
  chain.finally(() => {
    if (appInput && !appInput.destroyed) appInput.end();
  });
});

emit({ type: 'runner_started', protocol_version: PROTOCOL_VERSION });
