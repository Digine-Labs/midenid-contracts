# Miden Name Registry

A decentralized name resolution system built on the [Miden](https://miden.xyz), enabling human-readable names to map to Miden account addresses.

🌐 **Website**: [miden.name](https://miden.name)

## Overview

Miden Name Registry is a smart contract system that provides ENS-like functionality for the Miden ecosystem. Users can register unique names that resolve to their Miden account addresses, making it easier to interact with accounts using memorable names instead of complex account IDs.

## Current Status

Miden Name is actively developed and any functionality can be changed, removed, or updated during development. The initial version includes only a simple registry and access-controlled utilities.

### Key Features

- **Bidirectional Mapping**: Maps names to account IDs and vice versa
- **Multiple Payment Tokens**: Registration can be done by different tokens
- **Owner Controls**: Registry owner can update prices and transfer ownership
- **Unique Names**: Each name can only be registered once
- **Multiple Names**: Any accounts can own any amount of names
- **Transferable Names**: Name owners can transfer names to another accounts
- **Dynamic Pricing**: Registration fee depends on the length of the name
- **External Pricing Contract**: Name prices calculated by pricing contract. Each tokens have different pricing contract.

## Architecture

### Smart Contracts (Miden Assembly)

All core logic is implemented in Miden Assembly (`.masm` files):

#### Accounts
- **[naming.masm](masm/accounts/miden_id.masm)**: Main name registry contract
  - Storage slots:
    - `SLOT[0]`: Initialization flag
    - `SLOT[1]`: Owner account
    - `SLOT[2]`: Treasury account
    - `SLOT[3]`: Payment token to Pricing contract mapping
    - `SLOT[4]`: Account ID to Domain mapping
    - `SLOT[5]`: Domain to Account ID mapping
    - `SLOT[6]`: Domain to Domain owner mapping
    - `SLOT[7]`: Pricing contract to calculate_price procedure root mapping
- **[pricing.masm](masm/accounts/pricing.masm)**: Pricing contract that calculates price of a name. Each token should have different pricing contract instance.
- **[identity.masm](masm/accounts/identity.masm)**: Identity contract that stores users public identities (Under development)

#### Notes
- **[initialize_naming.masm](masm/notes/initialize_naming.masm)**: Initializes naming registry
- **[initialize_pricing.masm](masm/notes/initialize_pricing.masm)**: Initializes pricing contract
- **[register_name.masm](masm/notes/register_name.masm)**: Register a new name with payment
- **[transfer_domain.masm](masm/notes/transfer_domain.masm)**: Transfers domain to another account
- **[P2N.masm](masm/notes/P2N.masm)**: Pay-to-note for payment handling (TBD)

#### Scripts
- **[nop.masm](masm/scripts/nop.masm)**: No-operation script for testing

#### Auth
- **[no_auth.masm](masm/auth/no_auth.masm)**: No-auth authentication component for public access

### Testing Infrastructure (Rust)

The `src/` and `tests/` directories contain Rust code exclusively for testing and validating the Miden Assembly contracts:

- **[src/notes.rs](src/notes.rs)**: Note related utility functions that widely used on tests
- **[src/utils.rs](src/utils.rs)**: Utilities that widely used on tests

- **[tests/encoding_test.rs](tests/encoding_test.rs)**: Encoding and decoding of domain tests
- **[tests/naming_test.rs](tests/naming_test.rs)**: Naming registry tests
- **[tests/pricing_test.rs](tests/pricing_test.rs)**: Pricing tests

## Getting Started

### Prerequisites

- Rust toolchain (1.70+)
- Miden client dependencies
- Access to Miden testnet

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd midenid-contracts

# Build the project
cargo build --release
```

### Running Tests

Tests run against the Miden testnet by default. For local development, we provide automated scripts to set up and run a local Miden node.

#### Using Automated Scripts (Recommended for Local Development)

Miden Name Registry tests run on testnet. However, we have implemented scripts in the scripts folder to set up, start, and test a local node.The `scripts/` folder contains shell scripts to simplify local node setup and testing:

- **[setup_node.sh](scripts/setup_node.sh)**: Installs miden-node, creates required directories, and bootstraps a local node with genesis data
- **[start_node.sh](scripts/start_node.sh)**: Starts the local Miden node with RPC server on port 57291
- **[start_node_and_test.sh](scripts/start_node_and_test.sh)**: Complete automation script that starts the node, waits for it to be ready, runs all tests, and cleans up the node process automatically

```bash
# One-time setup: Install and bootstrap local node
bash scripts/setup_node.sh

# Option 1: Run tests with automatic node management (recommended)
bash scripts/start_node_and_test.sh

# Option 2: Manual node control
bash scripts/start_node.sh  # In one terminal
cargo test --release -- --nocapture --test-threads=1  # In another terminal
```

#### Manual Test Commands

```bash
# Run all tests (requires testnet or local node access)
cargo test --release -- --nocapture --test-threads=1

# Run specific test file
cargo test --release --test name_registration_test -- --nocapture --test-threads=1

# Run test demos
cargo run
```

### Development Commands

```bash
# Check code for issues
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy
```

## Deployment

For production deployment to testnet or mainnet, see the comprehensive **[DEPLOYMENT.md](DEPLOYMENT.md)** guide.

**Quick Start:**
```bash
cargo run --release --bin deploy -- <network> <payment_token_id> <price> [owner_account_id]
```

Example:
```bash
# Deploy to testnet with auto-created owner account
cargo run --release --bin deploy -- testnet 0x97598f759deab5201e93e1aac55997 10
```

The deployment script will:
- Deploy the registry contract as public, immutable
- Create or use an existing owner account
- Initialize the registry with payment token and price
- Save deployment info to `./deployments/` directory

## Usage Examples

### Deploying and Initializing Registry (Testing)

```rust
let mut helper = RegistryTestHelper::setup_with_deployed_contract().await?;
let owner_account = helper.create_account("Owner").await?;
let faucet_account = helper.create_faucet("REG", 8, 1_000_000).await?;

// Initialize with price of 100 tokens
helper.initialize_registry_with_faucet(&owner_account, Some(&faucet_account)).await?;
```

### Registering a Name

```rust
// Register name "alice" for user account
helper.register_name("alice", &user_account, payment_note).await?;

// Verify registration
let resolved_id = helper.get_account_for_name("alice").await?;
assert_eq!(resolved_id, Some(user_account.id()));
```

### Updating Price

```rust
// Owner updates registration price to 200
helper.update_price(&owner_account, 200).await?;

// Verify new price
let state = helper.get_contract_account_record().await?;
let price = helper.get_price(&state);
assert_eq!(price, 200);
```

## Storage Layout

The contract uses Miden's storage system with numbered slots:

| Slot | Content | Description |
|------|---------|-------------|
| 0 | Initialization flag | 0 = uninitialized, 1 = initialized |
| 1 | Owner prefix | First part of owner account ID |
| 2 | Owner suffix & Token | Owner suffix + payment token info |
| 3 | Name→ID mapping | Sparse Merkle Tree root for name lookups |
| 4 | ID→Name mapping | Sparse Merkle Tree root for reverse lookups |
| 5 | Registration price | Cost in fungible tokens to register |

## Contract Constraints

- **Maximum name length**: 20 characters
- **One name per account**: Each account can only register one name
- **Unique names**: Each name can only be registered once
- **Owner-only operations**: Price updates and ownership transfers require owner authentication
- **Payment validation**: Registration requires exact payment amount from specified token

## Testing

Tests run against Miden testnet and validate:

- ✅ Registry initialization and double-init prevention
- ✅ Name registration with payment validation
- ✅ Bidirectional mapping consistency
- ✅ Duplicate name rejection
- ✅ One-name-per-account enforcement
- ✅ Price update functionality
- ✅ Ownership transfer
- ✅ Payment token validation

## Resources

- [Miden Name](https://miden.name)
- [Miden Documentation](https://0xmiden.github.io/miden-docs/index.html)
- [Miden VM Documentation](https://0xmiden.github.io/miden-docs/imported/miden-vm/src/intro/main.html)
- [Miden](https://miden.xyz)


