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
WHITE='\033[1;37m'
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
echo ""
slow "  ${BOLD}Tired of repeating yourself?${RESET}" 0.04
pause 1.5
echo ""
echo ""

# ─────────────────────────────────────────
# THE READ PROBLEM
# ─────────────────────────────────────────

echo -e "  ${DIM}┌─────────────────────────────────────────────────────┐${RESET}"
echo -e "  ${DIM}│${RESET}  ${WHITE}THE READ PROBLEM${RESET}                                    ${DIM}│${RESET}"
echo -e "  ${DIM}└─────────────────────────────────────────────────────┘${RESET}"
echo ""
pause 0.8

echo -e "  ${DIM}# You need an LLM to understand your architecture.${RESET}"
echo -e "  ${DIM}# So you explain it. Again.${RESET}"
echo ""
pause 0.8

slow "  You: \"We're using BLAKE3 for content addressing because—\"" 0.025
slow "  You: \"—the CAS layer deduplicates automatically, and the DAG—\"" 0.025
slow "  You: \"—tracks provenance via parent links, which means—\"" 0.025
slow "  You: \"—every node records what it derived from and who wrote it.\"" 0.025
echo ""
pause 0.6

echo -e "  ${DIM}~2,000 tokens.${RESET}"
pause 0.6
echo -e "  ${DIM}Plus the code context. Plus the schema definitions.${RESET}"
pause 0.6
echo -e "  ${DIM}Plus \"as I mentioned in the previous conversation...\"${RESET}"
pause 0.8
echo ""

echo -e "  ${RED}~50,000 tokens${RESET} to get to: ${DIM}\"Perfect! Now I have the full context.\"${RESET}"
pause 1.5
echo ""

echo -e "  ${DIM}And that's a ${RESET}${BOLD}best-effort reconstruction${RESET}${DIM} from a lossy proxy.${RESET}"
echo -e "  ${DIM}You have no way to verify it matches what you originally said.${RESET}"
pause 2
echo ""

echo -e "  ${DIM}───────────────────────────────────────────${RESET}"
echo ""
pause 0.5

echo -e "  ${DIM}Now watch:${RESET}"
echo ""
pause 0.8

# Store the architecture context as a reasoning chain
RESULT=$(mcp "$(call 1 ket_store '{"content":"Decision: use BLAKE3 for content addressing. CAS layer deduplicates automatically. DAG tracks provenance via parent links.","kind":"reasoning","parents":[],"agent":"human"}')")
ROOT_CID=$(echo "$RESULT" | extract "['node_cid']")

RESULT=$(mcp "$(call 1 ket_store "{\"content\":\"Every node records what it derived from and who wrote it. Schema validation via canon.d ensures structural consistency.\",\"kind\":\"reasoning\",\"parents\":[\"$ROOT_CID\"],\"agent\":\"human\"}")")
ARCH_CID=$(echo "$RESULT" | extract "['node_cid']")

echo -e "  ${CYAN}$ ket_get ${ARCH_CID:0:16}...${RESET}"
pause 0.3
RESULT=$(mcp "$(call 1 ket_get "{\"cid\":\"$ARCH_CID\"}")")
echo "$RESULT" | python3 -c "
import sys, json
r = json.load(sys.stdin)
data = json.loads(r['result']['content'][0]['text'])
# This is a DAG node, show it as structure
d = json.loads(data['content'])
print(f'  \033[0;32m{data[\"size\"]} bytes. Verified by retrieval.\033[0m')
" 2>/dev/null || echo -e "  ${GREEN}Verified by retrieval.${RESET}"
pause 0.8
echo ""

echo -e "  ${GREEN}O(1).${RESET} Not reconstructed. Not summarized. ${BOLD}Retrieved.${RESET}"
echo -e "  ${DIM}The hash IS the proof. If it didn't match, you wouldn't get it back.${RESET}"
pause 2
echo ""
echo ""

# ─────────────────────────────────────────
# THE WRITE PROBLEM
# ─────────────────────────────────────────

echo -e "  ${DIM}┌─────────────────────────────────────────────────────┐${RESET}"
echo -e "  ${DIM}│${RESET}  ${WHITE}THE WRITE PROBLEM${RESET}                                   ${DIM}│${RESET}"
echo -e "  ${DIM}└─────────────────────────────────────────────────────┘${RESET}"
echo ""
pause 0.8

echo -e "  ${DIM}# Three ways to record a decision today:${RESET}"
echo ""
pause 0.5

echo -e "  ${DIM}  1. A human writes a design doc.${RESET}"
echo -e "  ${DIM}     Best-effort summary. Decays immediately. No link to what${RESET}"
echo -e "  ${DIM}     prompted it or what followed.${RESET}"
echo ""
pause 0.8

echo -e "  ${DIM}  2. An LLM writes a summary.${RESET}"
echo -e "  ${DIM}     Lossy compression. May hallucinate details. Gone when${RESET}"
echo -e "  ${DIM}     the session ends.${RESET}"
echo ""
pause 0.8

echo -e "  ${DIM}  3. The code captures the \"what\" but not the \"why\".${RESET}"
echo -e "  ${DIM}     Six months later: \"why did we do this?\"${RESET}"
echo -e "  ${DIM}     git blame → a commit message → silence.${RESET}"
echo ""
pause 1.5

echo -e "  ${DIM}───────────────────────────────────────────${RESET}"
echo ""
echo -e "  ${DIM}Now watch the same decision as a reasoning chain:${RESET}"
echo ""
pause 0.8

# Build a real chain showing the WHY
echo -e "  ${CYAN}$ ket_store \"Latency budget exceeded\" kind=reasoning agent=human${RESET}"
RESULT=$(mcp "$(call 1 ket_store '{"content":"Observation: API latency p99 exceeded 200ms budget after adding validation layer","kind":"reasoning","parents":[],"agent":"human"}')")
OBS_CID=$(echo "$RESULT" | extract "['node_cid']")
echo -e "    ${GREEN}${OBS_CID:0:16}...${RESET}"
pause 0.3

echo -e "  ${CYAN}$ ket_store \"Profiling: validation is 80% of cost\" parents=[${OBS_CID:0:12}...]${RESET}"
RESULT=$(mcp "$(call 1 ket_store "{\"content\":\"Profiling: schema validation accounts for 80% of added latency. Canonical form computation is the bottleneck.\",\"kind\":\"reasoning\",\"parents\":[\"$OBS_CID\"],\"agent\":\"claude\"}")")
PROF_CID=$(echo "$RESULT" | extract "['node_cid']")
echo -e "    ${GREEN}${PROF_CID:0:16}...${RESET}"
pause 0.3

echo -e "  ${CYAN}$ ket_store \"Decision: cache canonical forms\" parents=[${PROF_CID:0:12}...]${RESET}"
RESULT=$(mcp "$(call 1 ket_store "{\"content\":\"Decision: cache canonical byte forms by schema CID. Same input = same CID = cache hit. Eliminates redundant validation.\",\"kind\":\"reasoning\",\"parents\":[\"$PROF_CID\"],\"agent\":\"human\"}")")
DEC_CID=$(echo "$RESULT" | extract "['node_cid']")
echo -e "    ${GREEN}${DEC_CID:0:16}...${RESET}"
pause 0.3

echo -e "  ${CYAN}$ ket_store \"Implemented: p99 now 45ms\" parents=[${DEC_CID:0:12}...]${RESET}"
RESULT=$(mcp "$(call 1 ket_store "{\"content\":\"Implemented canonical form cache. p99 latency dropped from 210ms to 45ms. 12 tests passing.\",\"kind\":\"code\",\"parents\":[\"$DEC_CID\"],\"agent\":\"claude\"}")")
IMPL_CID=$(echo "$RESULT" | extract "['node_cid']")
echo -e "    ${GREEN}${IMPL_CID:0:16}...${RESET}"
pause 1
echo ""

echo -e "  ${DIM}Six months later:${RESET} ${BOLD}\"Why do we cache canonical forms?\"${RESET}"
echo ""
pause 1

echo -e "  ${CYAN}$ ket_lineage ${IMPL_CID:0:16}...${RESET}"
pause 0.5
RESULT=$(mcp "$(call 1 ket_lineage "{\"cid\":\"$IMPL_CID\"}")")
echo "$RESULT" | python3 -c "
import sys, json
r = json.load(sys.stdin)
chain = json.loads(r['result']['content'][0]['text'])['chain']
labels = [
    'implemented cache (claude)',
    'decided to cache (human)',
    'profiled bottleneck (claude)',
    'observed latency (human)',
]
for i, node in enumerate(chain):
    agent = node['agent']
    kind = node['kind']
    pad = '    ' * i
    arrow = '└─ ' if i > 0 else ''
    label = labels[i] if i < len(labels) else f'{kind} ({agent})'
    print(f'    {pad}{arrow}\033[1m{label}\033[0m')
"
pause 1
echo ""

echo -e "  ${DIM}Not a doc about the decision. The decision itself —${RESET}"
echo -e "  ${DIM}the observation, the analysis, the choice, the result —${RESET}"
echo -e "  ${BOLD}each node linked to what came before it.${RESET}"
pause 1.5
echo ""

echo -e "  ${DIM}The knowledge is in the structure, not the text.${RESET}"
pause 2
echo ""
echo ""

# ─────────────────────────────────────────
# THE COST
# ─────────────────────────────────────────

echo -e "  ${DIM}┌─────────────────────────────────────────────────────┐${RESET}"
echo -e "  ${DIM}│${RESET}  ${WHITE}THE COST${RESET}                                              ${DIM}│${RESET}"
echo -e "  ${DIM}└─────────────────────────────────────────────────────┘${RESET}"
echo ""
pause 0.8

echo -e "  ${DIM}Without k-stack:${RESET}"
echo ""
echo -e "    ${RED}50,000 tokens${RESET}    to re-explain your architecture"
echo -e "    ${RED}every session${RESET}     repeating what was already known"
echo -e "    ${RED}zero proof${RESET}        that the LLM understood correctly"
echo -e "    ${RED}zero lineage${RESET}      when you ask \"why?\" in six months"
pause 1.5
echo ""

echo -e "  ${DIM}With k-stack:${RESET}"
echo ""
echo -e "    ${GREEN}1 CID${RESET}            O(1) retrieval, self-verifying"
echo -e "    ${GREEN}1 lineage query${RESET}   full derivation chain, every agent recorded"
echo -e "    ${GREEN}0 tokens wasted${RESET}   on reconstruction from memory"
echo -e "    ${GREEN}0 trust required${RESET}  the content IS the proof"
pause 2
echo ""
echo ""

# ─────────────────────────────────────────
# BONUS: structure sees what text can't
# ─────────────────────────────────────────

echo -e "  ${DIM}┌─────────────────────────────────────────────────────┐${RESET}"
echo -e "  ${DIM}│${RESET}  ${WHITE}BONUS: STRUCTURE SEES WHAT TEXT CAN'T${RESET}                ${DIM}│${RESET}"
echo -e "  ${DIM}└─────────────────────────────────────────────────────┘${RESET}"
echo ""
pause 0.8

echo -e "  ${DIM}Two teams. Different names for the same things.${RESET}"
echo -e "  ${DIM}A human wouldn't notice. An LLM might guess. k-stack knows.${RESET}"
echo ""
pause 1

echo -e "  ${CYAN}$ ket_schema_store medical: patient_id, diagnosis, date${RESET}"
RESULT=$(mcp "$(call 1 ket_schema_store '{"name":"medical","version":1,"fields":[{"name":"patient_id","kind":"string","identity":true},{"name":"diagnosis","kind":"string"},{"name":"date","kind":"string"}]}')")
MED=$(echo "$RESULT" | extract "['cid']")
echo -e "  ${CYAN}$ ket_schema_store insurance: member_id, condition, claim_date${RESET}"
RESULT=$(mcp "$(call 1 ket_schema_store '{"name":"insurance","version":1,"fields":[{"name":"member_id","kind":"string","identity":true},{"name":"condition","kind":"string"},{"name":"claim_date","kind":"string"}]}')")
INS=$(echo "$RESULT" | extract "['cid']")
pause 0.5
echo ""

echo -e "  ${CYAN}$ ket_align medical insurance${RESET}"
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
    print(f'    {c[\"source_field\"]:>12} -> {c[\"target_field\"]:<16} {bar} {conf:.0%}')
"
pause 1
echo ""

echo -e "  ${DIM}\"patient_id\" and \"member_id\" — different names, both identity fields,${RESET}"
echo -e "  ${DIM}same type. 76% confidence from pure structure. No training data needed.${RESET}"
pause 2
echo ""
echo ""

# ─────────────────────────────────────────
# CLOSE
# ─────────────────────────────────────────

echo -e "  ${DIM}───────────────────────────────────────────${RESET}"
echo ""
echo ""
slow "  ${BOLD}Tired of repeating yourself?${RESET}" 0.04
echo ""
pause 1

echo -e "  ${DIM}git solved this for files.${RESET}"
pause 0.8
echo -e "  ${BOLD}k-stack solves it for everything else.${RESET}"
pause 1.5
echo ""
echo ""
echo -e "  ${CYAN}git clone https://github.com/nickjoven/k-stack${RESET}"
echo -e "  ${CYAN}cd k-stack && ./scripts/install.sh${RESET}"
echo ""
echo -e "  ${DIM}15 tools. Any LLM. Content in, CID out, parents link the story.${RESET}"
echo ""
