# S38-T3 — Add native supervision and wake adapters

**Layer:** Machine host and schedule ingress

**Serves:** Due work needs an OS-owned wake path even when the daemon's in-process timer or process
lifecycle fails.

## Work

- Add a macOS `launchd` wake-only entry and a Linux `systemd` equivalent targeting one authenticated
  `wake-due` operation.
- Keep company prompts, task payloads and credentials out of OS definitions and process arguments.
- Record wake delivery time and adapter identity as transport evidence.
- Retain the in-process next-due timer; make duplicate native and internal wakes safe.
- Detect and report unsupported installation, permissions and machine-sleep guarantees.

## Acceptance

Native and in-process wakes each discover the same due schedule through Restless. Duplicate, delayed
and reordered wakes create one claim. Removing or corrupting the OS entry becomes a visible degraded
condition rather than a silent missed schedule.

## Makes deletable

One cron entry per task, prompt-bearing OS jobs and the assumption that a live Tokio timer is durable.
