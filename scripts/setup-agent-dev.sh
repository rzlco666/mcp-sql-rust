#!/usr/bin/env bash
# Bootstrap local AI agent dev files (gitignored) from committed templates.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMPLATE_DIR="$ROOT/docs/templates/agent"

copy_template() {
  local src="$1"
  local dest="$2"
  if [[ -f "$dest" ]]; then
    echo "skip (exists): $dest"
  else
    cp "$src" "$dest"
    echo "installed: $dest"
  fi
}

mkdir -p "$ROOT/.cursor/rules" "$ROOT/.cursor/skills" "$ROOT/.opencode/agents"

copy_template "$TEMPLATE_DIR/AGENTS.md.template" "$ROOT/AGENTS.md"
copy_template "$TEMPLATE_DIR/OMNI_CURSOR_SETUP.md.template" "$ROOT/docs/OMNI_CURSOR_SETUP.md"
copy_template "$TEMPLATE_DIR/DEV_WORKFLOW.md.template" "$ROOT/docs/DEV_WORKFLOW.md"
copy_template "$TEMPLATE_DIR/opencode.json.template" "$ROOT/opencode.json"

echo ""
echo "Agent dev bootstrap complete."
echo "Optional: codegraph init && configure OMNI per docs/OMNI_CURSOR_SETUP.md"
