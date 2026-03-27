use serde_json::{json, Value};
use std::io::Write;
use std::process::{Command, Stdio};
use tempfile::TempDir;

/// Send JSON-RPC requests to k-stack and collect responses.
fn run_mcp(ket_home: &str, requests: &[Value]) -> Vec<Value> {
    let binary = env!("CARGO_BIN_EXE_k-stack");
    let mut input = String::new();
    for req in requests {
        input.push_str(&req.to_string());
        input.push('\n');
    }

    let mut child = Command::new(binary)
        .env("KET_HOME", ket_home)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("failed to start k-stack");

    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();

    let output = child.wait_with_output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();

    stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).expect("invalid JSON response"))
        .collect()
}

/// Extract the tool result text from an MCP response and parse it as JSON.
fn tool_result(response: &Value) -> Value {
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .expect("missing tool result text");
    serde_json::from_str(text).expect("tool result is not valid JSON")
}

#[test]
fn initialize() {
    let tmp = TempDir::new().unwrap();
    let responses = run_mcp(
        tmp.path().to_str().unwrap(),
        &[json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})],
    );
    assert_eq!(responses[0]["result"]["serverInfo"]["name"], "k-stack");
    assert_eq!(responses[0]["result"]["protocolVersion"], "2024-11-05");
}

#[test]
fn tools_list_count() {
    let tmp = TempDir::new().unwrap();
    let responses = run_mcp(
        tmp.path().to_str().unwrap(),
        &[json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}})],
    );
    let tools = responses[0]["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.len(), 15);
}

#[test]
fn put_get_verify_roundtrip() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_str().unwrap();

    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_put", "arguments": {"content": "hello world", "kind": "test"}
            }}),
        ],
    );

    let put_result = tool_result(&responses[1]);
    let cid = put_result["cid"].as_str().unwrap();
    assert_eq!(cid.len(), 64);

    // get + verify in second invocation
    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_get", "arguments": {"cid": cid}
            }}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
                "name": "ket_verify", "arguments": {"cid": cid}
            }}),
        ],
    );

    let get_result = tool_result(&responses[1]);
    assert_eq!(get_result["content"], "hello world");
    assert_eq!(get_result["size"], 11);

    let verify_result = tool_result(&responses[2]);
    assert_eq!(verify_result["valid"], true);
}

#[test]
fn dedup_same_content_same_cid() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_str().unwrap();

    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_put", "arguments": {"content": "duplicate me", "kind": "a"}
            }}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
                "name": "ket_put", "arguments": {"content": "duplicate me", "kind": "b"}
            }}),
        ],
    );

    let cid1 = tool_result(&responses[1])["cid"].as_str().unwrap().to_string();
    let cid2 = tool_result(&responses[2])["cid"].as_str().unwrap().to_string();
    assert_eq!(cid1, cid2, "same content must produce same CID");
}

#[test]
fn store_lineage_children() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_str().unwrap();

    // Create parent node
    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_store", "arguments": {
                    "content": "parent decision", "kind": "reasoning",
                    "parents": [], "agent": "human"
                }
            }}),
        ],
    );
    let parent_cid = tool_result(&responses[1])["node_cid"]
        .as_str().unwrap().to_string();

    // Create child linked to parent
    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_store", "arguments": {
                    "content": "child implementation", "kind": "code",
                    "parents": [parent_cid], "agent": "claude"
                }
            }}),
        ],
    );
    let child_cid = tool_result(&responses[1])["node_cid"]
        .as_str().unwrap().to_string();

    // Trace lineage from child
    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_lineage", "arguments": {"cid": child_cid}
            }}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
                "name": "ket_children", "arguments": {"cid": parent_cid}
            }}),
        ],
    );

    let lineage = tool_result(&responses[1]);
    let chain = lineage["chain"].as_array().unwrap();
    assert!(chain.len() >= 2, "lineage should include child + parent");

    let children = tool_result(&responses[2]);
    let kids = children["children"].as_array().unwrap();
    assert_eq!(kids.len(), 1);
    assert_eq!(kids[0]["agent"], "claude");
}

#[test]
fn secret_rejection() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_str().unwrap();

    let secrets = vec![
        "sk-abc123secretkey",
        "AKIA1234567890EXAMPLE",
        "password=hunter2",
        "-----BEGIN RSA PRIVATE KEY-----",
    ];

    for secret in secrets {
        let responses = run_mcp(
            home,
            &[
                json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
                json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                    "name": "ket_put", "arguments": {"content": secret, "kind": "test"}
                }}),
            ],
        );
        assert!(
            responses[1]["error"].is_object(),
            "should reject secret pattern: {secret}"
        );
    }
}

#[test]
fn schema_store_validate_canonicalize() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_str().unwrap();

    // Store a schema
    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_schema_store", "arguments": {
                    "name": "note", "version": 1,
                    "fields": [
                        {"name": "title", "kind": "string", "identity": true},
                        {"name": "body", "kind": "string"}
                    ]
                }
            }}),
        ],
    );
    let schema_cid = tool_result(&responses[1])["cid"]
        .as_str().unwrap().to_string();

    // Validate good content
    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_schema_validate", "arguments": {
                    "schema_cid": schema_cid,
                    "content": {"title": "test", "body": "hello"}
                }
            }}),
            // Validate bad content (missing required field)
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
                "name": "ket_schema_validate", "arguments": {
                    "schema_cid": schema_cid,
                    "content": {"title": "test"}
                }
            }}),
            // Canonicalize
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {
                "name": "ket_canonicalize", "arguments": {
                    "schema_cid": schema_cid,
                    "content": {"body": "hello", "title": "test"}
                }
            }}),
        ],
    );

    let valid = tool_result(&responses[1]);
    assert_eq!(valid["valid"], true);

    let invalid = tool_result(&responses[2]);
    assert_eq!(invalid["valid"], false);

    let canonical = tool_result(&responses[3]);
    assert!(canonical["cid"].as_str().unwrap().len() == 64);
    assert!(!canonical["canonical_bytes_hex"].as_str().unwrap().is_empty());
}

#[test]
fn schema_list_discovers_stored_schemas() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_str().unwrap();

    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_schema_store", "arguments": {
                    "name": "alpha", "version": 1,
                    "fields": [{"name": "x", "kind": "string"}]
                }
            }}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
                "name": "ket_schema_store", "arguments": {
                    "name": "beta", "version": 1,
                    "fields": [{"name": "y", "kind": "integer"}]
                }
            }}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {
                "name": "ket_schema_list", "arguments": {}
            }}),
        ],
    );

    let schemas = tool_result(&responses[3]);
    let list = schemas["schemas"].as_array().unwrap();
    assert_eq!(list.len(), 2);

    let names: Vec<&str> = list.iter().map(|s| s["name"].as_str().unwrap()).collect();
    assert!(names.contains(&"alpha"));
    assert!(names.contains(&"beta"));
}

#[test]
fn align_finds_field_mappings() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_str().unwrap();

    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_schema_store", "arguments": {
                    "name": "medical", "version": 1,
                    "fields": [
                        {"name": "patient_id", "kind": "string", "identity": true},
                        {"name": "diagnosis", "kind": "string"},
                        {"name": "date", "kind": "string"}
                    ]
                }
            }}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
                "name": "ket_schema_store", "arguments": {
                    "name": "insurance", "version": 1,
                    "fields": [
                        {"name": "member_id", "kind": "string", "identity": true},
                        {"name": "condition", "kind": "string"},
                        {"name": "claim_date", "kind": "string"}
                    ]
                }
            }}),
        ],
    );

    let med_cid = tool_result(&responses[1])["cid"].as_str().unwrap().to_string();
    let ins_cid = tool_result(&responses[2])["cid"].as_str().unwrap().to_string();

    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_align", "arguments": {
                    "source_schema_cid": med_cid,
                    "target_schema_cid": ins_cid
                }
            }}),
        ],
    );

    let result = tool_result(&responses[1]);
    let candidates = result["candidates"].as_array().unwrap();
    assert!(!candidates.is_empty(), "should find alignment candidates");

    // date -> claim_date should be a high-confidence match (substring bonus)
    let date_match = candidates.iter().find(|c| {
        c["source_field"] == "date" && c["target_field"] == "claim_date"
    });
    assert!(date_match.is_some(), "date -> claim_date should align");
    assert!(
        date_match.unwrap()["confidence"].as_f64().unwrap() > 0.5,
        "date -> claim_date confidence should be > 0.5"
    );
}

#[test]
fn search_finds_stored_content() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_str().unwrap();

    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_put", "arguments": {"content": "the quick brown fox", "kind": "test"}
            }}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
                "name": "ket_put", "arguments": {"content": "lazy dog sleeps", "kind": "test"}
            }}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {
                "name": "ket_search", "arguments": {"query": "brown fox"}
            }}),
        ],
    );

    let results = tool_result(&responses[3]);
    let matches = results["matches"].as_array().unwrap();
    assert_eq!(matches.len(), 1);
    assert!(matches[0]["snippet"].as_str().unwrap().contains("brown fox"));
}

#[test]
fn recent_returns_sorted_nodes() {
    let tmp = TempDir::new().unwrap();
    let home = tmp.path().to_str().unwrap();

    let responses = run_mcp(
        home,
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "ket_store", "arguments": {
                    "content": "first", "kind": "reasoning",
                    "parents": [], "agent": "human"
                }
            }}),
            json!({"jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": {
                "name": "ket_store", "arguments": {
                    "content": "second", "kind": "code",
                    "parents": [], "agent": "claude"
                }
            }}),
            json!({"jsonrpc": "2.0", "id": 4, "method": "tools/call", "params": {
                "name": "ket_recent", "arguments": {"limit": 10}
            }}),
            json!({"jsonrpc": "2.0", "id": 5, "method": "tools/call", "params": {
                "name": "ket_recent", "arguments": {"kind": "code"}
            }}),
        ],
    );

    let all = tool_result(&responses[3]);
    assert_eq!(all["nodes"].as_array().unwrap().len(), 2);

    let code_only = tool_result(&responses[4]);
    let nodes = code_only["nodes"].as_array().unwrap();
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0]["kind"], "code");
}

#[test]
fn unknown_tool_returns_error() {
    let tmp = TempDir::new().unwrap();
    let responses = run_mcp(
        tmp.path().to_str().unwrap(),
        &[
            json!({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}}),
            json!({"jsonrpc": "2.0", "id": 2, "method": "tools/call", "params": {
                "name": "nonexistent_tool", "arguments": {}
            }}),
        ],
    );
    assert!(responses[1]["error"].is_object());
}
