#!/usr/bin/env bash
set -euo pipefail

echo "============================================"
echo " HardTest Pipeline Execution"
echo "============================================"

# Cleanup any stale anvil
pkill anvil 2>/dev/null || true
sleep 1

# 1. Start anvil
echo ""
echo "--- Step 1: Starting anvil ---"
anvil --block-time 1 --silent &
ANVIL_PID=$!
sleep 3
echo "  anvil started (pid=$ANVIL_PID)"

cleanup() {
    echo ""
    echo "--- Cleanup: stopping anvil ---"
    kill $ANVIL_PID 2>/dev/null || true
}
trap cleanup EXIT

# 2. Deploy contracts
echo ""
echo "--- Step 2: Deploying HardTest contracts ---"
DEPLOY_OUT=$(forge script script/DeployHardTest.s.sol --broadcast --rpc-url http://localhost:8545 --sender 0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266 2>&1)
echo "$DEPLOY_OUT"

# Extract HardLender address
HARD_LENDER=$(echo "$DEPLOY_OUT" | grep "HardLender deployed at" | grep -oE '0x[a-fA-F0-9]{40}')
ORACLE_PROXY=$(echo "$DEPLOY_OUT" | grep "HardOracleProxy deployed at" | grep -oE '0x[a-fA-F0-9]{40}')

echo ""
echo "  HardLender:     $HARD_LENDER"
echo "  OracleProxy:    $ORACLE_PROXY"
echo "  Lending pool balance: $(cast balance $HARD_LENDER --rpc-url http://localhost:8545)"
echo "  Oracle code: $(cast code $ORACLE_PROXY --rpc-url http://localhost:8545 | head -c 60)..."

# 3. Run pipeline
echo ""
echo "--- Step 3: Running pipeline with RUST_LOG=info ---"
echo ""
DRPC_URL=http://localhost:8545 \
RUST_LOG=info \
cargo run -- scan "$HARD_LENDER" 2>&1 || true

echo ""
echo "============================================"
echo " HardTest complete"
echo "============================================"
