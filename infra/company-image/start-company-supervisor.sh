#!/bin/sh
set -eu

if [ "$(id -u)" = "0" ]; then
	# The trusted supervisor owns this one transition. The company supervisor
	# and every service it launches receive no supplementary groups, no
	# capability set (including no bounding capabilities), and NNP.
	exec /usr/bin/setpriv \
		--reuid=2000 \
		--regid=2000 \
		--clear-groups \
		--inh-caps=-all \
		--ambient-caps=-all \
		--bounding-set=-all \
		--no-new-privs \
		/usr/local/bin/start-company-supervisor
fi

expected_uid=2000
expected_gid=2000
actual_uid="$(id -u)"
actual_gid="$(id -g)"
if [ "$actual_uid" != "$expected_uid" ] || [ "$actual_gid" != "$expected_gid" ]; then
	printf 'company supervisor must start as uid/gid %s:%s (got %s:%s)\n' \
		"$expected_uid" "$expected_gid" "$actual_uid" "$actual_gid" >&2
	exit 1
fi

/usr/local/bin/assert-runtime-privileges company
umask 077
/usr/local/bin/validate-company-supervisor-config /company/services/supervisor
exec /usr/bin/supervisord -n -c /etc/supervisor/conf.d/company.conf
