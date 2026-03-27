# k-stack

Content-addressed storage, DAG lineage, and structural schema analysis — exposed as MCP tools for any LLM or IDE.

## Why

Most dev work produces artifacts that get copy-pasted, lost between sessions, or buried in chat history. k-stack makes three structural guarantees:

- **Nothing stored twice.** Same content = same CID. Two agents writing the same conclusion get one blob, two provenance records. Storage dedup is free.
- **Context survives sessions.** Store reasoning once, retrieve by CID forever. No re-explaining architecture to a new chat window.
- **Lineage is automatic.** Every node records what it derived from and who wrote it. "Why did we do this?" is a graph query, not an archaeology project.

Schema drift, duplicate work across agents, and context loss between handoffs are structural problems. k-stack solves them structurally — not with conventions or discipline, but with content addressing.

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

In Claude Code (CLI or VS Code), type `/mcp`. You should see `k-stack` with 15 tools.

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

### Schema (canon.d)

| Tool | Input | Output |
|------|-------|--------|
| `ket_schema_store` | `name`, `version`, `fields[]` | `cid` |
| `ket_schema_list` | — | `schemas[]` |
| `ket_schema_validate` | `schema_cid`, `content` | `valid`, `errors?` |
| `ket_canonicalize` | `schema_cid`, `content` | `cid`, `canonical_bytes_hex` |
| `ket_schema_stats` | `schema_cid` | `total_nodes`, `unique_outputs`, `dedup_ratio` |

### Structure (canon.d)

| Tool | Input | Output |
|------|-------|--------|
| `ket_align` | `source_schema_cid`, `target_schema_cid`, `min_confidence?` | `candidates[]` with confidence + rationale |
| `ket_topology` | `kind?` | `clusters[]`, `convergent_clusters`, `co_occurrences[]` |

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

    Human: "Are these two schemas talking about the same thing?"

    Claude: [ket_align(source_schema_cid="02b1...", target_schema_cid="5690...")]
            -> subject -> subject_id (1.0), claim -> assessment (0.64)
            "Yes — observation.subject maps to review.subject_id with full
             confidence. The schemas describe the same entities differently."

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
- **Alignment**: Structural comparison of schemas — name similarity, type compatibility, identity alignment — produces field mapping candidates without ML or external services.
- **Topology**: Read-only analysis of what agents have built. Clusters nodes by schema + identity, finds convergence (multi-agent agreement), reports schema co-occurrence.
- **Secrets**: Refuses to store content matching API key, password, and PEM patterns.

## Addons

k-stack is the core. These tools build on top of it:

| Addon | What it does | Repo |
|-------|-------------|------|
| **catbus** | Multi-model context handoffs via immutable handoff packets. Pack context + artifacts into CAS, hand a CID to the next agent. | [catbus](https://github.com/nickjoven/catbus) |
| **ket-cli** | Full CLI with `put`, `get`, `dag`, `repair`, `scan`, `export/import`, Graphviz DOT output, and more. | [ket](https://github.com/nickjoven/ket) |

catbus is intentionally not in the core stack — it enforces a specific handoff workflow (pack/unpack with required summaries). k-stack stays unopinionated: content in, CID out.

## License

MIT
