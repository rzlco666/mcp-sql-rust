# Installation

All install paths for **mcp-sql-rust**. Pick one channel; they install the same binary.

## Quick pick

| Channel | Best for |
|---------|----------|
| [curl install script](#curl-install-script) | Linux/macOS, fastest |
| [Homebrew](#homebrew) | macOS / Linux with brew |
| [Docker](#docker-ghcr) | Servers, CI, no local install |
| [GitHub Releases](#github-releases) | Manual download, air-gapped |
| [cargo-binstall](#cargo-binstall) | Rust developers |
| [Scoop / winget](#scoop-windows) | Windows |
| [Build from source](#build-from-source) | Contributors |

## curl install script

Auto-detects OS and architecture, verifies `SHA256SUMS`, installs to `~/.local/bin`:

```bash
curl -fsSL https://raw.githubusercontent.com/rzlco666/mcp-sql-rust/main/install.sh | sh
```

Options (pass after `sh -s --`):

```bash
curl -fsSL .../install.sh | sh -s -- --version 0.4.0 --prefix ~/.local
```

Add to `PATH` if needed:

```bash
export PATH="$HOME/.local/bin:$PATH"
```

## Homebrew

Tap repo: `github.com/rzlco666/homebrew-tap`

```bash
brew install rzlco666/tap/mcp-sql-rust
```

Formula source lives in this repo at [`packaging/homebrew/mcp-sql-rust.rb`](../packaging/homebrew/mcp-sql-rust.rb). After a release, maintainers run:

```bash
./scripts/bump-homebrew-formula.sh 0.4.0
# copy packaging/homebrew/mcp-sql-rust.rb → homebrew-tap/Formula/
```

## Docker (GHCR)

```bash
docker pull ghcr.io/rzlco666/mcp-sql-rust:latest
```

Run with stdio (for MCP clients that support docker):

```bash
docker run --rm -i \
  -e DATABASE_URL="postgresql://user:pass@host.docker.internal:5432/mydb" \
  ghcr.io/rzlco666/mcp-sql-rust:latest
```

HTTP mode:

```bash
docker run --rm -p 8080:8080 \
  -e DATABASE_URL="postgresql://..." \
  ghcr.io/rzlco666/mcp-sql-rust:latest \
  --http 0.0.0.0:8080
```

## GitHub Releases

Assets per release ([latest](https://github.com/rzlco666/mcp-sql-rust/releases/latest)):

| Platform | Archive |
|----------|---------|
| Linux x86_64 | `mcp-sql-rust-x86_64-unknown-linux-gnu.tar.gz` |
| Linux aarch64 | `mcp-sql-rust-aarch64-unknown-linux-gnu.tar.gz` |
| macOS x86_64 | `mcp-sql-rust-x86_64-apple-darwin.tar.gz` |
| macOS arm64 | `mcp-sql-rust-aarch64-apple-darwin.tar.gz` |
| Windows x86_64 | `mcp-sql-rust-x86_64-pc-windows-msvc.zip` |

Each archive contains a single binary (`mcp-sql-rust` or `mcp-sql-rust.exe`). MCP Registry bundles (`.mcpb`) are also published per platform.

### Verify checksum

```bash
curl -LO https://github.com/rzlco666/mcp-sql-rust/releases/download/v0.4.0/SHA256SUMS
curl -LO https://github.com/rzlco666/mcp-sql-rust/releases/download/v0.4.0/mcp-sql-rust-x86_64-unknown-linux-gnu.tar.gz
sha256sum -c SHA256SUMS --ignore-missing
```

### Manual install (Linux example)

```bash
tar -xzf mcp-sql-rust-x86_64-unknown-linux-gnu.tar.gz
sudo install -m 755 mcp-sql-rust /usr/local/bin/
```

## cargo-binstall

```bash
cargo binstall mcp-sql-rust
```

Metadata is in `Cargo.toml` under `[package.metadata.binstall]`. Pulls the matching `.tar.gz` or `.zip` from GitHub Releases.

## Scoop (Windows)

Manifest: [`packaging/scoop/mcp-sql-rust.json`](../packaging/scoop/mcp-sql-rust.json)

```bash
scoop bucket add rzlco666 https://github.com/rzlco666/scoop-bucket
scoop install rzlco666/mcp-sql-rust
```

Bump after release:

```bash
./scripts/bump-windows-manifests.sh 0.4.0
```

## winget (Windows)

Manifest: [`packaging/winget/rzlco666.mcp-sql-rust.yaml`](../packaging/winget/rzlco666.mcp-sql-rust.yaml)

```bash
winget install rzlco666.mcp-sql-rust
```

Submit updated manifest to [microsoft/winget-pkgs](https://github.com/microsoft/winget-pkgs) after each release.

## MCP Registry

Registry name: `io.github.rzlco666/mcp-sql-rust`

Clients that support the [MCP Registry](https://github.com/modelcontextprotocol/registry) can install the published `.mcpb` bundle. See [MCP_REGISTRY.md](MCP_REGISTRY.md).

## Build from source

Requires Rust 1.88+.

```bash
git clone https://github.com/rzlco666/mcp-sql-rust.git
cd mcp-sql-rust
cargo install --path .
```

Or:

```bash
cargo build --release
./target/release/mcp-sql-rust --help
```

## After install

1. Set `DATABASE_URL` (or use project `.env` — see [CONFIGURATION.md](CONFIGURATION.md))
2. Configure your MCP client — see [QUICKSTART.md](QUICKSTART.md)
3. Default mode is **read-only**; use `--allow-writes` / `--allow-ddl` only when needed ([SECURITY.md](SECURITY.md))
