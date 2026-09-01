#!/usr/bin/env node

import { constants as fsConstants } from 'node:fs';
import { access, readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';

export const PROTOCOL_VERSION = 1;
export const REGISTRATION_FEATURES = Object.freeze(['registration.v1']);
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
  });
}

export async function buildRegistration(config) {
  const capabilityBytes = await readFile(config.capabilityFile);
  if (capabilityBytes.length > 16 * 1024) throw new Error('Runtime Bridge capability file is too large');
  const capability = capabilityBytes.toString('utf8').trim();
  if (!capability || capability.includes('\n') || capability.includes('\r')) {
    throw new Error('Runtime Bridge capability file must contain one bounded token');
  }
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

function waitForClose(socket, signal) {
  return new Promise((resolve) => {
    const finish = () => {
      socket.removeEventListener('close', finish);
      signal.removeEventListener('abort', aborted);
      resolve();
    };
    const aborted = () => socket.close(1000, 'shutdown');
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
  await waitForClose(socket, signal);
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
