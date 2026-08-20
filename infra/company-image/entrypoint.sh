#!/bin/sh
# company-init: seed the persistent /company volume on first boot, then be a
# long-lived init. This single file is the whole inversion away from the
# legacy per-turn disposable sandbox: the container exists to be a company
# computer, not to run an agent.
set -eu

mkdir -p /tmp/.X11-unix
chmod 1777 /tmp/.X11-unix
mkdir -p /tmp/restless-effect
chown effect:company /tmp/restless-effect
chmod 0700 /tmp/restless-effect

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
	/company/run \
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
chown company:company /company/home /company/home/Desktop

# The imported supervisor owns durable desktop/browser services. tini remains
# PID 1 and reaps both those services and ordinary agent processes started by
# the Runtime Bridge.
exec tini -- /usr/bin/supervisord -n -c /etc/supervisor/conf.d/restless.conf
