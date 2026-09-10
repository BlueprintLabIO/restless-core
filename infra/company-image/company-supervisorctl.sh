#!/bin/sh
set -eu

# Validate before every supported reread/update. Supervisord itself remains an
# unprivileged uid-2000 process, so a validation race can cause only company-
# local failure; it cannot select root or reach the trusted supervisor.
/usr/local/bin/validate-company-supervisor-config /company/services/supervisor
exec /usr/bin/supervisorctl -c /etc/supervisor/conf.d/company.conf "$@"
