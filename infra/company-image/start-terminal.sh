#!/bin/sh
set -eu

/usr/local/bin/wait-for-company-display true
exec /usr/bin/xterm -fa 'IBM Plex Mono' -fs 11 -title 'Company Terminal' \
	-e /bin/sh -lc 'cd /company/home; exec /bin/bash -l'
