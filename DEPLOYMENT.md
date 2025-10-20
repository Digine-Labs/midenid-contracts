# Miden ID Registry - Deployment Guide

## Overview

This guide explains how to deploy the Miden ID Registry contract to the Miden network.

## NEW: Environment-Based Configuration

The main binary now supports environment-based configuration for easier deployment automation.

### Setup

1. Copy the example environment file:
   ```bash
   cp .env.example .env
   ```

2. Edit `.env` with your configuration:
   ```env
   MIDEN_NETWORK=testnet          # or mainnet
   INITIAL_PRICE=100              # Registration price
   # CONTRACT_ADDRESS=0x...       # Optional: for existing contracts
   ```

### Running with Environment Config

Simply run:
```bash
cargo run --release
```

Or for development:
```bash
cargo run
```

The tool will:
- Load configuration from `.env` file
- Validate all settings
- Display configuration before deployment
- Deploy to the specified network

**Recommendation**: Use `--release` for production deployments (faster and optimized).

---

## Prerequisites

1. **Miden Node Running**: Ensure you have a Miden node running locally or access to a testnet/devnet endpoint
2. **Rust Installed**: Version 1.75 or higher
3. **Clean State**: The deployment script will use `./store.sqlite3` and `./keystore/` directories

## Quick Deployment

### Command Syntax

```bash
cargo run --release --bin deploy -- <network> <payment_token_id> <price> [owner_account_id]
```

**Parameters:**
- `network`: Target network (`devnet`, `testnet`, or `mainnet`)
- `payment_token_id`: Faucet/token ID for payments (hex: `0x...` OR bech32: `mm1...`/`mtst...`/`mdev...`)
- `price`: Registration price in tokens (e.g., `10`, `100`)
- `owner_account_id`: (Optional) Existing owner account ID, or omit to auto-create

### Example 1: Auto-create Owner Account

```bash
cargo run --release --bin deploy -- testnet 0x97598f759deab5201e93e1aac55997 10
```

This will:
1. Deploy the registry contract
2. Create a new owner account automatically
3. Initialize registry with the specified payment token and price
4. Save the owner seed for backup

### Example 2: Use Existing Owner Account

```bash
cargo run --release --bin deploy -- testnet 0x97598f759deab5201e93e1aac55997 10 0x1c89546e3b82cd1012a9fe4853bc68
```

This will use an existing owner account that must be present in your local database.

### Example 3: Using Bech32 Addresses

```bash
cargo run --release --bin deploy -- testnet mtst1qwtv9a6d78tfjqs0fln24rze4v4qqqt2u5t 10 mtst1qqwgj4rw8wpv6yqj48lys5audpcqqykld75
```

Both hex and bech32 formats are supported for account IDs.

## Deployment Process

The deployment script performs the following steps:

### Step 1: Deploy Registry Contract
- Reads the contract code from `./masm/accounts/miden_id.masm`
- Deploys as a **public, immutable** contract
- Contract ID will be displayed

### Step 2: Create or Use Owner Account
- If no owner account ID is provided: Creates a new regular updatable account
- If owner account ID is provided: Uses the existing account from local database
- This account will own and control the registry
- Owner can update prices and manage the registry

### Step 3: Initialize Registry
- Uses the `./masm/notes/init.masm` note to initialize the registry
- Passes payment token and price as note inputs
- Sets the owner account and payment token configuration

### Step 4: Verify Deployment
- Reads contract storage to verify initialization
- Confirms all settings are correct

## Deployment Output

After successful deployment, you'll receive:

```
📋 Summary:

   Registry Contract:  0x[REGISTRY_CONTRACT_ID]
   Owner Account:      0x[OWNER_ACCOUNT_ID]
   Payment Token:      0x[PAYMENT_TOKEN_ID]
   Registration Price: [PRICE] tokens

💾 Deployment info saved to: ./deployments/[TIMESTAMP]_[REGISTRY_ID].txt
```

**IMPORTANT**:
- Deployment details are automatically saved to `./deployments/` directory
- If a new owner account was created, the seed phrase will be displayed and saved
- **Backup the owner account seed securely** - you need it to recover the account
- Save the Registry Contract ID for frontend integration

## Configuration

All configuration is now done via command-line arguments:

- **Network**: Specify `devnet`, `testnet`, or `mainnet` as the first argument
- **Payment Token**: Provide the faucet/token account ID (hex or bech32)
- **Registration Price**: Set the price as a number (e.g., `10`, `100`)
- **Owner Account**: Optionally provide an existing account ID, or let the script create one

No code changes needed for basic configuration!

## Frontend Integration

After deployment, update your frontend configuration with the contract IDs:

```javascript
// Example frontend config
const MIDEN_CONFIG = {
  registryContractId: "0x[REGISTRY_CONTRACT_ID]",
  paymentTokenId: "0x[PAYMENT_TOKEN_ID]",
  registrationPrice: 100,
};
```

The deployment info file in `./deployments/` includes a ready-to-use format:
```
REGISTRY_CONTRACT_ID="0x..."
PAYMENT_TOKEN_ID="0x..."
REGISTRATION_PRICE=100
```

**WARNING**: This will create new contract IDs. Update your frontend accordingly.

**Note**: Each deployment creates a record in `./deployments/` for tracking.


