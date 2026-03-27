# k-stack Setup Guide

Content-addressed storage, DAG lineage, and schema validation as MCP tools for Claude Code.

## Quick Start

### Option A: One-line install (CLI + VS Code)

```bash
./scripts/install.sh
```

This builds the binary and registers it with Claude Code. Works for both the CLI and VS Code extension.

### Option B: Manual setup

1. Build:
   ```bash
   cargo build --release
   ```

2. Register with Claude Code:
   ```bash
   claude mcp add --transport stdio k-stack -- /path/to/k-stack/target/release/k-stack
   ```

3. Verify in Claude Code:
   ```
   /mcp
   ```

### Option C: Project-scoped (.mcp.json)

Add to your project's `.mcp.json` so your whole team gets it:

```bash
claude mcp add --transport stdio --scope project k-stack -- /path/to/k-stack/target/release/k-stack
```

Or create `.mcp.json` manually:

```json
{
  "mcpServers": {
    "k-stack": {
      "command": "/absolute/path/to/k-stack/target/release/k-stack",
      "env": {
        "KET_HOME": ".ket"
      }
    }
  }
}
```

### Option D: Claude Code plugin

Clone this repo and install as a plugin:

```bash
claude plugins add /path/to/k-stack
```

The plugin auto-starts the MCP server when enabled. Use `/plugins` to manage.

## VS Code Specifics

The VS Code Claude Code extension picks up MCP servers from the same config as the CLI. After registering via any method above:

1. Open VS Code with the Claude Code extension
2. Open the Claude Code chat panel
3. Type `/mcp` to see k-stack listed with its 11 tools
4. Claude can now use `ket_put`, `ket_store`, `ket_lineage`, etc.

### Verify tools are available

Ask Claude: "What k-stack tools do you have access to?"

It should list all 11 tools: `ket_put`, `ket_get`, `ket_verify`, `ket_store`, `ket_lineage`, `ket_children`, `ket_schema_list`, `ket_schema_validate`, `ket_canonicalize`, `ket_search`, `ket_recent`.

## Configuration

### KET_HOME

The CAS store location. Defaults to `.ket` (relative to working directory).

```bash
# Per-project store (default)
KET_HOME=.ket

# Shared store
KET_HOME=/home/user/.ket

# Set via env in .mcp.json
"env": { "KET_HOME": "/path/to/.ket" }
```

The store auto-initializes on first use.

### Multiple projects

Each project can have its own CAS by using the default `.ket` relative path. Or share a global store by setting `KET_HOME` to an absolute path.

## Tools Reference

### CAS (Content-Addressed Storage)
- **ket_put** — Store content, get CID. Identical content = same CID.
- **ket_get** — Retrieve content by CID. Returns content + byte size.
- **ket_verify** — Re-hash stored content and confirm integrity.

### DAG (Directed Acyclic Graph)
- **ket_store** — Store content + create a DAG node linking to parents. Records agent and kind.
- **ket_lineage** — Walk parent chain. See how knowledge was derived.
- **ket_children** — Find everything derived from a node.

### Schema (canon.d)
- **ket_schema_list** — Discover schemas in the store.
- **ket_schema_validate** — Check if content conforms to a schema.
- **ket_canonicalize** — Deterministic encoding. Same content + schema = same CID always.

### Query
- **ket_search** — Full-text search across all stored content.
- **ket_recent** — Recent DAG nodes sorted by timestamp, filterable by kind.

## Example Session

```
Human: "Store this design decision and link it to the ticket"

Claude: [calls ket_store(content="We chose X because Y",
         kind="reasoning", parents=[ticket_cid], agent="claude")]
        → node_cid: "a3f8..."