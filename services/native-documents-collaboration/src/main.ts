import { pathToFileURL } from 'node:url';

import { configFromEnvironment } from './config.js';
import { PostgresDocumentStore } from './postgres-store.js';
import { NativeDocumentsCollaborationServer } from './server.js';

export async function run(): Promise<void> {
  const config = await configFromEnvironment();
  const store = new PostgresDocumentStore({ companyId: config.companyId, databaseUrl: config.databaseUrl });
  const server = new NativeDocumentsCollaborationServer(config, { store });
  let stopping = false;

  const stop = async (): Promise<void> => {
    if (stopping) return;
    stopping = true;
    try {
      await server.destroy();
      process.exitCode = 0;
    } catch {
      process.stderr.write('native Documents collaboration service could not shut down cleanly\n');
      process.exitCode = 1;
    }
  };
  const onSignal = (): void => {
    void stop();
  };
  process.once('SIGINT', onSignal);
  process.once('SIGTERM', onSignal);

  try {
    await server.listen();
    process.stdout.write('native Documents collaboration service is listening\n');
  } catch (error) {
    process.removeListener('SIGINT', onSignal);
    process.removeListener('SIGTERM', onSignal);
    throw error;
  }
}

/** Why startup failed, without secrets: the error's kind, code and first line, with any URL
 * credentials and long tokens removed. Without it a crash loop says nothing (it restarted
 * thousands of times on 24 October with no cause in its log). */
export function startupFailure(error: unknown): string {
  if (!(error instanceof Error)) return 'unknown error';
  const code = (error as { code?: unknown }).code;
  const line = (error.message.split('\n')[0] ?? '')
    .replace(/([a-z][a-z0-9+.-]*:\/\/)[^/@\s]*@/gi, '$1***@')
    .replace(/[A-Za-z0-9_\-.]{32,}/g, '***')
    .slice(0, 200);
  return `${error.name}${typeof code === 'string' ? ` ${code}` : ''}: ${line}`;
}

const invokedPath = process.argv[1];
if (invokedPath !== undefined && import.meta.url === pathToFileURL(invokedPath).href) {
  void run().catch((error: unknown) => {
    process.stderr.write(`native Documents collaboration service failed to start: ${startupFailure(error)}\n`);
    process.exitCode = 1;
  });
}
