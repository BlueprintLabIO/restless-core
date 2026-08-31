#!/bin/sh
set -eu

# Project work can add one purposeful launcher under the company-owned XDG
# applications directory. Reload tint2 so it is visible without restarting
# the persistent desktop or asking the owner to open a terminal.
/usr/local/bin/wait-for-company-display true
pkill -USR1 -x tint2 2>/dev/null || true
