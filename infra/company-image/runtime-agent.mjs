#!/usr/bin/env node

import { constants as fsConstants } from 'node:fs';
import { spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import {
  access, lstat, mkdir, open, readFile, realpath, rename, rm, writeFile,
} from 'node:fs/promises';
import { createConnection } from 'node:net';
import {
  basename, dirname, isAbsolute, join, relative, resolve, sep,
} from 'node:path';
import { pathToFileURL } from 'node:url';

export const PROTOCOL_VERSION = 1;
export const REGISTRATION_FEATURES = Object.freeze([
  'registration.v1',
  'capability-rotation.v1',
  'activity.v1',
  'desktop.v1',
  'files.v1',
  'process.v1',
  'streams.v1',
]);
const MAX_COMMAND_BYTES = 8 * 1024 * 1024;
const MAX_FILE_BYTES = 5 * 1024 * 1024;
const MAX_PROCESS_OUTPUT_BYTES = 1024 * 1024;
const MAX_REPLAY_ENTRIES = 128;
const STREAM_CHUNK_BYTES = 48 * 1024;
const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[1-8][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const LOWER_GIT_REVISION = /^[0-9a-f]{40}$/;
const IMMUTABLE_IMAGE = /^[^\s@]+@sha256:[0-9a-f]{64}$/;
const BOUNDED_IDENTITY = /^[A-Za-z0-9_.:-]+$/;

function required(environment, name) {
  const value = environment[name]?.trim();
  if (!value) throw new Error(`Runtime Agent requires ${name}`);
  return value;
}

function identity(environment, name, max = 160) {
  const value = required(environment, name);
  if (value.length > max || !BOUNDED_IDENTITY.test(value)) {
    throw new Error(`${name} is not a bounded identity`);
  }
  return value;
}

function uuid(environment, name) {
  const value = required(environment, name).toLowerCase();
  if (!UUID.test(value) || value === '00000000-0000-0000-0000-000000000000') {
    throw new Error(`${name} must be a non-nil canonical UUID`);
  }
  return value;
}

function positiveInteger(environment, name) {
  const raw = required(environment, name);
  if (!/^[1-9][0-9]*$/.test(raw)) throw new Error(`${name} must be a positive integer`);
  const value = Number(raw);
  if (!Number.isSafeInteger(value)) throw new Error(`${name} exceeds the safe integer range`);
  return value;
}

export function parseConfiguration(environment = process.env) {
  const bridgeUrl = new URL(required(environment, 'RESTLESS_RUNTIME_BRIDGE_URL'));
  const loopbackTest = bridgeUrl.protocol === 'ws:'
    && ['127.0.0.1', '::1', 'localhost'].includes(bridgeUrl.hostname);
  if ((!loopbackTest && bridgeUrl.protocol !== 'wss:')
      || bridgeUrl.username || bridgeUrl.password || bridgeUrl.search || bridgeUrl.hash
      || bridgeUrl.pathname !== '/internal/v1/runtime-bridge') {
    throw new Error('RESTLESS_RUNTIME_BRIDGE_URL must be the exact wss Runtime Bridge endpoint');
  }
  const runtimeImage = required(environment, 'RESTLESS_RUNTIME_IMAGE');
  if (!IMMUTABLE_IMAGE.test(runtimeImage)) {
    throw new Error('RESTLESS_RUNTIME_IMAGE must be an immutable registry sha256 reference');
  }
  const sourceRevision = required(environment, 'RESTLESS_SOURCE_REVISION');
  if (!LOWER_GIT_REVISION.test(sourceRevision)) {
    throw new Error('RESTLESS_SOURCE_REVISION must be an exact lowercase Git revision');
  }
  return Object.freeze({
    bridgeUrl: bridgeUrl.href,
    ownerId: uuid(environment, 'RESTLESS_RUNTIME_OWNER_ID'),
    planeId: uuid(environment, 'RESTLESS_RUNTIME_PLANE_ID'),
    companyId: uuid(environment, 'RESTLESS_RUNTIME_COMPANY_ID'),
    cellId: uuid(environment, 'RESTLESS_RUNTIME_CELL_ID'),
    company: identity(environment, 'RESTLESS_COMPANY', 96),
    runtimeId: identity(environment, 'RESTLESS_RUNTIME_ID'),
    runtimeGeneration: positiveInteger(environment, 'RESTLESS_RUNTIME_GENERATION'),
    desiredRevision: positiveInteger(environment, 'RESTLESS_RUNTIME_DESIRED_REVISION'),
    runtimeImage,
    volumeName: identity(environment, 'RESTLESS_RUNTIME_VOLUME_NAME'),
    sourceRevision,
    capabilityFile: required(environment, 'RESTLESS_RUNTIME_BRIDGE_CAPABILITY_FILE'),
    capabilityStateFile: required(
      environment,
      'RESTLESS_RUNTIME_BRIDGE_CAPABILITY_STATE_FILE',
    ),
  });
}

function boundedCapability(bytes) {
  if (bytes.length > 16 * 1024) throw new Error('Runtime Bridge capability file is too large');
  const capability = bytes.toString('utf8').trim();
  if (!capability || capability.includes('\n') || capability.includes('\r')) {
    throw new Error('Runtime Bridge capability file must contain one bounded token');
  }
  return capability;
}

export async function persistCapability(path, capability) {
  const bytes = Buffer.from(capability, 'utf8');
  boundedCapability(bytes);
  const parent = dirname(path);
  await mkdir(parent, { recursive: true, mode: 0o700 });
  const metadata = await lstat(parent);
  if (metadata.isSymbolicLink() || !metadata.isDirectory()) {
    throw new Error('Runtime Bridge capability state parent must be a real directory');
  }
  const temporary = join(parent, `.runtime-bridge-capability-${randomUUID()}.tmp`);
  let handle;
  try {
    handle = await open(temporary, 'wx', 0o600);
    await handle.writeFile(`${capability}\n`, { encoding: 'utf8' });
    await handle.sync();
    await handle.close();
    handle = undefined;
    await rename(temporary, path);
  } finally {
    if (handle) await handle.close().catch(() => {});
    await rm(temporary, { force: true }).catch(() => {});
  }
}

export async function buildRegistration(config) {
  let capabilityBytes;
  try {
    capabilityBytes = await readFile(config.capabilityStateFile);
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
    capabilityBytes = await readFile(config.capabilityFile);
  }
  const capability = boundedCapability(capabilityBytes);
  let persistentVolumeReady = true;
  try {
    await access('/company/.seeded', fsConstants.R_OK);
    await access('/company', fsConstants.R_OK | fsConstants.W_OK);
  } catch {
    persistentVolumeReady = false;
  }
  return {
    protocol_version: PROTOCOL_VERSION,
    owner_id: config.ownerId,
    plane_id: config.planeId,
    company_id: config.companyId,
    cell_id: config.cellId,
    company: config.company,
    runtime_id: config.runtimeId,
    runtime_generation: config.runtimeGeneration,
    desired_revision: config.desiredRevision,
    runtime_image: config.runtimeImage,
    volume_name: config.volumeName,
    persistent_volume_ready: persistentVolumeReady,
    source_revision: config.sourceRevision,
    supported_features: [...REGISTRATION_FEATURES],
    capability,
  };
}

function waitForOpen(socket, signal) {
  return new Promise((resolve, reject) => {
    const clean = () => {
      socket.removeEventListener('open', opened);
      socket.removeEventListener('error', failed);
      signal.removeEventListener('abort', aborted);
    };
    const opened = () => { clean(); resolve(); };
    const failed = () => { clean(); reject(new Error('Runtime Bridge connection failed')); };
    const aborted = () => { clean(); socket.close(); reject(signal.reason); };
    socket.addEventListener('open', opened, { once: true });
    socket.addEventListener('error', failed, { once: true });
    signal.addEventListener('abort', aborted, { once: true });
  });
}

function waitForRegistration(socket, registration, signal) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => finish(new Error('Runtime Bridge registration timed out')), 5_000);
    const clean = () => {
      clearTimeout(timer);
      socket.removeEventListener('message', message);
      socket.removeEventListener('close', closed);
      socket.removeEventListener('error', failed);
      signal.removeEventListener('abort', aborted);
    };
    const finish = (error, value) => { clean(); error ? reject(error) : resolve(value); };
    const message = (event) => {
      try {
        if (typeof event.data !== 'string') throw new Error('Runtime Bridge acknowledgement is not text');
        const value = JSON.parse(event.data);
        if (value.protocol_version !== PROTOCOL_VERSION || value.status !== 'registered'
            || value.company_id !== registration.company_id || value.cell_id !== registration.cell_id
            || value.runtime_id !== registration.runtime_id
            || value.runtime_generation !== registration.runtime_generation) {
          throw new Error('Runtime Bridge acknowledgement identity mismatch');
        }
        finish(null, value);
      } catch (error) {
        finish(error);
      }
    };
    const closed = () => finish(new Error('Runtime Bridge closed during registration'));
    const failed = () => finish(new Error('Runtime Bridge failed during registration'));
    const aborted = () => { socket.close(); finish(signal.reason); };
    socket.addEventListener('message', message);
    socket.addEventListener('close', closed, { once: true });
    socket.addEventListener('error', failed, { once: true });
    signal.addEventListener('abort', aborted, { once: true });
    socket.send(JSON.stringify(registration));
  });
}

function assertExactKeys(value, expected) {
  const keys = Object.keys(value).sort();
  if (JSON.stringify(keys) !== JSON.stringify([...expected].sort())) {
    throw new Error('Runtime Agent command shape is invalid');
  }
}

function validateCommon(command, config) {
  if (!command || typeof command !== 'object' || Array.isArray(command)
      || command.protocol_version !== PROTOCOL_VERSION
      || !UUID.test(command.operation_id)
      || command.runtime_id !== config.runtimeId
      || command.runtime_generation !== config.runtimeGeneration) {
    throw new Error('Runtime Agent command identity is invalid');
  }
}

async function confinedPath(companyRoot, requested, mustExist = true) {
  if (typeof requested !== 'string' || requested.length > 4096
      || (requested !== '/company' && !requested.startsWith('/company/'))) {
    throw new Error('Runtime Agent path must be beneath /company');
  }
  const root = await realpath(companyRoot);
  const suffix = requested === '/company' ? '.' : `.${requested.slice('/company'.length)}`;
  const candidate = resolve(root, suffix);
  const resolved = mustExist ? await realpath(candidate) : candidate;
  const escape = relative(root, resolved);
  if (escape === '..' || escape.startsWith(`..${sep}`) || isAbsolute(escape)) {
    throw new Error('Runtime Agent path escapes the company volume');
  }
  return resolved;
}

async function writableCompanyPath(companyRoot, requested) {
  if (typeof requested !== 'string' || requested.length > 4096
      || (requested !== '/company' && !requested.startsWith('/company/'))) {
    throw new Error('Runtime Agent path must be beneath /company');
  }
  const root = await realpath(companyRoot);
  const parts = requested.slice('/company/'.length).split('/');
  if (requested === '/company' || parts.some((part) => !part || part === '.' || part === '..')) {
    throw new Error('Runtime Agent write path is invalid');
  }
  let parent = root;
  for (const part of parts.slice(0, -1)) {
    const next = join(parent, part);
    try {
      const metadata = await lstat(next);
      if (metadata.isSymbolicLink() || !metadata.isDirectory()) {
        throw new Error('Runtime Agent write parent is not a real directory');
      }
    } catch (error) {
      if (error?.code !== 'ENOENT') throw error;
      await mkdir(next, { mode: 0o700 });
    }
    parent = await realpath(next);
    const escape = relative(root, parent);
    if (escape === '..' || escape.startsWith(`..${sep}`) || isAbsolute(escape)) {
      throw new Error('Runtime Agent write path escapes the company volume');
    }
  }
  const target = join(parent, parts.at(-1));
  try {
    const metadata = await lstat(target);
    if (metadata.isSymbolicLink() || !metadata.isFile()) {
      throw new Error('Runtime Agent write target is not a regular file');
    }
  } catch (error) {
    if (error?.code !== 'ENOENT') throw error;
  }
  return target;
}

function boundedBase64(value, maximum) {
  if (typeof value !== 'string'
      || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)) {
    throw new Error('Runtime Agent bytes are not canonical base64');
  }
  const bytes = Buffer.from(value, 'base64');
  if (bytes.length > maximum || bytes.toString('base64') !== value) {
    throw new Error('Runtime Agent bytes exceed their bound');
  }
  return bytes;
}

async function readCompanyFile(command, config, companyRoot) {
  assertExactKeys(command, [
    'type', 'protocol_version', 'operation_id', 'runtime_id', 'runtime_generation',
    'path', 'max_bytes',
  ]);
  if (!Number.isSafeInteger(command.max_bytes) || command.max_bytes < 1
      || command.max_bytes > MAX_FILE_BYTES) {
    throw new Error('Runtime Agent file bound is invalid');
  }
  const path = await confinedPath(companyRoot, command.path);
  const bytes = await readFile(path);
  if (bytes.length > command.max_bytes) throw new Error('Runtime Agent file exceeds requested bound');
  return {
    type: 'file.result',
    operation_id: command.operation_id,
    runtime_id: config.runtimeId,
    runtime_generation: config.runtimeGeneration,
    path: command.path,
    bytes_base64: bytes.toString('base64'),
    sha256: createHash('sha256').update(bytes).digest('hex'),
  };
}

async function readCompanyFileChunk(command, config, companyRoot) {
  assertExactKeys(command, [
    'type', 'protocol_version', 'operation_id', 'runtime_id', 'runtime_generation',
    'path', 'offset', 'max_bytes',
  ]);
  if (!Number.isSafeInteger(command.offset) || command.offset < 0
      || !Number.isSafeInteger(command.max_bytes) || command.max_bytes < 1
      || command.max_bytes > 1024 * 1024) {
    throw new Error('Runtime Agent file chunk bound is invalid');
  }
  const path = await confinedPath(companyRoot, command.path);
  const handle = await open(path, 'r');
  try {
    const metadata = await handle.stat();
    if (!metadata.isFile() || command.offset > metadata.size) {
      throw new Error('Runtime Agent file chunk offset is invalid');
    }
    const bytes = Buffer.alloc(Math.min(command.max_bytes, metadata.size - command.offset));
    const { bytesRead } = await handle.read(bytes, 0, bytes.length, command.offset);
    const chunk = bytes.subarray(0, bytesRead);
    return {
      type: 'file.chunk',
      operation_id: command.operation_id,
      runtime_id: config.runtimeId,
      runtime_generation: config.runtimeGeneration,
      path: command.path,
      offset: command.offset,
      size_bytes: metadata.size,
      eof: command.offset + bytesRead === metadata.size,
      bytes_base64: chunk.toString('base64'),
      sha256: createHash('sha256').update(chunk).digest('hex'),
    };
  } finally {
    await handle.close();
  }
}

async function writeCompanyFile(command, config, companyRoot) {
  assertExactKeys(command, [
    'type', 'protocol_version', 'operation_id', 'runtime_id', 'runtime_generation',
    'path', 'bytes_base64',
  ]);
  const bytes = boundedBase64(command.bytes_base64, MAX_FILE_BYTES);
  const path = await writableCompanyPath(companyRoot, command.path);
  const temporary = join(dirname(path), `.${basename(path)}.${randomUUID()}.tmp`);
  try {
    await writeFile(temporary, bytes, { flag: 'wx', mode: 0o600 });
    await rename(temporary, path);
  } finally {
    await rm(temporary, { force: true }).catch(() => {});
  }
  return {
    type: 'file.written',
    operation_id: command.operation_id,
    runtime_id: config.runtimeId,
    runtime_generation: config.runtimeGeneration,
    path: command.path,
    size_bytes: bytes.length,
    sha256: createHash('sha256').update(bytes).digest('hex'),
  };
}

async function removeCompanyFile(command, config, companyRoot) {
  assertExactKeys(command, [
    'type', 'protocol_version', 'operation_id', 'runtime_id', 'runtime_generation', 'path',
  ]);
  const path = await confinedPath(companyRoot, command.path);
  const metadata = await lstat(path);
  if (metadata.isSymbolicLink() || !metadata.isFile()) {
    throw new Error('Runtime Agent removal target is not a regular file');
  }
  await rm(path);
  return {
    type: 'file.removed',
    operation_id: command.operation_id,
    runtime_id: config.runtimeId,
    runtime_generation: config.runtimeGeneration,
    path: command.path,
  };
}

async function runCompanyProcess(command, config, companyRoot) {
  assertExactKeys(command, [
    'type', 'protocol_version', 'operation_id', 'runtime_id', 'runtime_generation',
    'program', 'args', 'cwd', 'environment', 'timeout_ms', 'max_output_bytes',
  ]);
  if (typeof command.program !== 'string' || !isAbsolute(command.program)
      || !Array.isArray(command.args) || command.args.length > 128
      || command.args.some((value) => typeof value !== 'string' || value.length > 4096)
      || !command.environment || typeof command.environment !== 'object'
      || Array.isArray(command.environment) || Object.keys(command.environment).length > 128
      || Object.entries(command.environment).some(([key, value]) => !/^[A-Za-z_][A-Za-z0-9_]*$/.test(key)
        || typeof value !== 'string' || value.length > 16 * 1024)
      || !Number.isSafeInteger(command.timeout_ms) || command.timeout_ms < 1
      || command.timeout_ms > 300_000
      || !Number.isSafeInteger(command.max_output_bytes) || command.max_output_bytes < 1
      || command.max_output_bytes > MAX_PROCESS_OUTPUT_BYTES) {
    throw new Error('Runtime Agent process bounds are invalid');
  }
  const cwd = await confinedPath(companyRoot, command.cwd);
  return new Promise((resolvePromise, reject) => {
    let timer;
    const child = spawn(command.program, command.args, {
      cwd,
      env: { ...process.env, ...command.environment },
      shell: false,
      stdio: ['ignore', 'pipe', 'pipe'],
    });
    const stdout = [];
    const stderr = [];
    let outputBytes = 0;
    let timedOut = false;
    let settled = false;
    const finish = (error, code = null, signal = null) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      if (error) return reject(error);
      resolvePromise({
        type: 'process.result',
        operation_id: command.operation_id,
        runtime_id: config.runtimeId,
        runtime_generation: config.runtimeGeneration,
        exit_code: code,
        signal,
        timed_out: timedOut,
        stdout_base64: Buffer.concat(stdout).toString('base64'),
        stderr_base64: Buffer.concat(stderr).toString('base64'),
      });
    };
    const consume = (target) => (chunk) => {
      outputBytes += chunk.length;
      if (outputBytes > command.max_output_bytes) {
        child.kill('SIGKILL');
        finish(new Error('Runtime Agent process output exceeds requested bound'));
      } else {
        target.push(chunk);
      }
    };
    child.stdout.on('data', consume(stdout));
    child.stderr.on('data', consume(stderr));
    child.once('error', (error) => finish(error));
    child.once('close', (code, signal) => finish(null, code, signal));
    timer = setTimeout(() => {
      timedOut = true;
      child.kill('SIGKILL');
    }, command.timeout_ms);
  });
}

export function createCommandHandler(config, options = {}) {
  const companyRoot = options.companyRoot ?? '/company';
  const now = options.now ?? (() => new Date());
  const sendEvent = options.sendEvent ?? (() => {});
  const connectTcp = options.connectTcp ?? ((port, host) => createConnection({ port, host }));
  const saveCapability = options.persistCapability
    ?? ((capability) => persistCapability(config.capabilityStateFile, capability));
  const replay = new Map();
  const streams = new Map();
  const streamEvent = (type, operationId, fields = {}) => sendEvent({
    type,
    protocol_version: PROTOCOL_VERSION,
    operation_id: operationId,
    runtime_id: config.runtimeId,
    runtime_generation: config.runtimeGeneration,
    ...fields,
  });
  return async function handle(raw) {
    if (typeof raw !== 'string' || Buffer.byteLength(raw) > MAX_COMMAND_BYTES) {
      throw new Error('Runtime Agent command must be one bounded text frame');
    }
    const digest = createHash('sha256').update(raw).digest('hex');
    const command = JSON.parse(raw);
    validateCommon(command, config);
    if (command.type === 'stream.data') {
      assertExactKeys(command, [
        'type', 'protocol_version', 'operation_id', 'runtime_id', 'runtime_generation',
        'bytes_base64',
      ]);
      const stream = streams.get(command.operation_id);
      if (!stream || typeof command.bytes_base64 !== 'string'
          || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(command.bytes_base64)) {
        throw new Error('Runtime Agent stream data is invalid');
      }
      const bytes = Buffer.from(command.bytes_base64, 'base64');
      if (bytes.length < 1 || bytes.length > STREAM_CHUNK_BYTES
          || bytes.toString('base64') !== command.bytes_base64) {
        throw new Error('Runtime Agent stream data exceeds its bound');
      }
      stream.write(bytes);
      return null;
    }
    if (command.type === 'stream.close') {
      assertExactKeys(command, [
        'type', 'protocol_version', 'operation_id', 'runtime_id', 'runtime_generation',
      ]);
      const stream = streams.get(command.operation_id);
      if (!stream) throw new Error('Runtime Agent stream is not active');
      streams.delete(command.operation_id);
      stream.end();
      return null;
    }
    const previous = replay.get(command.operation_id);
    if (previous) {
      if (previous.digest !== digest) throw new Error('Runtime Agent operation replay changed shape');
      return previous.response;
    }
    let response;
    if (command.type === 'capability.rotate') {
      assertExactKeys(command, [
        'type', 'protocol_version', 'operation_id', 'runtime_id', 'runtime_generation',
        'capability', 'valid_for_seconds',
      ]);
      if (!Number.isSafeInteger(command.valid_for_seconds)
          || command.valid_for_seconds < 3600 || command.valid_for_seconds > 86400) {
        throw new Error('Runtime Bridge capability lifetime is invalid');
      }
      boundedCapability(Buffer.from(command.capability ?? '', 'utf8'));
      await saveCapability(command.capability);
      response = {
        type: 'capability.rotated',
        operation_id: command.operation_id,
        runtime_id: config.runtimeId,
        runtime_generation: config.runtimeGeneration,
        valid_for_seconds: command.valid_for_seconds,
      };
    } else if (command.type === 'activity.observe') {
      assertExactKeys(command, [
        'type', 'protocol_version', 'operation_id', 'runtime_id', 'runtime_generation',
      ]);
      response = {
        type: 'activity.result',
        operation_id: command.operation_id,
        runtime_id: config.runtimeId,
        runtime_generation: config.runtimeGeneration,
        active_processes: [],
        observed_at: now().toISOString(),
      };
    } else if (command.type === 'file.read') {
      response = await readCompanyFile(command, config, companyRoot);
    } else if (command.type === 'file.read_chunk') {
      response = await readCompanyFileChunk(command, config, companyRoot);
    } else if (command.type === 'file.write') {
      response = await writeCompanyFile(command, config, companyRoot);
    } else if (command.type === 'file.remove') {
      response = await removeCompanyFile(command, config, companyRoot);
    } else if (command.type === 'process.run') {
      response = await runCompanyProcess(command, config, companyRoot);
    } else if (command.type === 'desktop.probe') {
      assertExactKeys(command, [
        'type', 'protocol_version', 'operation_id', 'runtime_id', 'runtime_generation',
      ]);
      const stream = connectTcp(6080, '127.0.0.1');
      await new Promise((resolvePromise, reject) => {
        const opened = () => { clean(); stream.destroy(); resolvePromise(); };
        const failed = () => { clean(); reject(new Error('Runtime Agent desktop is unavailable')); };
        const clean = () => {
          stream.removeListener('connect', opened);
          stream.removeListener('error', failed);
        };
        stream.once('connect', opened);
        stream.once('error', failed);
      });
      response = {
        type: 'desktop.result',
        operation_id: command.operation_id,
        runtime_id: config.runtimeId,
        runtime_generation: config.runtimeGeneration,
        status: 'available',
        host: '127.0.0.1',
        port: 6080,
      };
    } else if (command.type === 'stream.open') {
      assertExactKeys(command, [
        'type', 'protocol_version', 'operation_id', 'runtime_id', 'runtime_generation',
        'host', 'port',
      ]);
      if (command.host !== '127.0.0.1' || !Number.isSafeInteger(command.port)
          || command.port < 1 || command.port > 65535) {
        throw new Error('Runtime Agent stream target must be bounded loopback TCP');
      }
      const stream = connectTcp(command.port, command.host);
      await new Promise((resolvePromise, reject) => {
        const opened = () => { clean(); resolvePromise(); };
        const failed = () => { clean(); reject(new Error('Runtime Agent TCP stream failed to open')); };
        const clean = () => {
          stream.removeListener('connect', opened);
          stream.removeListener('error', failed);
        };
        stream.once('connect', opened);
        stream.once('error', failed);
      });
      stream.pause();
      streams.set(command.operation_id, stream);
      stream.on('data', (bytes) => {
        for (let offset = 0; offset < bytes.length; offset += STREAM_CHUNK_BYTES) {
          streamEvent('stream.data', command.operation_id, {
            bytes_base64: bytes.subarray(offset, offset + STREAM_CHUNK_BYTES).toString('base64'),
          });
        }
      });
      stream.once('end', () => {
        streams.delete(command.operation_id);
        streamEvent('stream.end', command.operation_id);
      });
      stream.once('error', () => {
        streams.delete(command.operation_id);
        streamEvent('stream.error', command.operation_id, { code: 'runtime_tcp_error' });
      });
      setImmediate(() => stream.resume());
      response = {
        type: 'stream.opened',
        operation_id: command.operation_id,
        runtime_id: config.runtimeId,
        runtime_generation: config.runtimeGeneration,
        host: command.host,
        port: command.port,
      };
    } else {
      throw new Error('Runtime Agent command type is not implemented');
    }
    replay.set(command.operation_id, { digest, response });
    if (replay.size > MAX_REPLAY_ENTRIES) replay.delete(replay.keys().next().value);
    return response;
  };
}

export async function handleCommand(raw, config, observedAt = new Date(), options = {}) {
  return createCommandHandler(config, { ...options, now: () => observedAt })(raw);
}

/* legacy function boundary retained for the exported test surface */
function validateBoundedCommand(raw) {
  if (typeof raw !== 'string' || Buffer.byteLength(raw) > MAX_COMMAND_BYTES) {
    throw new Error('Runtime Agent command must be one bounded text frame');
  }
}

function serveCommands(socket, config, signal) {
  const handle = createCommandHandler(config);
  return new Promise((resolve) => {
    const clean = () => {
      socket.removeEventListener('message', message);
      socket.removeEventListener('close', finish);
      signal.removeEventListener('abort', aborted);
    };
    const finish = () => { clean(); resolve(); };
    const aborted = () => socket.close(1000, 'shutdown');
    const message = async (event) => {
      try {
        validateBoundedCommand(event.data);
        const response = await handle(event.data);
        if (response) socket.send(JSON.stringify(response));
      } catch {
        socket.close(1008, 'invalid command');
      }
    };
    socket.addEventListener('message', message);
    socket.addEventListener('close', finish, { once: true });
    signal.addEventListener('abort', aborted, { once: true });
  });
}

export async function connectOnce(config, signal, WebSocketImpl = WebSocket) {
  const registration = await buildRegistration(config);
  const socket = new WebSocketImpl(config.bridgeUrl);
  await waitForOpen(socket, signal);
  await waitForRegistration(socket, registration, signal);
  process.stderr.write('Runtime Agent registered with its account plane\n');
  await serveCommands(socket, config, signal);
}

export async function run(environment = process.env, WebSocketImpl = WebSocket) {
  const config = parseConfiguration(environment);
  const shutdown = new AbortController();
  process.once('SIGTERM', () => shutdown.abort(new Error('shutdown')));
  process.once('SIGINT', () => shutdown.abort(new Error('shutdown')));
  let delay = 250;
  while (!shutdown.signal.aborted) {
    try {
      await connectOnce(config, shutdown.signal, WebSocketImpl);
      delay = 250;
    } catch {
      if (shutdown.signal.aborted) break;
      process.stderr.write('Runtime Agent connection unavailable; retrying\n');
    }
    await new Promise((resolve) => setTimeout(resolve, delay));
    delay = Math.min(delay * 2, 15_000);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  run().catch(() => {
    process.stderr.write('Runtime Agent configuration is invalid\n');
    process.exitCode = 1;
  });
}
