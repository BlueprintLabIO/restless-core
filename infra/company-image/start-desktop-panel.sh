#!/bin/sh
set -eu

/usr/local/bin/wait-for-company-display true
/usr/bin/xrdb -merge /etc/restless/Xresources

# Openbox deliberately owns windows only. Keep the canvas light and unobtrusive;
# the panel and Breeze window chrome carry the slate and blue brand colors.
/usr/bin/xsetroot -solid '#e9edf5'
exec /usr/bin/tint2 -c /etc/restless/tint2rc
