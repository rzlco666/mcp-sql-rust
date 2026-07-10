# MCP Registry — mcp-sql-rust

This project publishes to the [MCP Registry](https://registry.modelcontextprotocol.io) using **MCPB** bundles attached to [GitHub Releases](https://github.com/rzlco666/mcp-sql-rust/releases).

Registry name (discoverability convention):

```text
mcp-name: io.github.rzlco666/mcp-sql-rust
```

## Artifacts

| File | Purpose |
|------|---------|
| [`server.json`](../server.json) | Registry manifest (`registryType: mcpb`) |
| [`mcpb/manifest.json`](../mcpb/manifest.json) | MCPB bundle template (binary copied at pack time) |
| Release `.mcpb` assets | Platform bundles built in CI |
| [`.github/workflows/registry-publish.yml`](../.github/workflows/registry-publish.yml) | Auto-publish on GitHub Release |

Crates.io is **not** used (`publish = false` in `Cargo.toml`).

## Automated publish (recommended)

On each GitHub Release (`release: published`), CI runs `registry-publish.yml`:

1. Downloads `mcp-publisher`
2. Syncs `server.json` version from the release tag
3. `mcp-publisher login github-oidc`
4. `mcp-publisher publish`

Requires repository **Actions** permission for OIDC (`id-token: write` — already set in workflow).

## Manual publish (first time or fallback)

1. Install CLI:

```bash
curl -fsSL "https://github.com/modelcontextprotocol/registry/releases/latest/download/mcp-publisher_$(uname -s | tr '[:upper:]' '[:lower:]')_$(uname -m | sed 's/x86_64/amd64/;s/aarch64/arm64/').tar.gz" | tar xz mcp-publisher
mv mcp-publisher ~/.local/bin/
```

2. After release, patch hashes (Linux amd64 minimum):

```bash
./scripts/update-server-json-sha.sh 0.3.0
```

3. Authenticate and publish (interactive GitHub device flow):

```bash
mcp-publisher login github
mcp-publisher validate server.json
mcp-publisher publish
```

Must authenticate as the GitHub user that owns namespace `io.github.rzlco666/`.

4. Verify:

```bash
curl -s "https://registry.modelcontextprotocol.io/v0/servers?search=io.github.rzlco666/mcp-sql-rust"
```

## Cursor install (direct binary)

```json
{
  "mcpServers": {
    "sql": {
      "command": "mcp-sql-rust",
      "args": []
    }
  }
}
```

Set `DATABASE_URL` in the environment or project `.env`.
