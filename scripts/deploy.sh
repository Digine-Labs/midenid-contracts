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
echo "  2. Payment Token ID (faucet/token for registration fees)"
echo "  3. Registration Price (in tokens)"
echo "  4. Owner Account ID (optional - will auto-create if not provided)"
echo ""

# Check if parameters were provided
if [ $# -ge 3 ]; then
    NETWORK=$1
    TOKEN_ID=$2
    PRICE=$3
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
    read -p "Enter Payment Token ID (hex or bech32): " TOKEN_ID
    read -p "Enter Registration Price: " PRICE
    read -p "Enter Owner Account ID (leave empty to auto-create): " OWNER_ID
fi

echo ""
echo "Configuration:"
echo "  Network: $NETWORK"
echo "  Token:   $TOKEN_ID"
echo "  Price:   $PRICE"
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

if [ -z "$OWNER_ID" ]; then
    ./target/release/deploy "$NETWORK" "$TOKEN_ID" "$PRICE"
else
    ./target/release/deploy "$NETWORK" "$TOKEN_ID" "$PRICE" "$OWNER_ID"
fi

if [ $? -eq 0 ]; then
    echo ""
    echo "✅ Deployment completed successfully!"
    echo ""
    echo "Next steps:"
    echo "  1. Copy the Registry Contract ID from the output above"
    echo "  2. Update your frontend configuration:"
    echo "     - REGISTRY_CONTRACT_ID: [from output]"
    echo "     - PAYMENT_TOKEN_ID: $TOKEN_ID"
    echo "     - REGISTRATION_PRICE: $PRICE"
    echo ""
else
    echo ""
    echo "❌ Deployment failed!"
    exit 1
fi
