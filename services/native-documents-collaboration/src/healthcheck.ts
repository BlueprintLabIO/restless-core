import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

import { READY_PATH } from './constants.js';

export function healthcheckPort(environment: NodeJS.ProcessEnv): number {
  const raw = environment.RESTLESS_NATIVE_DOCUMENTS_LISTEN_PORT ?? '6688';
  if (!/^[1-9][0-9]{0,4}$/.test(raw)) throw new Error('invalid healthcheck port');
  const port = Number(raw);
  if (port > 65_535) throw new Error('invalid healthcheck port');
  return port;
}

export async function checkReadiness(
  environment: NodeJS.ProcessEnv = process.env,
  fetchImplementation: typeof fetch = fetch,
): Promise<boolean> {
  try {
    const response = await fetchImplementation(`http://127.0.0.1:${healthcheckPort(environment)}${READY_PATH}`, {
      headers: { accept: 'application/json' },
      redirect: 'error',
      signal: AbortSignal.timeout(3_000),
    });
    await response.body?.cancel();
    return response.status === 200;
  } catch {
    return false;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exitCode = (await checkReadiness()) ? 0 : 1;
}
