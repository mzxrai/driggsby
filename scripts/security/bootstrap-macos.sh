#!/usr/bin/env bash
set -euo pipefail

if ! command -v brew >/dev/null 2>&1; then
  echo "Homebrew is required. Install from https://brew.sh and rerun."
  exit 1
fi

brew install gitleaks cargo-deny trivy

git config core.hooksPath .githooks
chmod +x .githooks/pre-commit .githooks/pre-push

echo "Security bootstrap complete."
echo "Installed: gitleaks, cargo-deny, trivy"
echo "Configured: git hooks path -> .githooks"
