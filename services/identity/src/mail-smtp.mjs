import nodemailer from "nodemailer";
// Mail over SMTP. Verification is mandatory, so an issuer without mail cannot
// prove anyone's address and refuses to start.
export function smtpMail(settings) {
  if (!settings?.host || !settings?.from)
    throw Error(
      "SMTP host and from address are required for verification and invitations",
    );
  const { from, ...smtp } = settings;
  const transport = nodemailer.createTransport({
    ...smtp,
    connectionTimeout: 10000,
    greetingTimeout: 10000,
    socketTimeout: 15000,
  });
  return {
    send: (message) => transport.sendMail({ from, ...message }),
    close: () => transport.close(),
  };
}
