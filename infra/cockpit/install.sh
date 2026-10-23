#!/bin/sh
# Installs this release's owner cockpit into the folder the plane serves it from, then stays up so
# a deployment can see it is in place (S104: screen-only releases without restarting the plane).
#
# Content-hashed assets (_app/immutable) are added beside the ones already there, never replaced,
# so a tab still running the previous version keeps loading its own chunks; those older than two
# weeks are pruned. Everything else is copied over, and index.html goes last, by rename, so the
# plane never serves an index whose assets are not yet in place. `check` reports whether the
# folder holds this image's index.
set -eu
source_dir=/cockpit
target="${RESTLESS_COCKPIT_TARGET:-/var/lib/restless/cockpit}"

if [ "${1:-}" = check ]; then
  cmp -s "$source_dir/index.html" "$target/index.html"
  exit $?
fi

mkdir -p "$target"
if [ -d "$source_dir/_app/immutable" ]; then
  mkdir -p "$target/_app/immutable"
  cp -a "$source_dir/_app/immutable/." "$target/_app/immutable/"
  find "$target/_app/immutable" -type f -mtime +14 -delete 2>/dev/null || true
fi
# Everything but the immutable assets and the index, then the index by rename.
(cd "$source_dir" && find . -path ./_app/immutable -prune -o -type f ! -name index.html -print) |
  while IFS= read -r file; do
    mkdir -p "$target/$(dirname "$file")"
    cp -a "$source_dir/$file" "$target/$file.next" && mv -f "$target/$file.next" "$target/$file"
  done
cp -a "$source_dir/index.html" "$target/index.html.next"
mv -f "$target/index.html.next" "$target/index.html"
echo "cockpit installed in $target"

trap 'exit 0' TERM INT
while :; do sleep 3600 & wait $!; done
