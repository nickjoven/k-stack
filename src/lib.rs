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
