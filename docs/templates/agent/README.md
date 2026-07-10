# Agent dev templates

These files bootstrap **local-only** AI agent tooling. They are copied by [`scripts/setup-agent-dev.sh`](../../scripts/setup-agent-dev.sh) into gitignored paths:

| Template | Installed to |
|----------|----------------|
| `AGENTS.md.template` | `AGENTS.md` |
| `OMNI_CURSOR_SETUP.md.template` | `docs/OMNI_CURSOR_SETUP.md` |
| `DEV_WORKFLOW.md.template` | `docs/DEV_WORKFLOW.md` |
| `opencode.json.template` | `opencode.json` |

Cursor rules/skills (`.cursor/`) are maintained locally and are not published.
