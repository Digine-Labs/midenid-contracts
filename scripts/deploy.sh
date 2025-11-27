#!/bin/bash

# Get the directory where this script is located
SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"
# Get the project root (parent of scripts directory)
PROJECT_ROOT="$( cd "$SCRIPT_DIR/.." && pwd )"

# Change to project root
cd "$PROJECT_ROOT" || exit 1

echo "🚀 Miden ID Registry Deployment"
echo ""
echo "This script deploys the Miden ID Registry contract."
echo ""
echo "You will need to provide:"
echo "  1. Network (devnet, testnet, or mainnet)"
echo "  2. Registration Price (in tokens)"
echo "  3. Payment Token ID (optional - defaults to testnet faucet)"
echo "  4. Owner Account ID (optional - will auto-create if not provided)"
echo ""

# Default payment token
DEFAULT_TOKEN="0x97598f759deab5201e93e1aac55997"

# Check if parameters were provided
if [ $# -ge 2 ]; then
    NETWORK=$1
    PRICE=$2
    TOKEN_ID=${3:-$DEFAULT_TOKEN}
    OWNER_ID=${4:-""}
else
    # Prompt for parameters
    echo "Select network:"
    echo "  1. devnet"
    echo "  2. testnet"
    echo "  3. mainnet"
    read -p "Enter network choice (1-3): " NETWORK_CHOICE

    case $NETWORK_CHOICE in
        1) NETWORK="devnet";;
        2) NETWORK="testnet";;
        3) NETWORK="mainnet";;
        *) echo "Invalid choice. Defaulting to testnet."; NETWORK="testnet";;
    esac

    echo ""
    read -p "Enter Registration Price: " PRICE
    read -p "Enter Payment Token ID (leave empty for default testnet faucet): " TOKEN_ID
    if [ -z "$TOKEN_ID" ]; then
        TOKEN_ID=$DEFAULT_TOKEN
    fi
    read -p "Enter Owner Account ID (leave empty to auto-create): " OWNER_ID
fi

echo ""
echo "Configuration:"
echo "  Network: $NETWORK"
echo "  Price:   $PRICE"
if [ "$TOKEN_ID" = "$DEFAULT_TOKEN" ]; then
    echo "  Token:   $TOKEN_ID (default testnet faucet)"
else
    echo "  Token:   $TOKEN_ID (custom)"
fi
if [ -z "$OWNER_ID" ]; then
    echo "  Owner:   (will auto-create new account)"
else
    echo "  Owner:   $OWNER_ID"
fi
echo ""
read -p "Continue with deployment? (y/n) " -n 1 -r
echo ""

if [[ ! $REPLY =~ ^[Yy]$ ]]
then
    echo "Deployment cancelled."
    exit 1
fi

echo ""
echo "Building deployment binary..."
cargo build --release --bin deploy

if [ $? -ne 0 ]; then
    echo "❌ Build failed!"
    exit 1
fi

# Check if binary exists
if [ ! -f "./target/release/deploy" ]; then
    echo "❌ Error: Binary not found at ./target/release/deploy"
    echo "   Current directory: $(pwd)"
    echo "   Please run this script from the project root or use: ./scripts/deploy.sh"
    exit 1
fi

echo ""
echo "Running deployment..."
echo ""

# Capture deployment output to extract registry ID
DEPLOY_OUTPUT=""

# Build command with new parameter order: network price [token] [owner]
if [ -z "$OWNER_ID" ]; then
    if [ "$TOKEN_ID" = "$DEFAULT_TOKEN" ]; then
        # Use default token (don't pass it)
        DEPLOY_OUTPUT=$(./target/release/deploy "$NETWORK" "$PRICE" 2>&1)
    else
        # Custom token
        DEPLOY_OUTPUT=$(./target/release/deploy "$NETWORK" "$PRICE" "$TOKEN_ID" 2>&1)
    fi
else
    # With owner account
    DEPLOY_OUTPUT=$(./target/release/deploy "$NETWORK" "$PRICE" "$TOKEN_ID" "$OWNER_ID" 2>&1)
fi

DEPLOY_EXIT_CODE=$?

# Print the deployment output
echo "$DEPLOY_OUTPUT"

if [ $DEPLOY_EXIT_CODE -eq 0 ]; then
    # Extract registry contract ID from output
    REGISTRY_ID=$(echo "$DEPLOY_OUTPUT" | grep -oE "Registry Contract:\s+0x[a-f0-9]+" | awk '{print $3}')

    echo ""
    echo "✅ Deployment completed successfully!"
    echo ""
    echo "Next steps:"
    echo "  1. Use these values in your frontend configuration:"
    if [ -n "$REGISTRY_ID" ]; then
        echo "     - REGISTRY_CONTRACT_ID: $REGISTRY_ID"
    else
        echo "     - REGISTRY_CONTRACT_ID: [see output above]"
    fi
    echo "     - PAYMENT_TOKEN_ID: $TOKEN_ID"
    echo "     - REGISTRATION_PRICE: $PRICE"
    echo ""
else
    echo ""
    echo "❌ Deployment failed!"
    exit 1
fi
