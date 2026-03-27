# k-stack

Content-addressed storage, DAG lineage, and schema validation — exposed as MCP tools for any LLM or IDE.

## Install

### Prerequisites

- [Rust toolchain](https://rustup.rs/)
- [Claude Code](https://claude.com/claude-code) (CLI or VS Code extension)

### Build + register

```bash
git clone https://github.com/nickjoven/k-stack.git
cd k-stack
./scripts/install.sh
```

This builds the release binary and registers it with Claude Code. Works for both CLI and VS Code.

### Manual

```bash
cargo build --release
claude mcp add --transport stdio k-stack -- "$(pwd)/target/release/k-stack"
```

### Team setup (.mcp.json)

Commit a `.mcp.json` to your project root so everyone gets it:

```json
{
  "mcpServers": {
    "k-stack": {
      "command": "/absolute/path/to/k-stack/target/release/k-stack",
      "env": { "KET_HOME": ".ket" }
    }
  }
}
```

Or via CLI:

```bash
claude mcp add --transport stdio --scope project k-stack -- /path/to/target/release/k-stack
```

### Plugin

```bash
claude plugins add /path/to/k-stack
```

### Verify

In Claude Code (CLI or VS Code), type `/mcp`. You should see `k-stack` with 11 tools.

## Tools

### CAS

| Tool | Input | Output |
|------|-------|--------|
| `ket_put` | `content`, `kind` | `cid` |
| `ket_get` | `cid` | `content`, `size` |
| `ket_verify` | `cid` | `valid` |

### DAG

| Tool | Input | Output |
|------|-------|--------|
| `ket_store` | `content`, `kind`, `parents[]`, `agent` | `node_cid`, `content_cid` |
| `ket_lineage` | `cid`, `max_depth?` | `chain[]` |
| `ket_children` | `cid` | `children[]` |

### Schema

| Tool | Input | Output |
|------|-------|--------|
| `ket_schema_list` | — | `schemas[]` |
| `ket_schema_validate` | `schema_cid`, `content` | `valid`, `errors?` |
| `ket_canonicalize` | `schema_cid`, `content` | `cid`, `canonical_bytes_hex` |

### Query

| Tool | Input | Output |
|------|-------|--------|
| `ket_search` | `query` | `matches[]` |
| `ket_recent` | `limit?`, `kind?` | `nodes[]` |

## Example session

    Human: "Store this design decision and link it to the ticket"

    Claude: [ket_store(content="We chose X because Y",
             kind="reasoning", parents=[ticket_cid], agent="claude")]
            -> node_cid: "a3f8..."

    Human: "What led to this implementation?"

    Claude: [ket_lineage(cid="a3f8...", max_depth=5)]
            -> design decision <- ticket <- sprint goal <- quarterly OKR
            "This traces back through the ticket to the Q2 OKR
             for reducing API latency."

    Human: "Save a snapshot of this conversation context"

    Claude: [ket_store(kind="context", content=<summary>,
             parents=[recent_cids], agent="claude")]
            -> context persisted, recoverable by CID in any future session

## Configuration

**KET_HOME** controls where the CAS store lives. Defaults to `.ket` (relative to working directory).

    KET_HOME=.ket          # per-project store (default)
    KET_HOME=/home/me/.ket # shared global store

The store auto-initializes on first use. Each project can have its own store, or share one via an absolute path.

## How it works

Content in, CID out, parents link the story.

- **CAS**: BLAKE3 hash of content = CID. Same content = same CID, always. Automatic dedup.
- **DAG**: Nodes record what was produced, what it derived from, who produced it, and when.
- **Schema**: canon.d canonicalizes JSON deterministically. Validates structure, enables drift detection.
- **Secrets**: Refuses to store content matching API key, password, and PEM patterns.

## License

MIT
