#!/usr/bin/env bash
# Update Scoop and winget manifests from release SHA256SUMS.
set -euo pipefail

VERSION="${1:?usage: $0 VERSION (e.g. 0.4.0)}"
REPO="${REPO:-rzlco666/mcp-sql-rust}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SUMS_URL="https://github.com/${REPO}/releases/download/v${VERSION}/SHA256SUMS"
ARTIFACT="mcp-sql-rust-x86_64-pc-windows-msvc.zip"
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

curl -fsSL "$SUMS_URL" -o "$TMP"
HASH="$(grep " ${ARTIFACT}$" "$TMP" | awk '{print $1}')"
if [ -z "$HASH" ]; then
  echo "Hash not found for ${ARTIFACT}" >&2
  exit 1
fi

SCOOP="${ROOT}/packaging/scoop/mcp-sql-rust.json"
WINGET="${ROOT}/packaging/winget/rzlco666.mcp-sql-rust.yaml"

sed -i \
  -e "s/\"version\": \"[^\"]*\"/\"version\": \"${VERSION}\"/" \
  -e "s|releases/download/v[^/]*/|releases/download/v${VERSION}/|g" \
  -e "s/REPLACE_ME_WINDOWS_ZIP_SHA256/${HASH}/" \
  "$SCOOP" "$WINGET"

echo "Updated Scoop and winget manifests for v${VERSION}"
echo "  ${SCOOP}"
echo "  ${WINGET}"
