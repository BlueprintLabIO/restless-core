import { createServer } from 'node:http';

const integer = (name) => {
  const value = Number.parseInt(process.env[name] ?? '', 10);
  if (!Number.isInteger(value)) throw new Error(`${name} must be an integer`);
  return value;
};

const release = Object.freeze({
  core_version: process.env.RESTLESS_CORE_VERSION,
  source_revision: process.env.RESTLESS_SOURCE_REVISION,
  api_contract_version: integer('RESTLESS_API_CONTRACT_VERSION'),
  assertion_contract_version: integer('RESTLESS_ASSERTION_CONTRACT_VERSION'),
  schema_version: integer('RESTLESS_SCHEMA_VERSION'),
});

if (!release.core_version || !release.source_revision || release.source_revision === 'unknown') {
  throw new Error('Runtime image is missing its build-baked release identity');
}

createServer((request, response) => {
  response.setHeader('Content-Type', 'application/json');
  if (request.method === 'GET' && request.url === '/health') {
    response.writeHead(200);
    response.end(JSON.stringify({ status: 'ok', release }));
    return;
  }
  response.writeHead(404);
  response.end(JSON.stringify({ status: 'not_found' }));
}).listen(7789, '0.0.0.0');
