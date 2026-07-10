#!/usr/bin/env bash
# Patch server.json with release .mcpb SHA-256 for MCP Registry publish.
set -euo pipefail

VERSION="${1:?usage: $0 VERSION (e.g. 0.2.0)}"
REPO="${REPO:-rzlco666/mcp-sql-rust}"
ASSET="${ASSET:-mcp-sql-rust-x86_64-unknown-linux-gnu.mcpb}"
URL="https://github.com/${REPO}/releases/download/v${VERSION}/${ASSET}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SERVER_JSON="${ROOT}/server.json"
TMP="$(mktemp)"

cleanup() { rm -f "$TMP"; }
trap cleanup EXIT

if ! command -v jq >/dev/null 2>&1; then
  echo "jq is required" >&2
  exit 1
fi

curl -fsSL "$URL" -o "$TMP"
HASH="$(sha256sum "$TMP" | awk '{print $1}')"

jq \
  --arg version "$VERSION" \
  --arg url "$URL" \
  --arg hash "$HASH" \
  '.version = $version
   | .packages[0].identifier = $url
   | .packages[0].fileSha256 = $hash' \
  "$SERVER_JSON" > "${SERVER_JSON}.tmp"

mv "${SERVER_JSON}.tmp" "$SERVER_JSON"
echo "Updated server.json: $URL"
echo "fileSha256=$HASH"
