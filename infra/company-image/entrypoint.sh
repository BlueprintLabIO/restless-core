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
mkdir -p /tmp/restless-effect
chown effect:company /tmp/restless-effect
chmod 0700 /tmp/restless-effect

# `/company` is created as uid/gid 2000 in the immutable image. Docker must
# preserve that ownership when it initializes an empty named volume, and every
# existing upgrade volume must already have the same owner. Root never repairs
# a divergent durable volume: without DAC_OVERRIDE that would either half-boot
# or require broad filesystem authority. Fail closed and let Fleet restore or
# explicitly migrate the volume instead.
if [ ! -d /company ] || [ -L /company ]; then
	printf 'company Runtime requires one plain persistent /company directory\n' >&2
	exit 1
fi
if [ "$(stat -c '%u:%g' /company)" != "2000:2000" ]; then
	printf 'company Runtime persistent volume must be owned by uid/gid 2000:2000\n' >&2
	exit 1
fi

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
