# Local native Documents credential rotation

Use this only for a confirmed exposed **native Documents sidecar** database
password on a local Restless appliance. It does not rotate the cell owner's
`database.url`, a hosted cell's Infisical custody copy, or other MCP/provider
credentials. Treat journal entries that held the old password as sensitive even
after rotation.

## Maintenance sequence

1. Deploy the SQLx statement-logging suppression first. Identify the affected
   company from the role name and check that its local Docs sidecar is healthy.
   Schedule a short Docs editing pause so no collaborator has an unflushed edit.
2. Drain Restless work and stop the `restless-local.service` account plane. Do
   not run the rotation beside a live daemon: the one-shot command takes the
   same machine-profile singleton lock and refuses that race.
3. Run the staged `restlessd rotate-native-documents-credential <company>`
   binary with the **same profile environment and working directory** as the
   stopped service. Pass only the company name; no password appears in argv,
   terminal output, or the command line. The command gracefully stops only
   that company's Docs container, stages a private pending URL, changes the
   exact Docs role using a locally derived SCRAM verifier, terminates prior
   sessions, proves the new URL connects to the intended cell and that the old
   URL fails password authentication, then atomically replaces
   `native-documents-database.url`. It leaves a non-secret restart marker tied
   to the new password. PostgreSQL can log the verifier from the DDL, but does
   not receive the new plaintext password in that statement.
4. Restart `restless-local.service`. Startup sees the marker and recreates the
   old Docs container, because a Docker file bind mount keeps the old inode
   after an atomic file replacement. The new sidecar reads the new credential
   at startup. Its readiness check queries PostgreSQL, then Core clears the
   marker. Verify one real document read and save for the affected company.

If the one-shot command fails, do not remove the pending URL or marker by hand.
Rerun the same new binary after resolving the reported problem. Ordinary cell
startup refuses to reinstall the old password while a pending URL exists. A
crash after the file was promoted but before sidecar recreation leaves the
marker in place, so the next startup still replaces the old container. A
failed graceful stop halts before changing the password, allowing document
state to be inspected. This is a maintenance operation with a brief Docs
outage; it does not require stopping unrelated companies' Docs containers.
Do not restart the service until the one-shot command reports success. The
account plane must stay stopped during rotation, even if only one Docs
container is being changed.

The local command intentionally refuses network-hosted Docs deployments.
Hosted rotation also requires coordinated Infisical publication and a managed
sidecar rollout; reusing this local command there would leave the hosted
credential consumer on the old value.
