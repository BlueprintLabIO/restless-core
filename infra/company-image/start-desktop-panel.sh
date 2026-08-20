#!/bin/sh
set -eu

/usr/local/bin/wait-for-company-display true

# Openbox deliberately owns windows only. The quiet root colour and imported
# taskbar make the persistent session legible without introducing a full DE.
/usr/bin/xsetroot -solid '#1b2028'
exec /usr/bin/tint2 -c /etc/restless/tint2rc
