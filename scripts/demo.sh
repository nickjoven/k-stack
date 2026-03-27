#!/usr/bin/env bash
set -euo pipefail

# k-stack demo — run this to see it work in 30 seconds
# Usage: ./scripts/demo.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BINARY="$PROJECT_ROOT/target/release/k-stack"

if [ ! -f "$BINARY" ]; then
    echo "Building k-stack..."
    cargo build --release --manifest-path "$PROJECT_ROOT/Cargo.toml" 2>&1 | tail -1
fi

DEMO_HOME=$(mktemp -d)
trap 'rm -rf "$DEMO_HOME"' EXIT

call() {
    local id=$1 name=$2 args=$3
    printf '{"jsonrpc":"2.0","id":%d,"method":"tools/call","params":{"name":"%s","arguments":%s}}\n' "$id" "$name" "$args"
}

echo "=== k-stack demo ==="
echo ""

# --- CAS basics ---
echo "1. Store content, get a CID back"
RESULT=$(printf '%s\n' \
    '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}' \
    "$(call 1 ket_put '{"content":"Design decision: use BLAKE3 for content addressing because it is fast, secure, and produces deterministic hashes.","kind":"reasoning"}')" \
    | KET_HOME="$DEMO_HOME" "$BINARY" 2>/dev/null | tail -1)
CID=$(echo "$RESULT" | python3 -c "import sys,json; r=json.load(sys.stdin); print(json.loads(r['result']['content'][0]['text'])['cid'])")
echo "   CID: $CID"
echo ""

echo "2. Same content = same CID (automatic dedup)"
RESULT2=$(printf '%s\n' \
    '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}' \
    "$(call 1 ket_put '{"content":"Design decision: use BLAKE3 for content addressing because it is fast, secure, and produces deterministic hashes.","kind":"duplicate"}')" \
    | KET_HOME="$DEMO_HOME" "$BINARY" 2>/dev/null | tail -1)
CID2=$(echo "$RESULT2" | python3 -c "import sys,json; r=json.load(sys.stdin); print(json.loads(r['result']['content'][0]['text'])['cid'])")
echo "   CID: $CID2"
if [ "$CID" = "$CID2" ]; then
    echo "   Match! Stored once, referenced twice."
else
    echo "   ERROR: CIDs differ"
fi
echo ""

# --- DAG lineage ---
echo "3. Create a reasoning chain: decision -> implementation -> test"
RESULT=$(printf '%s\n' \
    '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}' \
    "$(call 1 ket_store "{\"content\":\"Decision: use content addressing\",\"kind\":\"reasoning\",\"parents\":[],\"agent\":\"human\"}")" \
    | KET_HOME="$DEMO_HOME" "$BINARY" 2>/dev/null | tail -1)
DECISION_CID=$(echo "$RESULT" | python3 -c "import sys,json; r=json.load(sys.stdin); print(json.loads(r['result']['content'][0]['text'])['node_cid'])")
echo "   Decision:       $DECISION_CID"

RESULT=$(printf '%s\n' \
    '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}' \
    "$(call 1 ket_store "{\"content\":\"Implemented BLAKE3 hashing in Store::put\",\"kind\":\"code\",\"parents\":[\"$DECISION_CID\"],\"agent\":\"claude\"}")" \
    | KET_HOME="$DEMO_HOME" "$BINARY" 2>/dev/null | tail -1)
IMPL_CID=$(echo "$RESULT" | python3 -c "import sys,json; r=json.load(sys.stdin); print(json.loads(r['result']['content'][0]['text'])['node_cid'])")
echo "   Implementation: $IMPL_CID"

RESULT=$(printf '%s\n' \
    '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}' \
    "$(call 1 ket_store "{\"content\":\"All 12 tests passing\",\"kind\":\"reasoning\",\"parents\":[\"$IMPL_CID\"],\"agent\":\"claude\"}")" \
    | KET_HOME="$DEMO_HOME" "$BINARY" 2>/dev/null | tail -1)
TEST_CID=$(echo "$RESULT" | python3 -c "import sys,json; r=json.load(sys.stdin); print(json.loads(r['result']['content'][0]['text'])['node_cid'])")
echo "   Test result:    $TEST_CID"
echo ""

echo "4. Trace lineage: who decided what, and why?"
RESULT=$(printf '%s\n' \
    '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}' \
    "$(call 1 ket_lineage "{\"cid\":\"$TEST_CID\"}")" \
    | KET_HOME="$DEMO_HOME" "$BINARY" 2>/dev/null | tail -1)
echo "$RESULT" | python3 -c "
import sys, json
r = json.load(sys.stdin)
chain = json.loads(r['result']['content'][0]['text'])['chain']
for i, node in enumerate(chain):
    arrow = '   ' if i == 0 else '   <- '
    print(f'{arrow}{node[\"kind\"]} (agent: {node[\"agent\"]})')
"
echo ""

# --- Schema alignment ---
echo "5. Schema alignment: can two different schemas talk to each other?"
RESULT=$(printf '%s\n' \
    '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}' \
    "$(call 1 ket_schema_store '{"name":"medical","version":1,"fields":[{"name":"patient_id","kind":"string","identity":true},{"name":"diagnosis","kind":"string"},{"name":"date","kind":"string"}]}')" \
    "$(call 2 ket_schema_store '{"name":"insurance","version":1,"fields":[{"name":"member_id","kind":"string","identity":true},{"name":"condition","kind":"string"},{"name":"claim_date","kind":"string"}]}')" \
    | KET_HOME="$DEMO_HOME" "$BINARY" 2>/dev/null | tail -2)
MED_CID=$(echo "$RESULT" | head -1 | python3 -c "import sys,json; r=json.load(sys.stdin); print(json.loads(r['result']['content'][0]['text'])['cid'])")
INS_CID=$(echo "$RESULT" | tail -1 | python3 -c "import sys,json; r=json.load(sys.stdin); print(json.loads(r['result']['content'][0]['text'])['cid'])")

RESULT=$(printf '%s\n' \
    '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}' \
    "$(call 1 ket_align "{\"source_schema_cid\":\"$MED_CID\",\"target_schema_cid\":\"$INS_CID\"}")" \
    | KET_HOME="$DEMO_HOME" "$BINARY" 2>/dev/null | tail -1)
echo "$RESULT" | python3 -c "
import sys, json
r = json.load(sys.stdin)
data = json.loads(r['result']['content'][0]['text'])
print(f'   {data[\"source_schema\"]} <-> {data[\"target_schema\"]}')
for c in data['candidates'][:3]:
    conf = c['confidence']
    bar = '#' * int(conf * 20)
    print(f'   {c[\"source_field\"]:>12} -> {c[\"target_field\"]:<16} [{bar:<20}] {conf:.2f}')
"
echo ""

# --- Secret rejection ---
echo "6. Secret rejection: refuses to store API keys"
RESULT=$(printf '%s\n' \
    '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}' \
    "$(call 1 ket_put '{"content":"sk-live-abc123secretkey","kind":"test"}')" \
    | KET_HOME="$DEMO_HOME" "$BINARY" 2>/dev/null | tail -1)
ERROR=$(echo "$RESULT" | python3 -c "import sys,json; r=json.load(sys.stdin); print(r.get('error',{}).get('message','no error'))" 2>/dev/null)
echo "   $ERROR"
echo ""

echo "=== Done. Clean store at $DEMO_HOME removed on exit. ==="
