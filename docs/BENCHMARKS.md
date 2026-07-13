# Benchmarks

Reproducible performance comparison of **mcp-sql-rust** against other MCP SQL servers on PostgreSQL.

## MySQL comparison (v0.5+)

Compare against `mcp-mysql-server` (local-mysql rival) on the same queries:

```bash
MYSQL_DATABASE_URL=mysql://demo:demo@127.0.0.1:3307/demo \
  ./scripts/benchmark/run-mysql-vs-mcp-mysql.sh
```

See also [`run-mysql.sh`](scripts/benchmark/run-mysql.sh) for the rival-only baseline.

## Summary (reference run)

Measured on **WSL2 Ubuntu 24.04** (`LAPTOP-OC7HTPEU`), `docker compose` seed DB, pinned versions in [`scripts/benchmark/versions.lock`](../scripts/benchmark/versions.lock).

| Server | Idle RSS (MB) | Load RSS (MB) | Cold start (ms) | p50 tool (ms) | tools/list (bytes) |
|--------|---------------|---------------|-----------------|---------------|-------------------|
| **mcp-sql-rust** | **10.4** | **12.8** | **13.5** | 0.88 | 2509 |
| server-postgres | 100.4 | 100.4 | 563.6 | 1.48 | 141 |
| postgres-mysql-mcp-server | 100.9 | 100.9 | 553.3 | 0.32 | 1969 |

Raw JSON: [`scripts/benchmark/benchmark-results.json`](../scripts/benchmark/benchmark-results.json) (2026-07-11).

> Re-run `./scripts/benchmark/run.sh` on your machine for authoritative numbers. Absolute latency includes local DB round-trip.

## Why these metrics

| Metric | Method | Rationale |
|--------|--------|-----------|
| **Idle RSS** | `/proc/<pid>/status` VmRSS, 2s after start | Memory at rest |
| **Load RSS** | Same after warmup + measured iterations | Steady-state under load |
| **Cold start** | Spawn → first `tools/list` response | First-use UX |
| **p50 tool latency** | `execute_sql` / `query` equivalent | Core MCP path |
| **tools/list bytes** | JSON serialized `tools/list` result | Tool-discovery token cost |

## Competitors

| Server | Package | Stack |
|--------|---------|-------|
| mcp-sql-rust | this repo | Rust |
| server-postgres | `@modelcontextprotocol/server-postgres@0.6.2` | Node/TS |
| postgres-mysql-mcp-server | `postgres-mysql-mcp-server@1.0.3` | Node/TS |

**Scope:** PostgreSQL only, same seed ([`docker/seed/postgres.sql`](../docker/seed/postgres.sql)), same query:

```sql
SELECT id, email, name FROM demo.users WHERE id = 42
```

`server-postgres` receives the DSN as a **CLI argument** (upstream requirement), not only `DATABASE_URL` env.

## Run locally

```bash
docker compose up -d
cargo build --release
./scripts/benchmark/run.sh
```

Requirements: Node.js 20+ (`npx`), Linux recommended for RSS.

### Harness files

| File | Role |
|------|------|
| [`mcp-client.mjs`](../scripts/benchmark/mcp-client.mjs) | NDJSON MCP stdio client |
| [`mcp-sql-rust.sh`](../scripts/benchmark/mcp-sql-rust.sh) | Single-server smoke |
| [`run.sh`](../scripts/benchmark/run.sh) | Full comparison orchestrator |
| [`versions.lock`](../scripts/benchmark/versions.lock) | Pinned rival versions |

## Interpreting results

- **RSS** — Rust binary + sqlx pool vs Node `npx` cold start footprint
- **Cold start** — includes rival package spawn; mcp-sql-rust is a native binary
- **p50** — dominated by Postgres latency on localhost; compare relative overhead cautiously
- **tools/list** — mcp-sql-rust lists richer schemas (3 tools with full descriptions); byte size is not the only token story

## Changelog

| Date | Change |
|------|--------|
| v0.4.0 | Initial harness + WSL2 reference numbers |

## MySQL localhost (v0.4.0 reference)

Manual HTTP benchmark on **WSL2**, MySQL 8.4 via `docker compose` (`localhost:3307`), seed [`docker/seed/mysql.sql`](../docker/seed/mysql.sql). Method: Streamable HTTP `execute_sql` / tool handlers, release binary.

| Metric | v0.1.0 | v0.4.0 |
|--------|--------|--------|
| Single SELECT avg | 14.5 ms | **6.5 ms** |
| P50 latency | — | 6.4 ms |
| P95 latency | — | 7.4 ms |
| P99 latency | — | 7.8 ms |
| Batch 4 queries | — | 8 ms |
| Batch 8 queries | — | 8 ms |
| Batch 16 queries | 31 ms | **10 ms** |
| 10× concurrent `SLEEP(0.2)` | — | 238 ms (≈8× speedup) |
| `describe_table` 80 cols | — | 11 ms |
| `search_objects` 200 tables | 109 ms | **13 ms** |
| Response 100 rows (`pegawai`) | — | 67 KB (~674 B/row) |

> These numbers measure end-to-end MCP + MySQL round-trip on one machine. They are **not** comparable 1:1 with the PostgreSQL rival harness above (different DB, transport path, and query mix).

Reproduce MySQL timings:

```bash
docker compose up -d mysql
cargo build --release
./scripts/benchmark/run-mysql.sh
```
