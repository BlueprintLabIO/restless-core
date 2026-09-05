#!/bin/sh
# company-init: seed the persistent /company volume on first boot, then be a
# long-lived init. This single file is the whole inversion away from the
# legacy per-turn disposable sandbox: the container exists to be a company
# computer, not to run an agent.
set -eu

# Lock the mounted directory inode, not an owner-unlinkable lock file. FD 9
# survives exec into tini; its child closes the descriptor before Supervisor
# starts, so company processes cannot inherit and unlock it. This coordinates
# cooperating inits on this filesystem, not independent restored copies or an
# older image without the guard. The host must still fence the previous Runtime.
exec 9</company
if ! flock --exclusive --nonblock 9; then
	echo 'company filesystem is already in use, or does not support runtime locking' >&2
	exit 75
fi
python3 /usr/local/lib/restless/recover-browser-profile.py

mkdir -p /tmp/.X11-unix
chmod 1777 /tmp/.X11-unix
mkdir -p /tmp/restless-effect
chown effect:company /tmp/restless-effect
chmod 0700 /tmp/restless-effect

# Self-hosted company images have no bridge environment and therefore no
# Runtime Agent process. Hosted Fleet supplies the exact identity environment;
# only then stage its capability/state and materialise its supervisor program.
runtime_agent_supervisor_dir=/run/restless-supervisor
install -d -o root -g root -m 0755 "$runtime_agent_supervisor_dir"
if [ -n "${RESTLESS_RUNTIME_BRIDGE_URL:-}" ]; then
	# Fleet mounts the one-use bootstrap root-readable. Stage it into the
	# cell's private /run tmpfs. The process immediately enters the dedicated
	# UID 2002 custody boundary, persists the rotation, then unlinks this copy.
	runtime_bridge_bootstrap_source=/run/secrets/restless-runtime-bridge-capability
	runtime_bridge_bootstrap_dir=/run/restless-agent
	runtime_bridge_bootstrap_target=${runtime_bridge_bootstrap_dir}/runtime-bridge-bootstrap
	if [ -f "$runtime_bridge_bootstrap_source" ]; then
		install -d -o runtime-agent -g runtime-agent -m 0700 "$runtime_bridge_bootstrap_dir"
		install -o runtime-agent -g runtime-agent -m 0400 \
			"$runtime_bridge_bootstrap_source" \
			"$runtime_bridge_bootstrap_target"
	fi

	# Rotation and idempotency receipts survive sleep/wake in the separate
	# per-cell control volume, outside company data/export/model custody.
	mkdir -p /var/lib/restless-runtime-agent
	chown root:root /var/lib/restless-runtime-agent
	chmod 0700 /var/lib/restless-runtime-agent
	chown runtime-agent:runtime-agent /var/lib/restless-runtime-agent

	printf '%s\n' \
		'[program:runtime-agent]' \
		'command=/usr/local/bin/restless-runtime-agent' \
		'user=root' \
		'environment=HOME="/var/lib/restless-runtime-agent",RESTLESS_RUNTIME_BRIDGE_CAPABILITY_FILE="/run/restless-agent/runtime-bridge-bootstrap",RESTLESS_RUNTIME_BRIDGE_CAPABILITY_STATE_FILE="/var/lib/restless-runtime-agent/runtime-bridge-capability"' \
		'priority=5' \
		'autostart=true' \
		'autorestart=true' \
		'startsecs=2' \
		'startretries=10' \
		'stopsignal=TERM' \
		'stopwaitsecs=10' \
		'stdout_logfile=/company/run/runtime-agent.log' \
		'stderr_logfile=/company/run/runtime-agent.log' \
		> "$runtime_agent_supervisor_dir/runtime-agent.conf"
	chmod 0444 "$runtime_agent_supervisor_dir/runtime-agent.conf"
fi

# Image upgrades may add durable directories after a volume has already been
# seeded. Ensure the current runtime shape on every boot; `.seeded` only says
# the original company filesystem exists, not that it has every later path.
mkdir -p \
	/company/org \
	/company/projects \
	/company/knowledge \
	/company/outputs \
	/company/repos \
	/company/home \
	/company/browser-profile \
	/company/downloads \
	/company/home/Desktop \
	/company/home/.local/share/applications \
	/company/run \
	/company/run/sessions \
	/company/services/supervisor

if [ ! -f /company/.seeded ]; then
	if [ ! -f /company/mission.md ]; then
		printf '# Mission\n\n(unset — the owner sets this via the company config)\n' > /company/mission.md
	fi
	touch /company/.seeded
	chown -R company:company /company
fi

# Chromium's enterprise-policy discovery differs across distro builds. The
# profile preference is the browser-owned, documented setting behind “continue
# where you left off”; seed/refresh that one key before Chromium starts rather
# than pretending an unobserved policy loaded. Preserve every other preference.
mkdir -p /company/browser-profile/Default
preferences=/company/browser-profile/Default/Preferences
if [ -f "$preferences" ]; then
	jq '.session.restore_on_startup = 1' "$preferences" > /company/run/Preferences.next
	mv /company/run/Preferences.next "$preferences"
else
	printf '{"session":{"restore_on_startup":1}}\n' > "$preferences"
fi
chown -R company:company /company/browser-profile /company/downloads /company/run

# A file manager should expose durable company places by names an ordinary
# owner recognises. These are links into the existing source-owned filesystem,
# not copied assets or a second custody lifecycle. Never replace an owner-created
# file or link at the same path.
for place in Downloads Projects Outputs; do
	case "$place" in
		Downloads) target=/company/downloads ;;
		Projects) target=/company/projects ;;
		Outputs) target=/company/outputs ;;
	esac
	link="/company/home/$place"
	if [ ! -e "$link" ] && [ ! -L "$link" ]; then
		ln -s "$target" "$link"
		chown -h company:company "$link"
	fi
done
chown company:company /company/home /company/home/Desktop /company/home/.local /company/home/.local/share /company/home/.local/share/applications

# Godot's export lookup is user-scoped even though the pinned engine/templates
# belong to the immutable Runtime image. Link the versioned image templates
# into the persistent company home instead of copying a second mutable engine
# payload into every project or volume. A future engine version uses a distinct
# directory, leaving an existing version reference intact for reproducibility.
godot_templates_source=/opt/restless/godot/export_templates/4.7.2.stable
godot_templates_parent=/company/home/.local/share/godot/export_templates
godot_templates_link="${godot_templates_parent}/4.7.2.stable"
if [ -d "$godot_templates_source" ]; then
	mkdir -p "$godot_templates_parent"
	if [ ! -e "$godot_templates_link" ] && [ ! -L "$godot_templates_link" ]; then
		ln -s "$godot_templates_source" "$godot_templates_link"
		chown -h company:company "$godot_templates_link"
	fi
fi

# The imported supervisor owns durable desktop/browser services. tini remains
# PID 1 and reaps both those services and ordinary agent processes started by
# the Runtime Bridge.
exec tini -- /bin/sh -c 'exec 9<&-; exec /usr/bin/supervisord -n -c /etc/supervisor/conf.d/restless.conf'
