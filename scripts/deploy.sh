#!/usr/bin/env bash
set -euo pipefail
RPC="http://127.0.0.1:8545"
PK="0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
FROM="0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"

TA_BC=$(python3 -c "import json; print(json.load(open('out/MiniToken.sol/MiniToken.json'))['bytecode']['object'])")
TB_BC=$TA_BC
PAIR_BC=$(python3 -c "import json; print(json.load(open('out/MockPair.sol/MockPair.json'))['bytecode']['object'])")
VAULT_BC=$(python3 -c "import json; print(json.load(open('out/HardVault.sol/HardVault.json'))['bytecode']['object'])")

deploy() {
  cast send --rpc-url $RPC --private-key $PK --create "$1" --from $FROM --json "$@" 2>/dev/null
}

echo "Deploying tokens..."
deploy "$TA_BC" > /tmp/ta.json; TA=$(python3 -c "import json; print(json.load(open('/tmp/ta.json'))['contractAddress'])")
echo "  A=$TA"
deploy "$TB_BC" > /tmp/tb.json; TB=$(python3 -c "import json; print(json.load(open('/tmp/tb.json'))['contractAddress'])")
echo "  B=$TB"

echo "Deploying pair..."
deploy "$PAIR_BC" --constructor-args "$TA" "$TB" > /tmp/pair.json
PAIR=$(python3 -c "import json; print(json.load(open('/tmp/pair.json'))['contractAddress'])")
echo "  Pair=$PAIR"

cast send --rpc-url $RPC --private-key $PK $PAIR "setReserves(uint256,uint256,uint32)" 1000000000000000000000 2000000000000000000000 0 > /dev/null 2>&1

echo "Deploying vault..."
deploy "$VAULT_BC" --constructor-args "$PAIR" "$TA" "$TB" > /tmp/vault.json
VAULT=$(python3 -c "import json; print(json.load(open('/tmp/vault.json'))['contractAddress'])")
echo "  Vault=$VAULT"

cast send --rpc-url $RPC --private-key $PK $TA "approve(address,uint256)" $VAULT 100000000000000000000 > /dev/null 2>&1
cast send --rpc-url $RPC --private-key $PK $VAULT "deposit(uint256)" 100000000000000000000 > /dev/null 2>&1

echo "DONE Vault=$VAULT"