#!/usr/bin/env bash
# Install mcp-sql-rust from GitHub Releases.
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/rzlco666/mcp-sql-rust/main/install.sh | sh
#   curl -fsSL ... | sh -s -- --version 0.4.0 --prefix ~/.local
set -euo pipefail

REPO="${REPO:-rzlco666/mcp-sql-rust}"
PREFIX="${PREFIX:-${HOME}/.local}"
VERSION=""
VERIFY=1

usage() {
  cat <<'EOF'
Usage: install.sh [options]

Options:
  --version VER   Install specific release (default: latest)
  --prefix DIR    Install directory (default: ~/.local)
  --no-verify     Skip SHA256SUMS verification
  -h, --help      Show this help
EOF
}

while [ $# -gt 0 ]; do
  case "$1" in
    --version) VERSION="$2"; shift 2 ;;
    --prefix) PREFIX="$2"; shift 2 ;;
    --no-verify) VERIFY=0; shift ;;
    -h|--help) usage; exit 0 ;;
    *) echo "Unknown option: $1" >&2; usage >&2; exit 1 ;;
  esac
done

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux)  PLATFORM="unknown-linux-gnu" ;;
  Darwin) PLATFORM="apple-darwin" ;;
  MINGW*|MSYS*|CYGWIN*)
    echo "On Windows, use Scoop or winget. See docs/INSTALL.md" >&2
    exit 1
    ;;
  *)
    echo "Unsupported OS: $OS" >&2
    exit 1
    ;;
esac

case "$ARCH" in
  x86_64|amd64)  TARGET_ARCH="x86_64" ;;
  aarch64|arm64) TARGET_ARCH="aarch64" ;;
  *)
    echo "Unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

TARGET="${TARGET_ARCH}-${PLATFORM}"
ARTIFACT="mcp-sql-rust-${TARGET}.tar.gz"

if [ -z "$VERSION" ]; then
  VERSION="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" | sed -n 's/.*"tag_name": *"v\([^"]*\)".*/\1/p' | head -1)"
  if [ -z "$VERSION" ]; then
    echo "Failed to resolve latest release version" >&2
    exit 1
  fi
fi

BASE_URL="https://github.com/${REPO}/releases/download/v${VERSION}"
TMPDIR="$(mktemp -d)"
trap 'rm -rf "$TMPDIR"' EXIT

echo "Installing mcp-sql-rust v${VERSION} (${TARGET}) to ${PREFIX}/bin"

curl -fsSL "${BASE_URL}/${ARTIFACT}" -o "${TMPDIR}/${ARTIFACT}"

if [ "$VERIFY" = "1" ]; then
  curl -fsSL "${BASE_URL}/SHA256SUMS" -o "${TMPDIR}/SHA256SUMS"
  (
    cd "$TMPDIR"
    EXPECTED="$(grep " ${ARTIFACT}$" SHA256SUMS | awk '{print $1}')"
    ACTUAL="$(sha256sum "${ARTIFACT}" | awk '{print $1}')"
    if [ -z "$EXPECTED" ] || [ "$EXPECTED" != "$ACTUAL" ]; then
      echo "SHA256 verification failed for ${ARTIFACT}" >&2
      exit 1
    fi
  )
fi

tar -xzf "${TMPDIR}/${ARTIFACT}" -C "$TMPDIR"
mkdir -p "${PREFIX}/bin"
install -m 755 "${TMPDIR}/mcp-sql-rust" "${PREFIX}/bin/mcp-sql-rust"

if ! echo ":${PATH}:" | grep -q ":${PREFIX}/bin:"; then
  echo ""
  echo "Add to PATH:"
  echo "  export PATH=\"${PREFIX}/bin:\$PATH\""
fi

echo "Installed: $("${PREFIX}/bin/mcp-sql-rust" --version 2>/dev/null || echo "${PREFIX}/bin/mcp-sql-rust")"
