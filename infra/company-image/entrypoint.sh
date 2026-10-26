#!/bin/sh
# company-init: seed the persistent /company volume on first boot, then be a
# long-lived init. This single file is the whole inversion away from the
# legacy per-turn disposable sandbox: the container exists to be a company
# computer, not to run an agent.
set -eu

# Hosted Runtimes use a read-only root filesystem. `/tmp` and `/run` are
# explicit ephemeral tmpfs mounts in the container contract; fail here instead
# of half-starting against the durable company volume when either is absent.
for runtime_dir in /tmp /run; do
	if [ ! -d "$runtime_dir" ] || [ ! -w "$runtime_dir" ]; then
		printf 'company Runtime requires a writable ephemeral %s mount\n' "$runtime_dir" >&2
		exit 1
	fi
done

case "${RESTLESS_RUNTIME_BRIDGE_CAPABILITY_FILE:-}" in
	/company|/company/*)
		printf 'Runtime bridge capability must not live on the company volume\n' >&2
		exit 1
		;;
esac

# The privileged supervisor's complete control surface stays outside
# company-writable storage. The directory is recreated on every boot.
mkdir -p /run/restless/trusted-supervisor
chmod 0700 /run/restless /run/restless/trusted-supervisor

mkdir -p /tmp/.X11-unix
chmod 1777 /tmp/.X11-unix
# Kubernetes keeps an emptyDir across container restarts (Docker's tmpfs starts empty), so this
# must be safe to run again: once `effect` owns the directory, init holds CHOWN but not FOWNER
# and can no longer chmod it. Fix it up only while it is still ours, then require the final state.
mkdir -p /tmp/restless-effect
if [ "$(stat -c '%U' /tmp/restless-effect)" = root ]; then
	chmod 0700 /tmp/restless-effect
	chown effect:company /tmp/restless-effect
fi
if [ "$(stat -c '%U:%G:%a' /tmp/restless-effect)" != "effect:company:700" ]; then
	printf 'company Runtime effect directory is not effect:company 0700\n' >&2
	exit 1
fi

# `/company` is root:company 1770: group-writable for the company, sticky so the
# company cannot rename or replace root's custody folders inside it. Volumes made
# before 27 Oct 2026 are 2000:2000; init takes them over with CHOWN alone (once
# root owns a path it may chmod it without FOWNER). Any other owner fails closed:
# root never repairs a volume it does not recognise. Nothing of the company runs
# yet, so nothing can race these steps.
if [ ! -d /company ] || [ -L /company ]; then
	printf 'company Runtime requires one plain persistent /company directory\n' >&2
	exit 1
fi
case "$(stat -c '%u:%g' /company)" in
	0:2000) ;;
	2000:2000) chown 0:2000 /company ;;
	*)
		printf 'company Runtime persistent volume must be owned by root:company or uid/gid 2000:2000\n' >&2
		exit 1
		;;
esac
chmod 1770 /company
# Custody folders: sealed review copies and each connected tool's private data.
# Owner-writable while they are ensured (no DAC_OVERRIDE), final modes after.
for custody in /company/reviews /company/reviews/git /company/reviews/by-attempt /company/tool-data; do
	if [ -L "$custody" ]; then
		rm -f "$custody"
	fi
	if [ -e "$custody" ] && [ ! -d "$custody" ]; then
		printf 'company Runtime custody path %s is not a directory\n' "$custody" >&2
		exit 1
	fi
	[ -d "$custody" ] || mkdir "$custody"
	chown 0:2000 "$custody"
	chmod 0750 "$custody"
done
chmod 0550 /company/reviews/git /company/reviews/by-attempt /company/reviews
chmod 0711 /company/tool-data

# Persistent initialization and image-upgrade migrations execute as the
# ordinary company identity. setpriv clears the inherited bridge-secret group,
# every capability set (including the bounding set), and sets NNP before the
# first durable write.
/usr/bin/setpriv \
	--reuid=2000 \
	--regid=2000 \
	--clear-groups \
	--inh-caps=-all \
	--ambient-caps=-all \
	--bounding-set=-all \
	--no-new-privs \
	/usr/local/bin/company-volume-init

# The immutable trusted supervisor owns only the Runtime Bridge and the
# uid-2000 company-supervisor launcher. The latter owns desktop/browser and
# company-authored services through its separate company-writable socket.
# tini remains PID 1 and reaps both trees.
exec tini -- /usr/bin/supervisord -n -c /etc/supervisor/conf.d/restless.conf
