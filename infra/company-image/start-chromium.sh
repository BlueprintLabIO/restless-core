#!/bin/sh
set -eu

/usr/local/bin/wait-for-company-display true

tabs=/company/browser-profile/restless-tabs.json

set -- \
	/usr/bin/chromium \
	--display=:1 \
	--user-data-dir=/company/browser-profile \
	--download-default-directory=/company/downloads \
	--remote-debugging-address=127.0.0.1 \
	--remote-debugging-port=9222 \
	--start-maximized \
	--password-store=basic \
	--no-first-run \
	--no-default-browser-check \
	--disable-dev-shm-usage \
	--disable-background-networking \
	--no-sandbox

urls=/company/run/restless-tabs.urls
: > "$urls"
if [ -s "$tabs" ] && jq -e 'type == "array"' "$tabs" >/dev/null 2>&1; then
	jq -r '.[] | select(type == "string") | select(startswith("http://") or startswith("https://") or startswith("file://"))' "$tabs" > "$urls"
fi

if [ -s "$urls" ]; then
	# Chromium's own session store does not record tabs navigated through CDP
	# in this Runtime's Debian build. Use the broker checkpoint and disable
	# native restore so those URLs are not opened a second time. Its session
	# files remain intact in the profile.
	preferences=/company/browser-profile/Default/Preferences
	if [ -f "$preferences" ]; then
		jq '.session.restore_on_startup = 5' "$preferences" > /company/run/Preferences.next
		mv /company/run/Preferences.next "$preferences"
	fi
	while IFS= read -r url; do
		set -- "$@" "$url"
	done < "$urls"
else
	# Missing, malformed, or URL-free checkpoints leave native recovery enabled.
	set -- "$@" --restore-last-session
fi

exec "$@"
