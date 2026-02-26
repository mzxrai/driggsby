#!/usr/bin/env bash
set -euo pipefail

mode="full"
if [ "${1:-}" = "--mode" ]; then
  mode="${2:-full}"
fi

required_tools=(gitleaks cargo-deny)
if [ "${mode}" = "full" ]; then
  required_tools+=(trivy)
fi

missing_tools=()
for tool in "${required_tools[@]}"; do
  if ! command -v "${tool}" >/dev/null 2>&1; then
    missing_tools+=("${tool}")
  fi
done

if [ "${#missing_tools[@]}" -gt 0 ]; then
  echo "Missing required tools: ${missing_tools[*]}"
  echo "Run: just security-bootstrap-macos"
  exit 1
fi

echo "Running gitleaks working tree scan..."
gitleaks dir . --config .gitleaks.toml --no-banner --redact

echo "Running gitleaks history scan..."
gitleaks git . --config .gitleaks.toml --log-opts="--all --full-history --reflog" --no-banner --redact

echo "Running cargo-deny advisory scan..."
cargo deny check advisories

if [ "${mode}" = "full" ]; then
  echo "Running trivy filesystem scan..."
  trivy fs \
    --scanners vuln,secret,misconfig \
    --severity HIGH,CRITICAL \
    --exit-code 1 \
    --no-progress \
    --skip-dirs target \
    .
fi

echo "Local security scan passed."
