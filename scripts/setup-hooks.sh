#!/bin/sh
# Enable the repo's git hooks (pre-commit fmt check, pre-push CI checks).
# Run once after cloning: ./scripts/setup-hooks.sh
set -e
cd "$(dirname "$0")/.."
chmod +x .githooks/pre-commit .githooks/pre-push
git config core.hooksPath .githooks
echo "git hooks enabled (core.hooksPath = .githooks)"
