import { createServer } from "node:http";
import nodemailer from "nodemailer";
import { readConfig } from "./config.mjs";
import { createIdentityService } from "./server.mjs";
const config = await readConfig(process.env.RESTLESS_IDENTITY_CONFIG);
if (!config.smtp?.host || !config.smtp?.from)
  throw Error(
    "SMTP host and from address are required for verification and invitations",
  );
const { from, ...smtp } = config.smtp;
const transport = nodemailer.createTransport({
  ...smtp,
  connectionTimeout: 10000,
  greetingTimeout: 10000,
  socketTimeout: 15000,
});
const service = await createIdentityService(config, {
  sendMail: (message) => transport.sendMail({ from, ...message }),
});
const server = createServer(service.handle);
server.listen(config.port, config.address, () =>
  console.log(`Restless accounts listening at ${config.origin}`),
);
async function close() {
  server.closeIdleConnections();
  await new Promise((resolve) => server.close(resolve));
  transport.close();
  await service.close();
}
process.once("SIGTERM", () => {
  void close().then(() => process.exit(0));
});
process.once("SIGINT", () => {
  void close().then(() => process.exit(0));
});
