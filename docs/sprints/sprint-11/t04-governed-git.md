# S11-T4 · Prove Git as an acquired governed capability

**Layer:** Runtime owns Git, skills and credential delivery; Authority owns approval, scoped secret
access, consequential publication and receipt.

**Observed friction:** Staff could inspect and commit locally but concluded it could not push because
`restless effect` was not an ACP-native tool and Git credentials were not discoverable through the
ordinary command path.

## Outcome

Git is the first concrete external-capability acquisition pattern. Agents use the installed CLI and
a small publication skill. Public reads and local work remain ordinary Runtime operations;
credentialed access is scoped; a push crosses the generic effect boundary and produces a receipt.

## Acceptance

- Public clone/fetch works without requesting a root credential.
- Credentialed access obtains only the scoped GitHub secret and does not print it in process output,
  agent transcript or cockpit events.
- Staff reads the publication skill and can explain the ordinary-work/effect boundary.
- An unauthorised push fails closed with the exact required party/scope.
- An authorised branch push succeeds through `restless effect`, records idempotency and receipt, and
  provider state is read back independently.
- No provider-specific kernel command, capability registry or marketplace is added.

## Deletion

Makes embedded tokens, manual owner push instructions, bespoke GitHub publication RPCs and
capability-unavailable prompt workarounds deletable.
