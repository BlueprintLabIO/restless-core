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

const invokedPath = process.argv[1];
if (invokedPath !== undefined && import.meta.url === pathToFileURL(invokedPath).href) {
  void run().catch(() => {
    process.stderr.write('native Documents collaboration service failed to start\n');
    process.exitCode = 1;
  });
}
