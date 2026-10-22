#!/bin/sh
set -eu

# The company computer's own container (S104 F2) starts here instead of the Runtime's trusted
# supervisor. It drops to the company's uid/gid with the same shape as the Runtime's company
# supervisor, then runs only the desktop programs.
if [ "$(id -u)" = "0" ]; then
	exec /usr/bin/setpriv \
		--reuid=2000 \
		--regid=2000 \
		--clear-groups \
		--inh-caps=-all \
		--ambient-caps=-all \
		--bounding-set=-all \
		--no-new-privs \
		/usr/local/bin/start-computer-supervisor
fi

expected_uid=2000
expected_gid=2000
if [ "$(id -u)" != "$expected_uid" ] || [ "$(id -g)" != "$expected_gid" ]; then
	printf 'computer supervisor must start as uid/gid %s:%s (got %s:%s)\n' \
		"$expected_uid" "$expected_gid" "$(id -u)" "$(id -g)" >&2
	exit 1
fi

/usr/local/bin/assert-runtime-privileges company
umask 077
# The Runtime made /company/run; wait for it rather than racing its first boot.
i=0
while [ ! -d /company/run ] && [ "$i" -lt 60 ]; do
	i=$((i + 1))
	sleep 1
done
exec /usr/bin/supervisord -n -c /etc/supervisor/conf.d/computer.conf
