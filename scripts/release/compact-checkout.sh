#!/usr/bin/env bash
# Keep a self-hosted runner's persistent checkout small. actions/checkout fetches each job's commit
# into the same repository as a new pack of about 97 MB and never removes the old ones: the Core
# runner's .git reached 156 packs and 15 GB by 9 October 2026, against 94 MB for a fresh shallow
# clone. Only the checked-out commit is needed, so everything else goes. It takes about a second.
set -euo pipefail
git reflog expire --expire=now --all
git -c gc.auto=0 gc --prune=now --quiet
