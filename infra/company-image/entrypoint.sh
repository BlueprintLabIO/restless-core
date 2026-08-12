#!/bin/sh
# company-init: seed the persistent /company volume on first boot, then be a
# long-lived init. This single file is the whole inversion away from the
# legacy per-turn disposable sandbox: the container exists to be a company
# computer, not to run an agent.
set -eu

if [ ! -f /company/.seeded ]; then
	mkdir -p \
		/company/org \
		/company/goals \
		/company/projects \
		/company/decisions \
		/company/knowledge \
		/company/outputs \
		/company/repos \
		/company/workspaces \
		/company/home
	if [ ! -f /company/mission.md ]; then
		printf '# Mission\n\n(unset — the owner sets this via the company config)\n' > /company/mission.md
	fi
	touch /company/.seeded
	chown -R company:company /company
fi

# tini as PID 1 reaps zombies; `sleep infinity` keeps the computer on.
# Agents arrive later via `docker exec -u company`.
exec tini -g -- sleep infinity
