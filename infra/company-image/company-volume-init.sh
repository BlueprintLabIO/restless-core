#!/bin/sh
# Initialize and upgrade only the persistent company filesystem. company-init
# enters this script as uid/gid 2000 with no supplementary groups, no
# capabilities, an empty bounding set, and no-new-privileges.
set -eu

/usr/local/bin/assert-runtime-privileges company
umask 0007

mkdir -p \
	/company/org \
	/company/projects \
	/company/knowledge \
	/company/outputs \
	/company/repos \
	/company/worktrees \
	/company/reviews/git \
	/company/reviews/by-attempt \
	/company/home \
	/company/browser-profile \
	/company/downloads \
	/company/home/Desktop \
	/company/home/.local/share/applications \
	/company/run/attempts \
	/company/run/gates \
	/company/services/supervisor

if [ ! -f /company/.seeded ]; then
	if [ ! -f /company/mission.md ]; then
		printf '# Mission\n\n(unset — the owner sets this via the company config)\n' > /company/mission.md
	fi
	touch /company/.seeded
fi

# Chromium's profile preference is the observed setting behind “continue
# where you left off”. Preserve every other preference during image upgrades.
mkdir -p /company/browser-profile/Default
preferences=/company/browser-profile/Default/Preferences
if [ -f "$preferences" ]; then
	jq '.session.restore_on_startup = 1' "$preferences" > /company/run/Preferences.next
	mv /company/run/Preferences.next "$preferences"
else
	printf '{"session":{"restore_on_startup":1}}\n' > "$preferences"
fi

# Expose durable company places without replacing owner-created entries.
for place in Downloads Projects Outputs; do
	case "$place" in
		Downloads) target=/company/downloads ;;
		Projects) target=/company/projects ;;
		Outputs) target=/company/outputs ;;
	esac
	link="/company/home/$place"
	if [ ! -e "$link" ] && [ ! -L "$link" ]; then
		ln -s "$target" "$link"
	fi
done

# Keep immutable, versioned Godot templates in the image and only link the
# selected version into the persistent user home.
godot_templates_source=/opt/restless/godot/export_templates/4.7.2.stable
godot_templates_parent=/company/home/.local/share/godot/export_templates
godot_templates_link="${godot_templates_parent}/4.7.2.stable"
if [ -d "$godot_templates_source" ]; then
	mkdir -p "$godot_templates_parent"
	if [ ! -e "$godot_templates_link" ] && [ ! -L "$godot_templates_link" ]; then
		ln -s "$godot_templates_source" "$godot_templates_link"
	fi
fi
