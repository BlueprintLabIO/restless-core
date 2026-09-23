// The self-hosted host: static Placement, SMTP Mail, one Node listener.
import { createServer } from "node:http";
import { readConfig } from "./config.mjs";
import { createIssuer } from "./issuer.mjs";
import { smtpMail } from "./mail-smtp.mjs";
import { staticPlacement } from "./placement-static.mjs";
const config = await readConfig(process.env.RESTLESS_IDENTITY_CONFIG);
const mail = smtpMail(config.smtp);
const issuer = await createIssuer(config, {
  placement: staticPlacement(config),
  mail,
});
const server = createServer(issuer.handle);
server.listen(config.port, config.address, () =>
  console.log(`Restless accounts listening at ${config.origin}`),
);
async function close() {
  server.closeIdleConnections();
  await new Promise((resolve) => server.close(resolve));
  mail.close();
  await issuer.close();
}
process.once("SIGTERM", () => {
  void close().then(() => process.exit(0));
});
process.once("SIGINT", () => {
  void close().then(() => process.exit(0));
});
