#!/usr/bin/env bash
#
# AgriTrust escrow contract — build, optimize, and deploy to the Stellar testnet.
#
# Uses the `stellar` CLI (v22+; the successor of the old `soroban` CLI).
# `stellar contract build` compiles the crate to wasm32v1-none (release) and
# `--optimize` runs wasm-opt as part of the build, so no separate optimize
# step is needed.
#
# One-time prerequisites:
#   # 1. Create + fund a deploy identity (friendbot faucets testnet XLM):
#   stellar keys generate testnet-admin
#   stellar keys fund testnet-admin
#   # 2. If `testnet` is missing from `stellar network ls`, add it:
#   stellar network add testnet \
#     --rpc-url https://soroban-testnet.stellar.org:443 \
#     --network-passphrase "Test SDF Network ; September 2015"
#
# Usage:
#   ./deploy_testnet.sh                        # deploy with `testnet-admin`
#   SOURCE=alice ./deploy_testnet.sh           # deploy with another identity
#   NETWORK=futurenet ./deploy_testnet.sh      # deploy to a different network
#
# Prints the deployed contract ID on stdout.

set -euo pipefail

cd "$(dirname "$0")"

# ---------------------------------------------------------------------------
# Configuration (override via env vars)
# ---------------------------------------------------------------------------
SOURCE="${SOURCE:-testnet-admin}"   # stellar identity used to sign + pay for the deploy
NETWORK="${NETWORK:-testnet}"
WASM_DIR="${WASM_DIR:-target/wasm32v1-none/release}"
WASM="${WASM:-$WASM_DIR/agritrust_contract.wasm}"
ALIAS="${ALIAS:-agritrust_escrow}"

# ---------------------------------------------------------------------------
# Pre-flight checks
# ---------------------------------------------------------------------------
if ! command -v stellar >/dev/null 2>&1; then
    echo "error: the 'stellar' CLI is not installed or not on PATH." >&2
    echo "       Install it from https://github.com/stellar/stellar-cli" >&2
    exit 1
fi

if ! stellar network ls | grep -qx "$NETWORK"; then
    echo "error: network '$NETWORK' is not configured. Add it with:" >&2
    echo "  stellar network add $NETWORK \\" >&2
    echo "    --rpc-url https://soroban-testnet.stellar.org:443 \\" >&2
    echo "    --network-passphrase \"Test SDF Network ; September 2015\"" >&2
    exit 1
fi

# ---------------------------------------------------------------------------
# 1. Build (wasm32v1-none, release, optimized)
# ---------------------------------------------------------------------------
echo "==> [1/2] Building contract ..."
stellar contract build --optimize

if [ ! -f "$WASM" ]; then
    echo "error: build did not produce '$WASM'." >&2
    echo "       (If the wasm32v1-none target is missing: rustup target add wasm32v1-none)" >&2
    exit 1
fi
echo "    wasm: $WASM"

# ---------------------------------------------------------------------------
# 2. Deploy
# ---------------------------------------------------------------------------
echo "==> [2/2] Deploying to '$NETWORK' ..."
CONTRACT_ID="$(stellar contract deploy \
    --wasm "$WASM" \
    --source "$SOURCE" \
    --network "$NETWORK" \
    --alias "$ALIAS")"

echo
echo "---------------------------------------------"
echo " AgriTrust escrow contract deployed!"
echo "   Network:     $NETWORK"
echo "   Deployer:    $SOURCE"
echo "   Alias:       $ALIAS"
echo "   Contract ID: $CONTRACT_ID"
echo "---------------------------------------------"
