#!/usr/bin/env node
/**
 * Official MCP launcher for Cursor — loads workspace .env before spawning mcp-sql-rust.
 *
 * ~/.cursor/mcp.json:
 *   "args": ["/path/to/mcp-sql-rust/packaging/cursor-mcp-launcher.mjs", "${workspaceFolder}"]
 */
import { spawn } from 'child_process';
import { createConnection } from 'net';
import { existsSync, readFileSync } from 'fs';
import { dirname, join, resolve } from 'path';

const MCP_BIN =
  process.env.MCP_SQL_RUST_BIN ||
  process.env.MCP_SQL_BIN ||
  'mcp-sql-rust';

const DEFAULT_ARGS = ['--allow-writes', '--full-tools'];
const MCP_ARGS = process.env.MCP_SQL_ARGS
  ? process.env.MCP_SQL_ARGS.split(/\s+/).filter(Boolean)
  : DEFAULT_ARGS;

const PREFLIGHT_TIMEOUT_MS = 5000;

function parseDotenv(text) {
  const vars = {};
  for (const rawLine of text.split('\n')) {
    const line = rawLine.trim();
    if (!line || line.startsWith('#')) continue;
    const eq = line.indexOf('=');
    if (eq <= 0) continue;
    const key = line.slice(0, eq).trim();
    let value = line.slice(eq + 1).trim();
    if (
      (value.startsWith('"') && value.endsWith('"')) ||
      (value.startsWith("'") && value.endsWith("'"))
    ) {
      value = value.slice(1, -1);
    }
    vars[key] = value;
  }
  return vars;
}

function findDotenv(startDir) {
  let dir = resolve(startDir);
  while (true) {
    const envPath = join(dir, '.env');
    if (existsSync(envPath)) {
      return { envPath, vars: parseDotenv(readFileSync(envPath, 'utf8')) };
    }
    const parent = dirname(dir);
    if (parent === dir) break;
    dir = parent;
  }
  return null;
}

function hasDbCredentials(vars) {
  if (vars.DATABASE_URL) return true;
  if (vars.POSTGRES_URL || vars.MYSQL_URL || vars.SQLITE_URL) return true;
  if (vars.MYSQL_HOST && vars.MYSQL_USER && (vars.MYSQL_DATABASE || vars.MYSQL_DB)) {
    return true;
  }
  if (vars.POSTGRES_HOST && vars.POSTGRES_USER && vars.POSTGRES_DB) return true;
  return false;
}

function resolveDatabaseUrl(vars) {
  if (vars.DATABASE_URL) return vars.DATABASE_URL;
  if (vars.MYSQL_URL) return vars.MYSQL_URL;
  if (vars.POSTGRES_URL) return vars.POSTGRES_URL;
  if (vars.SQLITE_URL) return vars.SQLITE_URL;
  if (vars.MYSQL_HOST && vars.MYSQL_USER) {
    const host = vars.MYSQL_HOST;
    const user = vars.MYSQL_USER;
    const password = vars.MYSQL_PASSWORD || '';
    const db = vars.MYSQL_DATABASE || vars.MYSQL_DB || '';
    const port = vars.MYSQL_PORT || '3306';
    return `mysql://${user}:${password}@${host}:${port}/${db}`;
  }
  if (vars.POSTGRES_HOST && vars.POSTGRES_USER && vars.POSTGRES_DB) {
    const host = vars.POSTGRES_HOST;
    const user = vars.POSTGRES_USER;
    const password = vars.POSTGRES_PASSWORD || '';
    const db = vars.POSTGRES_DB;
    const port = vars.POSTGRES_PORT || '5432';
    return `postgresql://${user}:${password}@${host}:${port}/${db}`;
  }
  return null;
}

function parseHostPort(url) {
  if (!url) return null;
  const lower = url.toLowerCase();
  let rest = url;
  if (lower.startsWith('mysql://')) rest = url.slice('mysql://'.length);
  else if (lower.startsWith('postgresql://')) rest = url.slice('postgresql://'.length);
  else if (lower.startsWith('postgres://')) rest = url.slice('postgres://'.length);
  else return null;

  const at = rest.lastIndexOf('@');
  const hostPart = at >= 0 ? rest.slice(at + 1) : rest;
  const slash = hostPart.indexOf('/');
  const hostPort = slash >= 0 ? hostPart.slice(0, slash) : hostPart;
  const [host, port] = hostPort.includes(':')
    ? hostPort.split(':')
    : [hostPort, lower.startsWith('mysql') ? '3306' : '5432'];
  if (!host) return null;
  return { host, port: Number(port) };
}

function tcpPreflight(host, port) {
  return new Promise((resolvePromise) => {
    const socket = createConnection({ host, port });
    const timer = setTimeout(() => {
      socket.destroy();
      resolvePromise(false);
    }, PREFLIGHT_TIMEOUT_MS);
    socket.on('connect', () => {
      clearTimeout(timer);
      socket.end();
      resolvePromise(true);
    });
    socket.on('error', () => {
      clearTimeout(timer);
      resolvePromise(false);
    });
  });
}

const workspaceArg = process.argv[2];
const workspace =
  workspaceArg && workspaceArg !== '${workspaceFolder}'
    ? resolve(workspaceArg)
    : resolve(process.cwd());

const found = findDotenv(workspace);
if (!found || !hasDbCredentials(found.vars)) {
  console.error(
    `mcp-sql-rust launcher: no DATABASE_URL (or MYSQL_*/POSTGRES_* parts) in .env under ${workspace}`,
  );
  process.exit(1);
}

const dbUrl = resolveDatabaseUrl(found.vars);
const endpoint = parseHostPort(dbUrl);
if (endpoint) {
  const ok = await tcpPreflight(endpoint.host, endpoint.port);
  if (!ok) {
    console.error(
      `mcp-sql-rust launcher: cannot reach ${endpoint.host}:${endpoint.port} — start the database or fix .env in ${workspace}`,
    );
    process.exit(1);
  }
}

const rustArgs = [...MCP_ARGS, '--workspace', workspace];
const child = spawn(MCP_BIN, rustArgs, {
  stdio: 'inherit',
  env: { ...process.env, ...found.vars },
  cwd: workspace,
});

child.on('exit', (code, signal) => {
  if (signal) process.kill(process.pid, signal);
  process.exit(code ?? 0);
});
