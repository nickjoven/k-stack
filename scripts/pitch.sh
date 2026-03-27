#!/usr/bin/env bash
set -euo pipefail

# k-stack: a commercial in bash
# Usage: ./scripts/pitch.sh

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
BINARY="$PROJECT_ROOT/target/release/k-stack"

if [ ! -f "$BINARY" ]; then
    echo "Building k-stack..."
    cargo build --release --manifest-path "$PROJECT_ROOT/Cargo.toml" 2>&1 | tail -1
    echo ""
fi

DEMO_HOME=$(mktemp -d)
trap 'rm -rf "$DEMO_HOME"' EXIT

call() {
    local id=$1 name=$2 args=$3
    printf '{"jsonrpc":"2.0","id":%d,"method":"tools/call","params":{"name":"%s","arguments":%s}}\n' "$id" "$name" "$args"
}

mcp() {
    printf '%s\n' '{"jsonrpc":"2.0","id":0,"method":"initialize","params":{}}' "$@" \
        | KET_HOME="$DEMO_HOME" "$BINARY" 2>/dev/null | tail -1
}

extract() {
    python3 -c "import sys,json; r=json.load(sys.stdin); print(json.loads(r['result']['content'][0]['text'])$1)"
}

CYAN='\033[0;36m'
DIM='\033[2m'
BOLD='\033[1m'
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
RESET='\033[0m'

slow() {
    local text="$1"
    local delay="${2:-0.02}"
    for (( i=0; i<${#text}; i++ )); do
        printf '%s' "${text:$i:1}"
        sleep "$delay"
    done
    echo ""
}

pause() { sleep "${1:-1.2}"; }

clear
echo ""
slow "${BOLD}Tired of repeating yourself?${RESET}" 0.04
pause
echo ""

echo -e "${DIM}# Monday. New chat window.${RESET}"
pause 0.8
slow "You: \"We're using BLAKE3 for content addressing because—\"" 0.025
slow "LLM: \"Got it! Let me help you implement SHA-256...\"" 0.025
pause
echo ""

echo -e "${DIM}# Tuesday. Different model.${RESET}"
pause 0.8
slow "You: \"As I explained yesterday, the architecture uses—\"" 0.025
slow "LLM: \"I don't have context from previous sessions.\"" 0.025
pause
echo ""

echo -e "${DIM}# Wednesday. Same project, third time.${RESET}"
pause 0.8
slow "You: \"OK. One more time. From the top.\"" 0.025
pause 1.5
echo ""

echo -e "${DIM}───────────────────────────────────────────${RESET}"
echo ""
slow "${BOLD}What if the context proved itself?${RESET}" 0.04
pause
echo ""
slow "Not remembered. Not summarized. ${BOLD}Retrieved — and verified by the act of retrieval.${RESET}" 0.03
pause 1.5
echo ""
echo -e "${DIM}───────────────────────────────────────────${RESET}"
echo ""

# --- ACT 1: Store once ---
echo -e "${CYAN}$ ket_put \"Design decision: use BLAKE3 for content addressing\"${RESET}"
pause 0.5
RESULT=$(mcp "$(call 1 ket_put '{"content":"Design decision: use BLAKE3 for content addressing because it is fast, secure, and produces deterministic hashes across all platforms.","kind":"reasoning"}')")
CID=$(echo "$RESULT" | extract "['cid']")
echo -e "  ${GREEN}cid: ${CID}${RESET}"
pause

echo ""
echo -e "${DIM}# That hash IS the content. If it didn't match, you wouldn't get it back.${RESET}"
pause 1.5
echo ""

# --- ACT 2: Dedup ---
echo -e "${CYAN}$ ket_put \"Design decision: use BLAKE3 for content addressing\"${RESET}"
pause 0.5
RESULT=$(mcp "$(call 1 ket_put '{"content":"Design decision: use BLAKE3 for content addressing because it is fast, secure, and produces deterministic hashes across all platforms.","kind":"reasoning"}')")
CID2=$(echo "$RESULT" | extract "['cid']")
echo -e "  ${GREEN}cid: ${CID2}${RESET}"
pause 0.5

if [ "$CID" = "$CID2" ]; then
    echo ""
    echo -e "  ${BOLD}Same content. Same hash. Stored once.${RESET}"
    echo -e "  ${DIM}No dedup policy. No storage rules. It's structural.${RESET}"
fi
pause 1.5
echo ""

# --- ACT 3: Lineage ---
echo -e "${DIM}───────────────────────────────────────────${RESET}"
echo ""
slow "Now link it to what came next." 0.03
echo ""
pause 0.5

echo -e "${CYAN}$ ket_store \"Decision: use BLAKE3\" kind=reasoning agent=human${RESET}"
RESULT=$(mcp "$(call 1 ket_store '{"content":"Decision: use BLAKE3 for content addressing","kind":"reasoning","parents":[],"agent":"human"}')")
D_CID=$(echo "$RESULT" | extract "['node_cid']")
echo -e "  ${GREEN}node: ${D_CID:0:16}...${RESET}"
pause 0.4

echo -e "${CYAN}$ ket_store \"Implemented Store::put\" kind=code agent=claude parents=[$D_CID]${RESET}"
RESULT=$(mcp "$(call 1 ket_store "{\"content\":\"Implemented BLAKE3 hashing in Store::put with atomic writes\",\"kind\":\"code\",\"parents\":[\"$D_CID\"],\"agent\":\"claude\"}")")
I_CID=$(echo "$RESULT" | extract "['node_cid']")
echo -e "  ${GREEN}node: ${I_CID:0:16}...${RESET}"
pause 0.4

echo -e "${CYAN}$ ket_store \"12 tests passing\" kind=reasoning agent=claude parents=[$I_CID]${RESET}"
RESULT=$(mcp "$(call 1 ket_store "{\"content\":\"All 12 integration tests passing — CAS, DAG, schema, alignment\",\"kind\":\"reasoning\",\"parents\":[\"$I_CID\"],\"agent\":\"claude\"}")")
T_CID=$(echo "$RESULT" | extract "['node_cid']")
echo -e "  ${GREEN}node: ${T_CID:0:16}...${RESET}"
pause 1

echo ""
echo -e "${CYAN}$ ket_lineage $T_CID${RESET}"
pause 0.5
RESULT=$(mcp "$(call 1 ket_lineage "{\"cid\":\"$T_CID\"}")")
echo "$RESULT" | python3 -c "
import sys, json
r = json.load(sys.stdin)
chain = json.loads(r['result']['content'][0]['text'])['chain']
for i, node in enumerate(chain):
    indent = '    ' + ('  ' * i)
    kind = node['kind']
    agent = node['agent']
    arrow = '' if i == 0 else '<- '
    print(f'  {indent}{arrow}\033[1m{kind}\033[0m ({agent})')
"
pause 1
echo ""
echo -e "  ${DIM}\"Why did we do this?\" is a graph query. Not an archaeology project.${RESET}"
pause 1.5
echo ""

# --- ACT 4: Schema alignment ---
echo -e "${DIM}───────────────────────────────────────────${RESET}"
echo ""
slow "Two teams. Two schemas. Same entities." 0.03
echo ""
pause 0.5

echo -e "${CYAN}$ ket_schema_store medical v1: patient_id, diagnosis, date${RESET}"
RESULT=$(mcp "$(call 1 ket_schema_store '{"name":"medical","version":1,"fields":[{"name":"patient_id","kind":"string","identity":true},{"name":"diagnosis","kind":"string"},{"name":"date","kind":"string"}]}')")
MED=$(echo "$RESULT" | extract "['cid']")
echo -e "  ${GREEN}schema: ${MED:0:16}...${RESET}"
pause 0.3

echo -e "${CYAN}$ ket_schema_store insurance v1: member_id, condition, claim_date${RESET}"
RESULT=$(mcp "$(call 1 ket_schema_store '{"name":"insurance","version":1,"fields":[{"name":"member_id","kind":"string","identity":true},{"name":"condition","kind":"string"},{"name":"claim_date","kind":"string"}]}')")
INS=$(echo "$RESULT" | extract "['cid']")
echo -e "  ${GREEN}schema: ${INS:0:16}...${RESET}"
pause 0.8

echo ""
echo -e "${CYAN}$ ket_align medical insurance${RESET}"
pause 0.5
RESULT=$(mcp "$(call 1 ket_align "{\"source_schema_cid\":\"$MED\",\"target_schema_cid\":\"$INS\"}")")
echo "$RESULT" | python3 -c "
import sys, json
r = json.load(sys.stdin)
data = json.loads(r['result']['content'][0]['text'])
for c in data['candidates'][:3]:
    conf = c['confidence']
    n = int(conf * 20)
    bar = '\033[32m' + '█' * n + '\033[2m' + '░' * (20 - n) + '\033[0m'
    print(f'  {c[\"source_field\"]:>12} -> {c[\"target_field\"]:<16} {bar} {conf:.0%}')
"
pause 1
echo ""
echo -e "  ${DIM}No ML. No embeddings. Pure structural comparison.${RESET}"
echo -e "  ${DIM}Name similarity + type compatibility + identity alignment.${RESET}"
pause 1.5
echo ""

# --- ACT 5: Guardrails ---
echo -e "${DIM}───────────────────────────────────────────${RESET}"
echo ""
echo -e "${CYAN}$ ket_put \"sk-live-abc123-my-production-api-key\"${RESET}"
pause 0.8
RESULT=$(mcp "$(call 1 ket_put '{"content":"sk-live-abc123-my-production-api-key","kind":"oops"}')")
ERROR=$(echo "$RESULT" | python3 -c "import sys,json; r=json.load(sys.stdin); print(r.get('error',{}).get('message',''))")
echo -e "  ${RED}✗ $ERROR${RESET}"
pause 1
echo ""
echo -e "  ${DIM}Secrets never enter the store. Checked on write, not on policy review.${RESET}"
pause 1.5
echo ""

# --- CLOSE ---
echo -e "${DIM}───────────────────────────────────────────${RESET}"
echo ""
slow "${BOLD}Tired of repeating yourself?${RESET}" 0.04
echo ""
echo -e "  ${DIM}git solved this for files.${RESET}"
pause 0.8
echo -e "  ${BOLD}k-stack solves it for everything else.${RESET}"
pause 1
echo ""
echo -e "  ${CYAN}git clone https://github.com/nickjoven/k-stack${RESET}"
echo -e "  ${CYAN}cd k-stack && ./scripts/install.sh${RESET}"
echo ""
echo -e "  ${DIM}15 tools. Any LLM. Content in, CID out, parents link the story.${RESET}"
echo ""
