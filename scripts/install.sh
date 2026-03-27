#!/usr/bin/env bash
set -euo pipefail

# Build k-stack and configure it for Claude Code (CLI + VS Code)

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"

echo "Building k-stack..."
cargo build --release --manifest-path "$PROJECT_ROOT/Cargo.toml"

BINARY="$PROJECT_ROOT/target/release/k-stack"

if [ ! -f "$BINARY" ]; then
    echo "Error: build failed, binary not found at $BINARY"
    exit 1
fi

echo "Binary: $BINARY"
echo ""

# Detect if claude CLI is available
if command -v claude &>/dev/null; then
    echo "Registering with Claude Code..."
    claude mcp add --transport stdio k-stack -- "$BINARY"
    echo "Done. Use /mcp in Claude Code to verify."
else
    echo "Claude CLI not found. Add manually to .mcp.json:"
    echo ""
    echo '{'
    echo '  "mcpServers": {'
    echo '    "k-stack": {'
    echo "      \"command\": \"$BINARY\","
    echo '      "env": { "KET_HOME": ".ket" }'
    echo '    }'
    echo '  }'
    echo '}'
fi
