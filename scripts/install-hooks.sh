#!/usr/bin/env bash
# Install scripts/pre-push as the git pre-push hook.
set -euo pipefail
root="$(git rev-parse --show-toplevel)"
ln -sf ../../scripts/pre-push "$root/.git/hooks/pre-push"
chmod +x "$root/scripts/pre-push"
echo "installed pre-push hook"
