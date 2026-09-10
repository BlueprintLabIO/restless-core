#!/bin/sh
set -eu

# Local appliance mode deliberately keeps its direct-Docker transport. Only a
# Fleet-created hosted Runtime receives this exact outbound bridge endpoint.
if [ -z "${RESTLESS_RUNTIME_BRIDGE_URL:-}" ]; then
  exit 0
fi

if [ "$(id -g)" = "0" ]; then
	# The bridge uses the company primary group only for ordinary Runtime paths
	# and one dedicated supplementary group only for its read-only capability
	# mount. No company process inherits either trusted group arrangement.
	exec /usr/bin/setpriv \
		--regid=2000 \
		--groups=10002 \
		/usr/local/bin/start-runtime-bridge
fi

/usr/local/bin/assert-runtime-privileges trusted-bridge
exec /usr/local/bin/restless-runtime-bridge
