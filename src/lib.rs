use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, thiserror::Error)]
pub enum McpError {
    #[error("Unknown tool: {0}")]
    UnknownTool(String),
    #[error("Invalid params: {0}")]
    InvalidParams(String),
    #[error("CAS error: {0}")]
    Cas(#[from] ket_cas::CasError),
    #[error("DAG error: {0}")]
    Dag(#[from] ket_dag::DagError),
    #[error("Canon error: {0}")]
    Canon(#[from] canon_d::CanonError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
}

#[derive(Debug, Serialize)]
pub struct ToolDescriptor {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// Secret pattern check — refuse to store content matching common secret patterns.
fn contains_secret(content: &str) -> bool {
    let patterns = [
        "AKIA",           // AWS access key prefix
        "sk-",            // OpenAI/Stripe secret key prefix
        "ghp_",           // GitHub personal access token
        "gho_",           // GitHub OAuth token
        "-----BEGIN",     // PEM private key
        "password=",
        "PASSWORD=",
        "api_key=",
        "API_KEY=",
        "secret_key=",
        "SECRET_KEY=",
        "ANTHROPIC_API_KEY=",
    ];
    patterns.iter().any(|p| content.contains(p))
}

pub fn tool_descriptors() -> Vec<ToolDescriptor> {
    vec![
        ToolDescriptor {
            name: "ket_put".into(),
            description: "Store content in the content-addressed store. Returns a CID (BLAKE3 hash). Identical content always produces the same CID.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "Content to store" },
                    "kind": { "type": "string", "description": "Content kind label (e.g. 'note', 'config', 'doc')" }
                },
                "required": ["content", "kind"]
            }),
        },
        ToolDescriptor {
            name: "ket_get".into(),
            description: "Retrieve stored content by its CID. Returns the raw content and byte size.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "cid": { "type": "string", "description": "Content identifier (BLAKE3 hash)" }
                },
                "required": ["cid"]
            }),
        },
        ToolDescriptor {
            name: "ket_verify".into(),
            description: "Verify integrity of stored content by re-hashing and comparing to its CID.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "cid": { "type": "string", "description": "Content identifier to verify" }
                },
                "required": ["cid"]
            }),
        },
        ToolDescriptor {
            name: "ket_store".into(),
            description: "Store content and create a DAG node with provenance. Links content to parents, records agent and kind. Returns both the node CID and content CID.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "content": { "type": "string", "description": "Content to store" },
                    "kind": { "type": "string", "description": "Node kind: memory, code, reasoning, task, cdom, score, context" },
                    "parents": { "type": "array", "items": { "type": "string" }, "description": "Parent CIDs this derives from" },
                    "agent": { "type": "string", "description": "Agent name (e.g. 'claude', 'human', 'copilot')" }
                },
                "required": ["content", "kind", "parents", "agent"]
            }),
        },
        ToolDescriptor {
            name: "ket_lineage".into(),
            description: "Trace a node's ancestry by walking parent links up the DAG. Shows how knowledge was derived.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "cid": { "type": "string", "description": "Node CID to trace from" },
                    "max_depth": { "type": "integer", "description": "Maximum depth to traverse (default: unlimited)" }
                },
                "required": ["cid"]
            }),
        },
        ToolDescriptor {
            name: "ket_children".into(),
            description: "Find all DAG nodes that list the given CID as a parent. Shows what was derived from this node.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "cid": { "type": "string", "description": "Parent CID to find children of" }
                },
                "required": ["cid"]
            }),
        },
        ToolDescriptor {
            name: "ket_schema_list".into(),
            description: "List all schemas stored in the CAS. Returns schema name, version, and fields.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {}
            }),
        },
        ToolDescriptor {
            name: "ket_schema_validate".into(),
            description: "Validate JSON content against a stored schema. Returns whether the content conforms and any errors.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "schema_cid": { "type": "string", "description": "CID of the schema to validate against" },
                    "content": { "description": "JSON content to validate" }
                },
                "required": ["schema_cid", "content"]
            }),
        },
        ToolDescriptor {
            name: "ket_canonicalize".into(),
            description: "Canonicalize JSON content using a schema. Produces deterministic bytes and a CID. Same content always yields the same CID.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "schema_cid": { "type": "string", "description": "CID of the schema" },
                    "content": { "description": "JSON content to canonicalize" }
                },
                "required": ["schema_cid", "content"]
            }),
        },
        ToolDescriptor {
            name: "ket_align".into(),
            description: "Compare two schemas and find candidate field mappings. Uses structural alignment: name similarity (Levenshtein + substring bonus), type compatibility, and identity alignment. Returns candidates ranked by confidence.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "source_schema_cid": { "type": "string", "description": "CID of the source schema" },
                    "target_schema_cid": { "type": "string", "description": "CID of the target schema" },
                    "min_confidence": { "type": "number", "description": "Minimum confidence threshold 0.0-1.0 (default 0.3)" }
                },
                "required": ["source_schema_cid", "target_schema_cid"]
            }),
        },
        ToolDescriptor {
            name: "ket_topology".into(),
            description: "Analyze emergent structure in the DAG. Clusters nodes by schema + identity, finds convergence points (multi-agent agreement), and reports schema co-occurrence in lineage chains. Read-only — never mutates.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "description": "Optional: filter nodes by kind before analysis" }
                }
            }),
        },
        ToolDescriptor {
            name: "ket_schema_store".into(),
            description: "Store a schema definition in CAS. Define fields with name, kind (string/integer/float/bool/cid), required/optional, and identity flags. Returns the schema CID for use with validate, canonicalize, and align tools.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Schema name (e.g. 'observation', 'claim')" },
                    "version": { "type": "integer", "description": "Schema version number" },
                    "fields": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "name": { "type": "string" },
                                "kind": { "type": "string", "description": "string, integer, float, bool, cid" },
                                "required": { "type": "boolean", "description": "Default true" },
                                "identity": { "type": "boolean", "description": "Identity-bearing field. Default false" }
                            },
                            "required": ["name", "kind"]
                        },
                        "description": "Ordered field definitions"
                    }
                },
                "required": ["name", "version", "fields"]
            }),
        },
        ToolDescriptor {
            name: "ket_schema_stats".into(),
            description: "Check deduplication effectiveness for a schema. Returns total nodes tagged with the schema vs unique output CIDs. High ratio means the schema is working — identical observations hash identically.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "schema_cid": { "type": "string", "description": "Schema CID to check stats for" }
                },
                "required": ["schema_cid"]
            }),
        },
        ToolDescriptor {
            name: "ket_search".into(),
            description: "Full-text search across all CAS blobs. Finds content matching a query string.".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Text to search for (case-insensitive)" }
                },
                "required": ["query"]
            }),
        },
        ToolDescriptor {
            name: "ket_recent".into(),
            description: "List recent DAG nodes, optionally filtered by kind. Returns nodes sorted by timestamp (newest first).".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Maximum results (default 20)" },
                    "kind": { "type": "string", "description": "Filter by node kind" }
                }
            }),
        },
    ]
}

fn parse_field_kind(s: &str) -> Result<canon_d::FieldKind, McpError> {
    match s {
        "string" => Ok(canon_d::FieldKind::String),
        "integer" => Ok(canon_d::FieldKind::Integer),
        "float" => Ok(canon_d::FieldKind::Float),
        "bool" => Ok(canon_d::FieldKind::Bool),
        "cid" => Ok(canon_d::FieldKind::Cid),
        _ => Err(McpError::InvalidParams(format!("Unknown field kind: {s}. Use: string, integer, float, bool, cid"))),
    }
}

fn parse_node_kind(s: &str) -> Result<ket_dag::NodeKind, McpError> {
    match s {
        "memory" => Ok(ket_dag::NodeKind::Memory),
        "code" => Ok(ket_dag::NodeKind::Code),
        "reasoning" => Ok(ket_dag::NodeKind::Reasoning),
        "task" => Ok(ket_dag::NodeKind::Task),
        "cdom" => Ok(ket_dag::NodeKind::Cdom),
        "score" => Ok(ket_dag::NodeKind::Score),
        "context" => Ok(ket_dag::NodeKind::Context),
        _ => Err(McpError::InvalidParams(format!("Unknown node kind: {s}"))),
    }
}

pub fn handle_tool_call(
    tool_name: &str,
    params: &Value,
    cas: &ket_cas::Store,
) -> Result<Value, McpError> {
    match tool_name {
        "ket_put" => {
            let content = params["content"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("content required".into()))?;
            if contains_secret(content) {
                return Err(McpError::InvalidParams(
                    "Refused: content matches secret patterns (API keys, passwords, PEM keys)".into(),
                ));
            }
            let cid = cas.put(content.as_bytes())?;
            Ok(serde_json::json!({ "cid": cid.as_str() }))
        }

        "ket_get" => {
            let cid_str = params["cid"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("cid required".into()))?;
            let cid = ket_cas::Cid::from(cid_str);
            let data = cas.get(&cid)?;
            let size = data.len();
            let content = String::from_utf8_lossy(&data).into_owned();
            Ok(serde_json::json!({ "content": content, "size": size }))
        }

        "ket_verify" => {
            let cid_str = params["cid"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("cid required".into()))?;
            let cid = ket_cas::Cid::from(cid_str);
            let valid = cas.verify(&cid)?;
            Ok(serde_json::json!({ "valid": valid }))
        }

        "ket_store" => {
            let content = params["content"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("content required".into()))?;
            if contains_secret(content) {
                return Err(McpError::InvalidParams(
                    "Refused: content matches secret patterns".into(),
                ));
            }
            let kind_str = params["kind"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("kind required".into()))?;
            let agent = params["agent"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("agent required".into()))?;
            let parents: Vec<ket_cas::Cid> = params
                .get("parents")
                .and_then(|p| p.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(ket_cas::Cid::from))
                        .collect()
                })
                .unwrap_or_default();

            let kind = parse_node_kind(kind_str)?;
            let dag = ket_dag::Dag::new(cas);
            let (node_cid, content_cid) =
                dag.store_with_node(content.as_bytes(), kind, parents, agent)?;

            Ok(serde_json::json!({
                "node_cid": node_cid.as_str(),
                "content_cid": content_cid.as_str()
            }))
        }

        "ket_lineage" => {
            let cid_str = params["cid"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("cid required".into()))?;
            let max_depth = params.get("max_depth").and_then(|v| v.as_u64()).map(|d| d as u32);
            let dag = ket_dag::Dag::new(cas);
            let lineage = dag.lineage_bounded(&ket_cas::Cid::from(cid_str), max_depth)?;
            let chain: Vec<Value> = lineage
                .iter()
                .map(|(cid, node)| {
                    serde_json::json!({
                        "cid": cid.as_str(),
                        "kind": node.kind.to_string(),
                        "agent": node.agent,
                        "parents": node.parents.iter().map(|p| p.as_str()).collect::<Vec<_>>(),
                    })
                })
                .collect();
            Ok(serde_json::json!({ "chain": chain }))
        }

        "ket_children" => {
            let target_cid = params["cid"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("cid required".into()))?;
            let dag = ket_dag::Dag::new(cas);
            let all_cids = cas.list()?;
            let mut children = Vec::new();
            for cid in &all_cids {
                if let Ok(node) = dag.get_node(cid) {
                    if node.parents.iter().any(|p| p.as_str() == target_cid) {
                        children.push(serde_json::json!({
                            "cid": cid.as_str(),
                            "kind": node.kind.to_string(),
                            "agent": node.agent,
                        }));
                    }
                }
            }
            Ok(serde_json::json!({ "children": children }))
        }

        "ket_schema_list" => {
            let all_cids = cas.list()?;
            let mut schemas = Vec::new();
            for cid in &all_cids {
                if let Ok(data) = cas.get(cid) {
                    if let Ok(val) = serde_json::from_slice::<Value>(&data) {
                        // Detect schema objects by presence of "name", "version", "fields" keys
                        if val.get("name").is_some()
                            && val.get("version").is_some()
                            && val.get("fields").and_then(|f| f.as_array()).is_some()
                        {
                            let fields: Vec<Value> = val["fields"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|f| {
                                    serde_json::json!({
                                        "name": f["name"],
                                        "kind": f["kind"],
                                        "required": f["required"],
                                        "identity": f["identity"],
                                    })
                                })
                                .collect();
                            schemas.push(serde_json::json!({
                                "cid": cid.as_str(),
                                "name": val["name"],
                                "version": val["version"],
                                "fields": fields,
                            }));
                        }
                    }
                }
            }
            Ok(serde_json::json!({ "schemas": schemas }))
        }

        "ket_schema_validate" => {
            let schema_cid_str = params["schema_cid"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("schema_cid required".into()))?;
            let content = params
                .get("content")
                .ok_or_else(|| McpError::InvalidParams("content required".into()))?;

            // Load schema from CAS
            let schema_data = cas.get(&ket_cas::Cid::from(schema_cid_str))?;
            let schema: canon_d::Schema = serde_json::from_slice(&schema_data)
                .map_err(|e| McpError::InvalidParams(format!("Failed to parse schema: {e}")))?;

            let canon = canon_d::Canon::new(&schema);
            match canon.encode(content) {
                Ok(_) => Ok(serde_json::json!({ "valid": true })),
                Err(e) => Ok(serde_json::json!({
                    "valid": false,
                    "errors": [e.to_string()]
                })),
            }
        }

        "ket_canonicalize" => {
            let schema_cid_str = params["schema_cid"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("schema_cid required".into()))?;
            let content = params
                .get("content")
                .ok_or_else(|| McpError::InvalidParams("content required".into()))?;

            let schema_data = cas.get(&ket_cas::Cid::from(schema_cid_str))?;
            let schema: canon_d::Schema = serde_json::from_slice(&schema_data)
                .map_err(|e| McpError::InvalidParams(format!("Failed to parse schema: {e}")))?;

            let canon = canon_d::Canon::new(&schema);
            let canonical_bytes = canon.encode(content)?;
            let cid = ket_cas::hash_bytes(&canonical_bytes);
            let hex = canonical_bytes
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>();

            Ok(serde_json::json!({
                "cid": cid.as_str(),
                "canonical_bytes_hex": hex
            }))
        }

        "ket_align" => {
            let src_cid = params["source_schema_cid"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("source_schema_cid required".into()))?;
            let tgt_cid = params["target_schema_cid"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("target_schema_cid required".into()))?;
            let min_conf = params
                .get("min_confidence")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.3);

            let src_data = cas.get(&ket_cas::Cid::from(src_cid))?;
            let src_schema: canon_d::Schema = serde_json::from_slice(&src_data)
                .map_err(|e| McpError::InvalidParams(format!("Failed to parse source schema: {e}")))?;

            let tgt_data = cas.get(&ket_cas::Cid::from(tgt_cid))?;
            let tgt_schema: canon_d::Schema = serde_json::from_slice(&tgt_data)
                .map_err(|e| McpError::InvalidParams(format!("Failed to parse target schema: {e}")))?;

            let config = canon_d::AlignConfig {
                min_confidence: min_conf,
                ..canon_d::AlignConfig::default()
            };
            let candidates = canon_d::align(&src_schema, &tgt_schema, &config);

            let results: Vec<Value> = candidates
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "source_field": c.source_field,
                        "target_field": c.target_field,
                        "confidence": c.confidence,
                        "rationale": {
                            "name_score": c.rationale.name_score,
                            "type_score": c.rationale.type_score,
                            "identity_score": c.rationale.identity_score,
                        }
                    })
                })
                .collect();

            Ok(serde_json::json!({
                "source_schema": src_schema.name,
                "target_schema": tgt_schema.name,
                "candidates": results
            }))
        }

        "ket_topology" => {
            let kind_filter = params.get("kind").and_then(|v| v.as_str());
            let dag = ket_dag::Dag::new(cas);
            let all_cids = cas.list()?;

            let mut node_infos = Vec::new();
            for cid in &all_cids {
                if let Ok(node) = dag.get_node(cid) {
                    if let Some(filter) = kind_filter {
                        if node.kind.to_string() != filter {
                            continue;
                        }
                    }

                    // Compute identity hash if schema is present
                    let identity_hash = if let Some(ref schema_cid) = node.schema_cid {
                        if let Ok(schema_data) = cas.get(schema_cid) {
                            if let Ok(schema) = serde_json::from_slice::<canon_d::Schema>(&schema_data) {
                                if let Ok(content_data) = cas.get(&node.output_cid) {
                                    if let Ok(content_val) = serde_json::from_slice::<Value>(&content_data) {
                                        let canon = canon_d::Canon::new(&schema);
                                        canon.identity_projection(&content_val)
                                            .ok()
                                            .map(|bytes| ket_cas::hash_bytes(&bytes).as_str().to_string())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    } else {
                        None
                    };

                    // Collect ancestor schemas by walking parents (1 level)
                    let ancestor_schemas: Vec<String> = node
                        .parents
                        .iter()
                        .filter_map(|p| {
                            dag.get_node(p)
                                .ok()
                                .and_then(|n| n.schema_cid.map(|s| s.as_str().to_string()))
                        })
                        .collect();

                    node_infos.push(canon_d::NodeInfo {
                        node_cid: cid.as_str().to_string(),
                        schema_cid: node.schema_cid.as_ref().map(|s| s.as_str().to_string()),
                        identity_hash,
                        agent: node.agent.clone(),
                        ancestor_schemas,
                    });
                }
            }

            let topo = canon_d::TopologyView::from_nodes(&node_infos);

            let clusters: Vec<Value> = topo
                .clusters()
                .iter()
                .map(|c| {
                    serde_json::json!({
                        "schema_cid": c.schema_cid,
                        "identity_hash": c.identity_hash,
                        "node_count": c.node_cids.len(),
                        "agent_count": c.agent_count,
                        "node_cids": c.node_cids,
                    })
                })
                .collect();

            let convergent_count = topo.convergent_clusters().len();
            let co_occurrences: Vec<Value> = topo
                .schema_co_occurrences()
                .iter()
                .map(|(a, b, count)| {
                    serde_json::json!({
                        "schema_a": a,
                        "schema_b": b,
                        "count": count,
                    })
                })
                .collect();

            Ok(serde_json::json!({
                "schema_count": topo.schema_count(),
                "cluster_count": topo.cluster_count(),
                "convergent_clusters": convergent_count,
                "clusters": clusters,
                "co_occurrences": co_occurrences,
            }))
        }

        "ket_schema_store" => {
            let name = params["name"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("name required".into()))?;
            let version = params["version"]
                .as_u64()
                .ok_or_else(|| McpError::InvalidParams("version required".into()))? as u32;
            let fields_arr = params["fields"]
                .as_array()
                .ok_or_else(|| McpError::InvalidParams("fields array required".into()))?;

            let mut schema = canon_d::Schema::new(name, version);
            for field in fields_arr {
                let fname = field["name"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("field name required".into()))?;
                let fkind_str = field["kind"]
                    .as_str()
                    .ok_or_else(|| McpError::InvalidParams("field kind required".into()))?;
                let fkind = parse_field_kind(fkind_str)?;
                let is_identity = field.get("identity").and_then(|v| v.as_bool()).unwrap_or(false);
                let is_required = field.get("required").and_then(|v| v.as_bool()).unwrap_or(true);

                if is_identity {
                    schema = schema.identity(fname, fkind);
                } else if is_required {
                    schema = schema.required(fname, fkind);
                } else {
                    schema = schema.optional(fname, fkind);
                }
            }

            let schema_json = serde_json::to_vec(&schema)
                .map_err(|e| McpError::InvalidParams(format!("Failed to serialize schema: {e}")))?;
            let cid = cas.put(&schema_json)?;

            Ok(serde_json::json!({
                "cid": cid.as_str(),
                "name": name,
                "version": version,
                "field_count": fields_arr.len(),
            }))
        }

        "ket_schema_stats" => {
            let schema_cid = params["schema_cid"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("schema_cid required".into()))?;
            let dag = ket_dag::Dag::new(cas);
            let (total, unique) = dag.schema_stats(&ket_cas::Cid::from(schema_cid))?;
            let dedup_ratio = if unique > 0 {
                format!("{:.2}", total as f64 / unique as f64)
            } else {
                "N/A".to_string()
            };
            Ok(serde_json::json!({
                "total_nodes": total,
                "unique_outputs": unique,
                "dedup_ratio": dedup_ratio,
            }))
        }

        "ket_search" => {
            let query = params["query"]
                .as_str()
                .ok_or_else(|| McpError::InvalidParams("query required".into()))?;
            let query_lower = query.to_lowercase();
            let all_cids = cas.list()?;
            let mut matches = Vec::new();

            for cid in &all_cids {
                if matches.len() >= 50 {
                    break;
                }
                if let Ok(data) = cas.get(cid) {
                    if let Ok(text) = std::str::from_utf8(&data) {
                        if text.to_lowercase().contains(&query_lower) {
                            // Try to detect kind from DAG node
                            let dag = ket_dag::Dag::new(cas);
                            let kind = dag
                                .get_node(cid)
                                .ok()
                                .map(|n| n.kind.to_string())
                                .unwrap_or_default();
                            let snippet = if text.len() > 200 {
                                format!("{}...", &text[..200])
                            } else {
                                text.to_string()
                            };
                            matches.push(serde_json::json!({
                                "cid": cid.as_str(),
                                "snippet": snippet,
                                "kind": kind,
                            }));
                        }
                    }
                }
            }
            Ok(serde_json::json!({ "matches": matches }))
        }

        "ket_recent" => {
            let limit = params
                .get("limit")
                .and_then(|v| v.as_u64())
                .unwrap_or(20) as usize;
            let kind_filter = params.get("kind").and_then(|v| v.as_str());

            let dag = ket_dag::Dag::new(cas);
            let all_cids = cas.list()?;
            let mut nodes: Vec<(String, Value)> = Vec::new();

            for cid in &all_cids {
                if let Ok(node) = dag.get_node(cid) {
                    if let Some(filter) = kind_filter {
                        if node.kind.to_string() != filter {
                            continue;
                        }
                    }
                    let ts = node.timestamp.clone();
                    nodes.push((
                        ts.clone(),
                        serde_json::json!({
                            "cid": cid.as_str(),
                            "kind": node.kind.to_string(),
                            "agent": node.agent,
                            "timestamp": ts,
                        }),
                    ));
                }
            }

            // Sort by timestamp descending (newest first)
            nodes.sort_by(|a, b| b.0.cmp(&a.0));
            nodes.truncate(limit);
            let result: Vec<Value> = nodes.into_iter().map(|(_, v)| v).collect();
            Ok(serde_json::json!({ "nodes": result }))
        }

        _ => Err(McpError::UnknownTool(tool_name.to_string())),
    }
}

pub fn handle_jsonrpc(
    request: &JsonRpcRequest,
    cas: &ket_cas::Store,
) -> JsonRpcResponse {
    match request.method.as_str() {
        "initialize" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id.clone(),
            result: Some(serde_json::json!({
                "protocolVersion": "2024-11-05",
                "capabilities": { "tools": {} },
                "serverInfo": {
                    "name": "k-stack",
                    "version": "0.1.0"
                }
            })),
            error: None,
        },

        "tools/list" => {
            let tools = tool_descriptors();
            JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: request.id.clone(),
                result: Some(serde_json::json!({ "tools": tools })),
                error: None,
            }
        }

        "tools/call" => {
            let tool_name = request.params["name"].as_str().unwrap_or("");
            let arguments = request
                .params
                .get("arguments")
                .cloned()
                .unwrap_or(Value::Object(Default::default()));

            match handle_tool_call(tool_name, &arguments, cas) {
                Ok(result) => JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id: request.id.clone(),
                    result: Some(serde_json::json!({
                        "content": [{ "type": "text", "text": result.to_string() }]
                    })),
                    error: None,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id: request.id.clone(),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: e.to_string(),
                    }),
                },
            }
        }

        "notifications/initialized" => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id.clone(),
            result: Some(Value::Null),
            error: None,
        },

        _ => JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id: request.id.clone(),
            result: None,
            error: Some(JsonRpcError {
                code: -32601,
                message: format!("Method not found: {}", request.method),
            }),
        },
    }
}

pub fn run_stdio_server(cas: &ket_cas::Store) -> Result<(), McpError> {
    use std::io::{BufRead, BufReader, Write};

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    let reader = BufReader::new(stdin.lock());

    for line in reader.lines() {
        let line = line?;
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<JsonRpcRequest>(&line) {
            Ok(request) => handle_jsonrpc(&request, cas),
            Err(e) => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: None,
                result: None,
                error: Some(JsonRpcError {
                    code: -32700,
                    message: format!("Parse error: {e}"),
                }),
            },
        };

        let response_json = serde_json::to_string(&response)?;
        writeln!(stdout, "{response_json}").map_err(McpError::Io)?;
        stdout.flush().map_err(McpError::Io)?;
    }

    Ok(())
}
