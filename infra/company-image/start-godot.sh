#!/bin/sh
set -eu

/usr/local/bin/wait-for-company-display true
export HOME=/company/home
exec /usr/local/bin/godot --editor
