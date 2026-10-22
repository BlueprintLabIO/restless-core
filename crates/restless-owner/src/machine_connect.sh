#!/bin/sh
# Lend this computer to your Restless plane (Settings › Computers › Add a computer).
#
# It starts one container, "restless-machine", that opens a single outbound connection to your
# plane. Nothing listens on this computer and no port is opened. Restless can then run its own
# sealed containers here, and nothing else: it cannot read your files or touch your other
# containers. Remove the computer in Settings › Computers to cut it off at once.
set -eu

PLANE="__RESTLESS_PLANE__"
CODE="${1:-}"
HELPER=restless-machine

if [ -z "$CODE" ]; then
  echo "Usage: curl -fsSL $PLANE/connect | sh -s -- <code from Settings › Computers>" >&2
  exit 1
fi
if ! command -v docker >/dev/null 2>&1; then
  echo "Restless runs on Docker, which is not installed here." >&2
  echo "Install Docker Desktop (https://www.docker.com/products/docker-desktop/) or Docker Engine, start it, then run this again." >&2
  exit 1
fi
if ! docker info >/dev/null 2>&1; then
  echo "Docker is installed but not running (or needs sudo). Start it, then run this again." >&2
  exit 1
fi

echo "Fetching the Restless computer image from your plane. This is a few GB, once."
LOADED=$(curl -fSL --progress-bar -H "X-Restless-Pairing: $CODE" "$PLANE/machines/helper-image" | docker load -q)
IMAGE=$(printf '%s\n' "$LOADED" | sed -n 's/^Loaded image ID: //p; s/^Loaded image: //p' | tail -n 1)
if [ -z "$IMAGE" ]; then
  echo "Could not fetch the image. A code lasts 10 minutes and works once; make a new one and try again." >&2
  exit 1
fi

# Replace an earlier helper, never a container Restless did not make.
if [ "$(docker inspect -f '{{index .Config.Labels "io.restless.helper"}}' "$HELPER" 2>/dev/null || true)" = "1" ]; then
  docker rm -f "$HELPER" >/dev/null
elif docker inspect "$HELPER" >/dev/null 2>&1; then
  echo "A container named $HELPER already exists and is not Restless's. Rename or remove it, then run this again." >&2
  exit 1
fi
docker volume create restless-machine-state >/dev/null
docker run -d --name "$HELPER" --restart unless-stopped \
  --label io.restless.helper=1 \
  --user 0 \
  -v /var/run/docker.sock:/var/run/docker.sock \
  -v restless-machine-state:/var/lib/restless-machine \
  -e RESTLESS_MACHINE_NAME="$(hostname 2>/dev/null || echo 'My computer')" \
  --entrypoint /usr/local/bin/restless-machine \
  "$IMAGE" connect "$PLANE" "$CODE" >/dev/null

echo "Waiting for your plane to confirm..."
i=0
while [ $i -lt 45 ]; do
  if docker logs "$HELPER" 2>&1 | grep -q "linked to the plane"; then
    echo "Connected. This computer now appears in Settings › Computers. You can close this window."
    exit 0
  fi
  if [ "$(docker inspect -f '{{.State.Running}}' "$HELPER" 2>/dev/null)" != "true" ]; then
    echo "The helper stopped:" >&2
    docker logs "$HELPER" 2>&1 | tail -n 5 >&2
    exit 1
  fi
  i=$((i + 1))
  sleep 2
done
echo "Still connecting. Check Settings › Computers; the helper keeps trying on its own."
