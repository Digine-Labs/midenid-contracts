# Miden Name Registry

A decentralized name resolution system built on [Miden](https://miden.xyz), enabling human-readable names to map to Miden account addresses with advanced features like referrals, domain expiry, and revenue management.

🌐 **Website**: [miden.name](https://miden.name)

## Overview

Miden Name Registry is a smart contract system that provides ENS-like functionality for the Miden ecosystem. Users can register unique names that resolve to their Miden account addresses, making it easier to interact with accounts using memorable names instead of complex account IDs.

## Current Status

Miden Name is actively developed and any functionality can be changed, removed, or updated during development. The current version includes a comprehensive registry with domain ownership, transfers, expiry management, referral system, and revenue tracking.

### Key Features

- **Bidirectional Mapping**: Maps names to account IDs and vice versa
- **Domain Ownership**: Separate domain ownership from account mapping (requires activation)
- **Domain Expiry**: Domains expire after registration period (1-10 years)
- **Domain Extension**: Owners can extend domain registration before expiry
- **Transferable Names**: Domain owners can transfer ownership to other accounts
- **Multiple Names Per Account**: Accounts can own unlimited domains
- **Dynamic Pricing**: Registration fee depends on domain length
- **Discount System**: Multi-year registrations get discounts (3+ years: 30%, 5+ years: 50%)
- **Referral System**: Referrers earn a percentage of registration fees
- **Revenue Tracking**: Protocol tracks total and claimable revenue per token
- **Owner Controls**: Registry owner can update prices, set referral rates, and claim revenue
- **Expired Domain Cleanup**: Permissionless function to clear expired domain mappings

If you are learning Miden as a developer, you can find practices for the following examples:
- Account-based smart contracts with storage maps
- Note-based transaction system
- Ownership and access control patterns
- Payment validation and asset handling
- Time-based logic (domain expiry)
- Referral and revenue distribution systems
- Storage optimization techniques

## Architecture

### Smart Contracts (Miden Assembly)

All core logic is implemented in Miden Assembly (`.masm` files):

#### Accounts

- **[naming.masm](masm/accounts/naming.masm)**: Main name registry contract
  - Storage slots (see Storage Layout section below)
  - Exports: `register`, `register_with_referrer`, `activate_domain`, `transfer`, `extend_domain`, `clear_expired_domain`, `init`, `receive_asset`, `update_registry_owner`, `set_price`, `set_referrer_rate`, `claim_protocol_revenue`

- **[identity.masm](masm/accounts/identity.masm)**: Identity contract for user profiles (under development)

#### Notes

Note scripts enable cross-account interactions and contract calls:

- **[initialize_naming.masm](masm/notes/initialize_naming.masm)**: Initializes naming registry with owner and year timestamp
- **[register_name.masm](masm/notes/register_name.masm)**: Register a new domain with payment
- **[register_with_referrer.masm](masm/notes/register_with_referrer.masm)**: Register with referral code
- **[activate_domain.masm](masm/notes/activate_domain.masm)**: Activate domain mapping to account ID
- **[transfer_domain.masm](masm/notes/transfer_domain.masm)**: Transfer domain ownership to another account
- **[extend_domain.masm](masm/notes/extend_domain.masm)**: Extend domain registration period
- **[clear_expired_domain.masm](masm/notes/clear_expired_domain.masm)**: Clear expired domain mappings
- **[set_all_prices.masm](masm/notes/set_all_prices.masm)**: Set prices for all domain lengths
- **[set_all_prices_testnet.masm](masm/notes/set_all_prices_testnet.masm)**: Set test prices for testnet
- **[set_referrer_rate.masm](masm/notes/set_referrer_rate.masm)**: Set referral commission rate
- **[claim_protocol_revenue.masm](masm/notes/claim_protocol_revenue.masm)**: Claim accumulated protocol revenue
- **[transfer_ownership.masm](masm/notes/transfer_ownership.masm)**: Transfer registry ownership
- **[P2N.masm](masm/notes/P2N.masm)**: Pay-to-note for payment handling

#### Auth

- **[no_auth.masm](masm/auth/no_auth.masm)**: No-auth authentication component for public access

### Testing Infrastructure (Rust)

The `src/` and `tests/` directories contain Rust code for testing, deployment, and utilities:

#### Source Modules

- **[src/client.rs](src/client.rs)**: Client initialization and keystore management
- **[src/accounts.rs](src/accounts.rs)**: Account creation utilities (deployer, naming contract)
- **[src/notes.rs](src/notes.rs)**: Note creation utilities for contract interactions
- **[src/transaction.rs](src/transaction.rs)**: Transaction waiting and status checking
- **[src/scripts.rs](src/scripts.rs)**: Deployment scripts for the registry
- **[src/domain.rs](src/domain.rs)**: Domain name encoding/decoding functions
- **[src/storage.rs](src/storage.rs)**: Storage slot definitions for contract initialization

#### Test Files

- **[tests/test_utils.rs](tests/test_utils.rs)**: Shared test utilities and helpers
- **[tests/encoding_test.rs](tests/encoding_test.rs)**: Domain encoding/decoding validation
- **[tests/naming_register_tests.rs](tests/naming_register_tests.rs)**: Domain registration tests
- **[tests/naming_transfer_tests.rs](tests/naming_transfer_tests.rs)**: Domain transfer tests
- **[tests/naming_referral_tests.rs](tests/naming_referral_tests.rs)**: Referral system tests
- **[tests/naming_protocol_tests.rs](tests/naming_protocol_tests.rs)**: Protocol-level functionality tests

## Getting Started

### Prerequisites

- Rust toolchain (1.70+)
- Miden client dependencies
- Access to Miden testnet or local node

### Installation

```bash
# Clone the repository
git clone <repository-url>
cd midenid-contracts

# Build the project
cargo build --release
```

### Running Tests

Tests run against the Miden testnet by default. Tests must run sequentially due to shared SQLite state.

```bash
# Run all tests
cargo test --release -- --nocapture --test-threads=1

# Run specific test file
cargo test --release --test naming_register_tests -- --nocapture --test-threads=1

# Run single test
cargo test --release --test naming_register_tests -- --nocapture --test-threads=1 test_register_name
```

### CLI Usage

The project includes a CLI for deployment and management:

```bash
# Show available commands
cargo run -- --help

# Deploy the naming contract
cargo run -- deploy

# Initialize the registry (planned)
cargo run -- init --owner <owner_account_id>

# Register a name (planned)
cargo run -- register --name alice --account <account_id>
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

## Storage Layout

The naming contract uses Miden's storage system with numbered slots:

| Slot | Content | Type | Description |
|------|---------|------|-------------|
| 0 | Initialization flag | Value | 0 = uninitialized, 1 = initialized |
| 1 | Owner account | Value | Registry owner's account ID |
| 2 | Prices | Map | `[0, letter_count, token_prefix, token_suffix] -> price` |
| 3 | Account→Domain mapping | Map | Account ID to owned domain name |
| 4 | Domain→Account mapping | Map | Domain name to linked account ID |
| 5 | Domain→Owner mapping | Map | Domain name to owner account ID |
| 6 | Referral rate | Map | Referrer account to commission rate (basis points) |
| 7 | Referral total revenue | Map | Referrer account to total earned revenue |
| 8 | Referral claimed revenue | Map | Referrer account to claimed revenue |
| 9 | Domain count | Value | Total number of registered domains |
| 10 | Total revenue | Map | `[0, 0, token_prefix, token_suffix] -> total_amount` |
| 11 | Claimed revenue | Map | `[0, 0, token_prefix, token_suffix] -> claimed_amount` |
| 12 | Domain expiry dates | Map | Domain name to expiry timestamp |
| 13 | One year timestamp | Value | Number of seconds in one year (for calculations) |

## Contract Constraints

- **Maximum domain length**: 21 characters (alphanumeric: a-z, 0-9)
- **Minimum domain length**: 1 character
- **Multiple domains per account**: Accounts can own unlimited domains
- **Unique active domains**: Only one account can have an active mapping per domain
- **Registration period**: 1-10 years per registration
- **Owner-only operations**: Price updates, referral rates, ownership transfer, revenue claims
- **Domain ownership**: Registration creates ownership; activation creates account mapping
- **Expiry enforcement**: Expired domains can be cleared permissionlessly
- **Referral rate limit**: Maximum 25% (2500 basis points)
- **Discount tiers**: 3+ years = 30% off, 5+ years = 50% off

## Domain Lifecycle

1. **Registration**: User pays to register domain, becomes owner, domain starts inactive
2. **Activation**: Owner activates domain to link it to their account ID
3. **Active Period**: Domain resolves to owner's account, can be extended before expiry
4. **Expiry**: Domain expires after registration period ends
5. **Cleanup**: Anyone can call `clear_expired_domain` to remove expired mappings
6. **Re-registration**: Expired domain can be registered again by anyone

## Testing

Tests validate the following functionality:

- ✅ Registry initialization
- ✅ Domain registration with payment
- ✅ Domain activation and mapping
- ✅ Domain transfer between accounts
- ✅ Domain expiry and extension
- ✅ Expired domain cleanup
- ✅ Referral system and revenue distribution
- ✅ Multi-year discounts
- ✅ Protocol revenue tracking
- ✅ Owner controls (price updates, referral rates)
- ✅ Domain encoding/decoding
- ✅ Access control enforcement

## Resources

- [Miden Name](https://miden.name)
- [Miden Documentation](https://0xmiden.github.io/miden-docs/index.html)
- [Miden VM Documentation](https://0xmiden.github.io/miden-docs/imported/miden-vm/src/intro/main.html)
- [Miden Network](https://miden.xyz)