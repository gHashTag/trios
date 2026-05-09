#!/usr/bin/env bash
# trios-mesh-node — 25-test E2E suite
# L-E2E-1 · trinity-fpga#23 · EPIC trinity-fpga#22
# Anchor: φ² + φ⁻² = 3
#
# Runs against two locally-launched mesh-node instances on $PORT_A / $PORT_B.
# CI calls this after `cargo build --release -p trios-mesh-node`.
#
# Tests ladder up by category:
#   T1–T5    health / info
#   T6–T10   announce path
#   T11–T15  next-hop path
#   T16–T20  encryption (X25519 + ChaCha20)
#   T21–T25  edge cases (replay, MITM, GF16 clamp, persistence smoke)

set -uo pipefail

BIN="${BIN:-./target/release/mesh-node}"
PORT_A="${PORT_A:-18080}"
PORT_B="${PORT_B:-18081}"
A="http://localhost:${PORT_A}"
B="http://localhost:${PORT_B}"

PASS=0
FAIL=0
declare -a FAILED_TESTS=()

ok()   { PASS=$((PASS+1)); printf "  ✅ %-44s %s\n" "$1" "${2:-}"; }
fail() { FAIL=$((FAIL+1)); FAILED_TESTS+=("$1"); printf "  ❌ %-44s %s\n" "$1" "${2:-}"; }

assert_eq() {
    # $1=label  $2=expected  $3=actual
    if [[ "$2" == "$3" ]]; then ok "$1" "= $2"; else fail "$1" "expected='$2' actual='$3'"; fi
}

assert_contains() {
    if [[ "$3" == *"$2"* ]]; then ok "$1" "contains '$2'"; else fail "$1" "no '$2' in '$3'"; fi
}

cleanup() {
    [[ -n "${NODE_A_PID:-}" ]] && kill -9 "$NODE_A_PID" 2>/dev/null || true
    [[ -n "${NODE_B_PID:-}" ]] && kill -9 "$NODE_B_PID" 2>/dev/null || true
}
trap cleanup EXIT

# ── Boot two nodes ──────────────────────────────────────────────────────────
MESH_SEED=0 MESH_NODE_NAME=node-0 PORT="$PORT_A" "$BIN" >/tmp/node-a.log 2>&1 &
NODE_A_PID=$!
MESH_SEED=1 MESH_NODE_NAME=node-1 PORT="$PORT_B" "$BIN" >/tmp/node-b.log 2>&1 &
NODE_B_PID=$!

# Wait for both nodes to be reachable (max ~10 s)
for _ in $(seq 1 20); do
    sleep 0.5
    curl -sf "$A/health" >/dev/null 2>&1 && curl -sf "$B/health" >/dev/null 2>&1 && break
done

echo "── trios-mesh-node E2E suite (25 tests) ────────────────────────────────"
echo "  node-A → $A   pid=$NODE_A_PID"
echo "  node-B → $B   pid=$NODE_B_PID"
echo

# ── T1–T5: health / info ────────────────────────────────────────────────────
H_A=$(curl -sf "$A/health" || echo "")
assert_eq "T1  GET /health node-A returns 'ok'" "ok" "$H_A"

H_B=$(curl -sf "$B/health" || echo "")
assert_eq "T2  GET /health node-B returns 'ok'" "ok" "$H_B"

INFO_A=$(curl -sf "$A/info")
NAME_A=$(echo "$INFO_A" | python3 -c "import sys,json; print(json.load(sys.stdin)['name'])")
# Convention (ADR in this PR): MESH_NODE_NAME is honoured verbatim — no "trinity-" prefix.
assert_eq "T3  /info honours MESH_NODE_NAME (no prefix)" "node-0" "$NAME_A"

ENC_A=$(echo "$INFO_A" | python3 -c "import sys,json; print(json.load(sys.stdin)['encryption'])")
assert_eq "T4  /info advertises encryption suite" "X25519-ECDH+ChaCha20Poly1305" "$ENC_A"

PUB_A=$(echo "$INFO_A" | python3 -c "import sys,json; print(json.load(sys.stdin)['pubkey'])")
if [[ ${#PUB_A} -eq 64 ]]; then ok "T5  pubkey is 32-byte hex (64 chars)" "len=${#PUB_A}"
else fail "T5  pubkey length" "got ${#PUB_A}"; fi

DEST_A=$(echo "$INFO_A" | python3 -c "import sys,json; print(json.load(sys.stdin)['dest_hash'])")
INFO_B=$(curl -sf "$B/info")
DEST_B=$(echo "$INFO_B" | python3 -c "import sys,json; print(json.load(sys.stdin)['dest_hash'])")
PUB_B=$(echo "$INFO_B" | python3 -c "import sys,json; print(json.load(sys.stdin)['pubkey'])")

# ── T6–T10: announce ────────────────────────────────────────────────────────
RES=$(curl -sf -X POST "$A/announce" -H 'Content-Type: application/json' \
        -d "{\"dest_hash\":\"$DEST_B\",\"sender\":\"$DEST_B\",\"hops\":1,\"quality\":2}")
ACC=$(echo "$RES" | python3 -c "import sys,json; print(json.load(sys.stdin)['accepted'])")
assert_eq "T6  POST /announce basic accept" "True" "$ACC"

# Worse path → still accepted on first sight of new dest, here re-announce of same dest
# Cheaper path (lower cost) → must be accepted
RES=$(curl -sf -X POST "$A/announce" -H 'Content-Type: application/json' \
        -d "{\"dest_hash\":\"$DEST_B\",\"sender\":\"$DEST_B\",\"hops\":1,\"quality\":0}")
ACC=$(echo "$RES" | python3 -c "import sys,json; print(json.load(sys.stdin)['accepted'])")
assert_eq "T7  /announce strictly-better path replaces" "True" "$ACC"

# Worse path → must be rejected (ETX metric working)
RES=$(curl -sf -X POST "$A/announce" -H 'Content-Type: application/json' \
        -d "{\"dest_hash\":\"$DEST_B\",\"sender\":\"$DEST_B\",\"hops\":5,\"quality\":15}")
ACC=$(echo "$RES" | python3 -c "import sys,json; print(json.load(sys.stdin)['accepted'])")
assert_eq "T8  /announce worse path rejected (ETX)" "False" "$ACC"

# Malformed dest_hash → not accepted
RES=$(curl -sf -X POST "$A/announce" -H 'Content-Type: application/json' \
        -d '{"dest_hash":"not-hex","sender":"not-hex","hops":1,"quality":1}' || echo '{"accepted":false}')
ACC=$(echo "$RES" | python3 -c "import sys,json; print(json.load(sys.stdin)['accepted'])")
assert_eq "T9  /announce rejects malformed hex" "False" "$ACC"

# announce with pubkey field (post-L-E2E-3): still accepted (no-op for routing)
RES=$(curl -sf -X POST "$A/announce" -H 'Content-Type: application/json' \
        -d "{\"dest_hash\":\"$DEST_B\",\"sender\":\"$DEST_B\",\"hops\":1,\"quality\":0,\"pubkey\":\"$PUB_B\"}")
[[ -n "$RES" ]] && ok "T10 /announce accepts pubkey field" "" || fail "T10 /announce with pubkey"

# ── T11–T15: next-hop ───────────────────────────────────────────────────────
HOP=$(curl -sf -X POST "$A/next-hop" -H 'Content-Type: application/json' \
        -d "{\"dest_hash\":\"$DEST_B\"}")
NH=$(echo "$HOP" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('next_hop') or '')")
assert_eq "T11 next-hop B from A returns DEST_B" "$DEST_B" "$NH"

LOCAL=$(curl -sf -X POST "$A/next-hop" -H 'Content-Type: application/json' \
        -d "{\"dest_hash\":\"$DEST_A\"}" | python3 -c "import sys,json; print(json.load(sys.stdin)['local'])")
assert_eq "T12 next-hop self → local=true" "True" "$LOCAL"

# Unknown dest → next_hop=None
UNKNOWN_HOP=$(curl -sf -X POST "$A/next-hop" -H 'Content-Type: application/json' \
        -d '{"dest_hash":"00000000000000000000000000000000"}' \
        | python3 -c "import sys,json; d=json.load(sys.stdin); print('null' if d['next_hop'] is None else d['next_hop'])")
assert_eq "T13 next-hop unknown → null" "null" "$UNKNOWN_HOP"

INVALID=$(curl -sf -X POST "$A/next-hop" -H 'Content-Type: application/json' \
        -d '{"dest_hash":"zzz"}' || echo '{"local":false,"next_hop":null}')
LOCAL2=$(echo "$INVALID" | python3 -c "import sys,json; print(json.load(sys.stdin)['local'])")
assert_eq "T14 next-hop invalid hex → local=false" "False" "$LOCAL2"

# Routes count visible in /info
ROUTES_A=$(curl -sf "$A/info" | python3 -c "import sys,json; print(json.load(sys.stdin)['routes'])")
if [[ "$ROUTES_A" -ge 1 ]]; then ok "T15 /info routes count ≥ 1" "= $ROUTES_A"
else fail "T15 /info routes count" "got $ROUTES_A"; fi

# ── T16–T20: encryption ─────────────────────────────────────────────────────
PLAINTEXT="phi^2 + phi^-2 = 3"
ENC_RESP=$(curl -sf -X POST "$A/encrypt" -H 'Content-Type: application/json' \
        -d "{\"recipient_pubkey\":\"$PUB_B\",\"plaintext\":\"$PLAINTEXT\"}")
PAYLOAD=$(echo "$ENC_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['payload'])")
SENDER_PK=$(echo "$ENC_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['sender_pubkey'])")
[[ -n "$PAYLOAD" && "$PAYLOAD" != error* ]] && ok "T16 /encrypt produces ciphertext" "len=${#PAYLOAD}" \
    || fail "T16 /encrypt" "$PAYLOAD"
assert_eq "T17 /encrypt sender_pubkey == node-A pubkey" "$PUB_A" "$SENDER_PK"

# B decrypts via /message
DEC_RESP=$(curl -sf -X POST "$B/message" -H 'Content-Type: application/json' \
        -d "{\"to\":\"$DEST_B\",\"sender_pubkey\":\"$SENDER_PK\",\"payload\":\"$PAYLOAD\"}")
DELIV=$(echo "$DEC_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['delivered'])")
DEC=$(echo "$DEC_RESP"   | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('decrypted') or '')")
assert_eq "T18 /message delivered=true on B" "True" "$DELIV"
assert_eq "T19 /message decrypted plaintext matches" "$PLAINTEXT" "$DEC"

# Encryption against invalid pubkey returns error payload
BAD=$(curl -sf -X POST "$A/encrypt" -H 'Content-Type: application/json' \
        -d '{"recipient_pubkey":"deadbeef","plaintext":"x"}')
BADP=$(echo "$BAD" | python3 -c "import sys,json; print(json.load(sys.stdin)['payload'])")
assert_contains "T20 /encrypt errors on bad pubkey" "error:" "$BADP"

# ── T21–T25: edge cases ─────────────────────────────────────────────────────

# T21 — MITM tamper: flip a byte in payload, expect decrypt failure
TAMPERED="${PAYLOAD/A/B}"; TAMPERED="${TAMPERED/a/b}"
[[ "$TAMPERED" == "$PAYLOAD" ]] && TAMPERED="X${PAYLOAD:1}"  # ensure mutation
TAMP_RESP=$(curl -sf -X POST "$B/message" -H 'Content-Type: application/json' \
        -d "{\"to\":\"$DEST_B\",\"sender_pubkey\":\"$SENDER_PK\",\"payload\":\"$TAMPERED\"}")
TAMP_OK=$(echo "$TAMP_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['delivered'])")
assert_eq "T21 MITM tampered payload rejected" "False" "$TAMP_OK"

# T22 — wrong sender_pubkey: AEAD must fail
WRONG_PK="0000000000000000000000000000000000000000000000000000000000000000"
WRONG_RESP=$(curl -sf -X POST "$B/message" -H 'Content-Type: application/json' \
        -d "{\"to\":\"$DEST_B\",\"sender_pubkey\":\"$WRONG_PK\",\"payload\":\"$PAYLOAD\"}")
WRONG_OK=$(echo "$WRONG_RESP" | python3 -c "import sys,json; print(json.load(sys.stdin)['delivered'])")
assert_eq "T22 wrong sender pubkey → undecryptable" "False" "$WRONG_OK"

# T23 — message addressed to another node yields next-hop hint, no decrypt
FOREIGN=$(curl -sf -X POST "$B/message" -H 'Content-Type: application/json' \
        -d "{\"to\":\"$DEST_A\",\"sender_pubkey\":\"$SENDER_PK\",\"payload\":\"$PAYLOAD\"}")
DEL=$(echo "$FOREIGN" | python3 -c "import sys,json; print(json.load(sys.stdin)['delivered'])")
assert_eq "T23 foreign /message not decrypted on B" "False" "$DEL"

# T24 — GF16 clamp: hops=0xFF, quality=0xFF must be silently clamped, still accepted
RES=$(curl -sf -X POST "$A/announce" -H 'Content-Type: application/json' \
        -d "{\"dest_hash\":\"deadbeefdeadbeefdeadbeefdeadbeef\",\"sender\":\"$DEST_B\",\"hops\":255,\"quality\":255}")
ACC=$(echo "$RES" | python3 -c "import sys,json; print(json.load(sys.stdin)['accepted'])")
assert_eq "T24 GF16 nibble clamp on huge values" "True" "$ACC"

# T25 — persistence smoke: when DATABASE_URL unset, /info still works (no crash)
[[ -z "${DATABASE_URL:-}" ]] && PERSIST_NOTE="DATABASE_URL unset (in-memory mode)" \
                             || PERSIST_NOTE="DATABASE_URL set"
PING_OK=$(curl -sf "$A/info" | python3 -c "import sys,json; print('ok' if json.load(sys.stdin) else 'fail')")
assert_eq "T25 in-memory mode survives without DB ($PERSIST_NOTE)" "ok" "$PING_OK"

# ── Summary ─────────────────────────────────────────────────────────────────
TOTAL=$((PASS+FAIL))
echo
echo "──────────────────────────────────────────────────────────────"
echo "  $PASS / $TOTAL tests green"
if [[ $FAIL -gt 0 ]]; then
    echo "  ❌ Failed:"
    for t in "${FAILED_TESTS[@]}"; do echo "      - $t"; done
    echo
    echo "── node-A log (last 30 lines) ─"; tail -30 /tmp/node-a.log
    echo "── node-B log (last 30 lines) ─"; tail -30 /tmp/node-b.log
    exit 1
fi
echo "  🎉 25/25 green · φ² + φ⁻² = 3"
