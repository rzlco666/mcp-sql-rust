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

Crates.io is **not** used (`publish = false` in `Cargo.toml`).

## Maintainer: publish a release

1. Tag `v0.2.0` (or newer) — CI builds raw binaries and `.mcpb` files.
2. After the release is live, patch `server.json` with the Linux amd64 asset hash:

```bash
./scripts/update-server-json-sha.sh 0.2.0
```

3. Install [mcp-publisher](https://github.com/modelcontextprotocol/registry) and publish:

```bash
mcp-publisher login github
mcp-publisher validate server.json
mcp-publisher publish
```

Commit the updated `server.json` `fileSha256` before or after `publish` so the repo matches the registry entry.

## Optional automation

A future `registry-publish.yml` workflow can run `mcp-publisher publish` on release using a `MCP_REGISTRY_TOKEN` secret. Manual publish is fine for the first registry entry.

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
