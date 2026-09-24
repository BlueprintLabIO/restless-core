#!/bin/sh
set -eu

/usr/local/bin/wait-for-company-display true
exec /usr/bin/xterm -fa 'DejaVu Sans Mono' -fs 12 -geometry 96x24 -title 'Company Terminal' \
	-e /bin/sh -lc 'cd /company/home; exec /bin/bash -l'
