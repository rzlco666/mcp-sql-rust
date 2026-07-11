#!/usr/bin/env bash
# Update packaging/homebrew/mcp-sql-rust.rb version and sha256 from release SHA256SUMS.
set -euo pipefail

VERSION="${1:?usage: $0 VERSION (e.g. 0.4.0)}"
REPO="${REPO:-rzlco666/mcp-sql-rust}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FORMULA="${ROOT}/packaging/homebrew/mcp-sql-rust.rb"
SUMS_URL="https://github.com/${REPO}/releases/download/v${VERSION}/SHA256SUMS"
TMP="$(mktemp)"
trap 'rm -f "$TMP"' EXIT

curl -fsSL "$SUMS_URL" -o "$TMP"

hash_for() {
  local artifact="$1"
  grep " ${artifact}$" "$TMP" | awk '{print $1}'
}

AARCH64_APPLE="$(hash_for "mcp-sql-rust-aarch64-apple-darwin.tar.gz")"
X86_64_APPLE="$(hash_for "mcp-sql-rust-x86_64-apple-darwin.tar.gz")"
AARCH64_LINUX="$(hash_for "mcp-sql-rust-aarch64-unknown-linux-gnu.tar.gz")"
X86_64_LINUX="$(hash_for "mcp-sql-rust-x86_64-unknown-linux-gnu.tar.gz")"

for h in "$AARCH64_APPLE" "$X86_64_APPLE" "$AARCH64_LINUX" "$X86_64_LINUX"; do
  if [ -z "$h" ]; then
    echo "Missing hash in SHA256SUMS" >&2
    exit 1
  fi
done

sed -i \
  -e "s/version \"[^\"]*\"/version \"${VERSION}\"/" \
  -e "s|releases/download/v[^/]*/|releases/download/v${VERSION}/|g" \
  -e "s/REPLACE_ME_AARCH64_APPLE_DARWIN/${AARCH64_APPLE}/" \
  -e "s/REPLACE_ME_X86_64_APPLE_DARWIN/${X86_64_APPLE}/" \
  -e "s/REPLACE_ME_AARCH64_LINUX/${AARCH64_LINUX}/" \
  -e "s/REPLACE_ME_X86_64_LINUX/${X86_64_LINUX}/" \
  "$FORMULA"

echo "Updated ${FORMULA} for v${VERSION}"
echo "Copy to homebrew-tap: Formula/mcp-sql-rust.rb"
