# Miden Name Registry (MidenID)

A decentralized domain name system for the Miden blockchain ecosystem. Register human-readable names that resolve to Miden account addresses - similar to ENS on Ethereum.

## Features

- **Domain Registration**: Register unique domain names (1-21 characters, a-z and 0-9)
- **Domain Resolution**: Map domain names to Miden account addresses
- **Domain Transfer**: Transfer ownership of domains between accounts
- **Domain Extension**: Extend registration period before expiry
- **Referral System**: Earn commissions by referring new registrations
- **Multi-Year Discounts**: 30% off for 3+ years, 50% off for 5+ years
- **Protocol Revenue**: Automated revenue tracking and claiming

## Architecture

The project uses a dual-language architecture:

| Layer | Language | Purpose |
|-------|----------|---------|
| Smart Contracts | Miden Assembly | On-chain logic for registration, transfers, expiry |
| Infrastructure | Rust | CLI tools, deployment, testing, client management |

### Project Structure

```
midenid-contracts/
├── src/                          # Rust source code
│   ├── main.rs                   # CLI entry point
│   ├── accounts.rs               # Account creation utilities
│   ├── scripts.rs                # Deployment & transaction scripts
│   ├── notes.rs                  # Note creation for contract interaction
│   ├── domain.rs                 # Domain encoding/decoding
│   ├── storage.rs                # Storage slot definitions
│   ├── client.rs                 # Miden network client
│   └── utils.rs                  # Pricing utilities
├── masm/                         # Miden Assembly contracts
│   ├── accounts/
│   │   ├── naming.masm           # Main registry contract
│   │   ├── naming_unsafe.masm    # Unsafe version for testing
│   │   └── naming_discount.masm  # Discount-enabled version
│   ├── notes/
│   │   ├── register_name.masm    # Domain registration note
│   │   ├── activate_domain.masm  # Domain activation note
│   │   ├── transfer_domain.masm  # Domain transfer note
│   │   ├── extend_domain.masm    # Domain extension note
│   │   └── ...                   # Other note scripts
│   └── scripts/
│       └── init_on_chain.masm    # On-chain initialization
├── tests/                        # Test suite
│   ├── naming_register_tests.rs
│   ├── naming_referral_tests.rs
│   ├── naming_protocol_tests.rs
│   ├── naming_transfer_tests.rs
│   └── test_utils.rs
├── keystore/                     # Private key storage
├── .env                          # Configuration
└── Cargo.toml                    # Rust dependencies
```

## Prerequisites

- Rust 2024 edition
- Miden SDK dependencies (miden-client, miden-lib, miden-objects)
- Access to Miden testnet

## Installation

```bash
# Clone the repository
git clone https://github.com/diginelabs/midenid-contracts.git
cd midenid-contracts

# Build the project
cargo build --release
```

## Configuration

Create a `.env` file with the following variables:

```env
# Network configuration
MIDEN_NETWORK=testnet

# Pricing (in smallest token units)
PRICE_1_LETTER=375000000
PRICE_2_LETTER=200000000
PRICE_3_LETTER=120000000
PRICE_4_LETTER=55000000
PRICE_5_LETTER=20000000

# Account addresses
PRICING_SETTER_ACCOUNT=0x...
NAMING_OWNER_ACCOUNT=0x...
NAMING_TREASURY_ACCOUNT=0x...
PRICING_TOKEN_ADDRESS=0x...
```

## Usage

### Deploy Registry Contract

```bash
# Local deployment
cargo run -- deploy

# Deploy as network account (production)
cargo run -- deploy-network
```

### Initialize Registry

```bash
cargo run -- init --owner <owner_account_id>
```

### Register a Domain

```bash
cargo run -- register \
  --account <your_account_id> \
  --naming-account <registry_account_id> \
  --faucet-id <payment_token_id> \
  --name <domain_name>
```

### Consume Note

```bash
cargo run -- consume-note \
  --note-id <note_id> \
  --naming-account-id <registry_account_id>
```

## Domain Lifecycle

```
┌─────────────┐     ┌─────────────┐     ┌─────────────┐
│  Available  │────▶│  Registered │────▶│   Active    │
│             │     │  (Inactive) │     │             │
└─────────────┘     └─────────────┘     └─────────────┘
       ▲                                       │
       │                                       │
       │            ┌─────────────┐            │
       └────────────│   Expired   │◀───────────┘
                    │             │
                    └─────────────┘
```

1. **Available**: Domain not registered, can be claimed
2. **Registered (Inactive)**: Owner paid, but domain not linked to account
3. **Active**: Domain resolves to owner's account address
4. **Expired**: Registration period ended, can be cleared and re-registered

## Smart Contract Storage

| Slot | Name | Description |
|------|------|-------------|
| 0 | `init_flag` | 0 = uninitialized, 1 = initialized |
| 1 | `owner` | Registry owner account ID |
| 2 | `prices` | Price map by domain length and token |
| 3 | `account_to_domain` | Account ID → Domain name |
| 4 | `domain_to_account` | Domain name → Account ID |
| 5 | `domain_to_owner` | Domain name → Owner account ID |
| 6 | `referral_rate` | Referrer → Commission rate (basis points) |
| 7 | `referral_total` | Referrer → Total earned revenue |
| 8 | `referral_claimed` | Referrer → Claimed revenue |
| 9 | `domain_count` | Total registered domains |
| 10 | `total_revenue` | Protocol total revenue by token |
| 11 | `claimed_revenue` | Protocol claimed revenue by token |

## Contract Functions

### Public Functions

| Function | Description |
|----------|-------------|
| `register` | Register a domain with payment |
| `register_with_referrer` | Register with referral code |
| `activate_domain` | Activate domain to link to account |
| `transfer` | Transfer domain ownership |
| `extend_domain` | Extend registration period |
| `clear_expired_domain` | Remove expired domain mappings |
| `receive_asset` | Receive payment tokens |

### Owner-Only Functions

| Function | Description |
|----------|-------------|
| `init` | Initialize the registry |
| `set_price` | Update pricing for domain lengths |
| `set_referrer_rate` | Set referral commission rate |
| `update_registry_owner` | Transfer registry ownership |
| `claim_protocol_revenue` | Claim accumulated revenue |

## Domain Encoding

Domains are encoded using a custom scheme:
- **Allowed characters**: a-z (1-26), 0-9 (27-36)
- **Encoding**: 7 bits per character
- **Storage**: 4 Felts (Word) per domain
- **Maximum length**: 21 characters

```rust
// Example encoding
"alice" → [Felt, Felt, Felt, Felt]
```

## Pricing Structure

| Domain Length | Base Price | 3+ Years (-30%) | 5+ Years (-50%) |
|---------------|------------|-----------------|-----------------|
| 1 character | 375M | 262.5M | 187.5M |
| 2 characters | 200M | 140M | 100M |
| 3 characters | 120M | 84M | 60M |
| 4 characters | 55M | 38.5M | 27.5M |
| 5+ characters | 20M | 14M | 10M |

## Testing

```bash
# Run all tests (sequential due to shared SQLite state)
cargo test --release -- --nocapture --test-threads=1

# Run specific test file
cargo test --release --test naming_register_tests -- --nocapture --test-threads=1

# Run single test
cargo test --release --test naming_register_tests test_register_name -- --nocapture --test-threads=1
```

### Test Coverage

- Registry initialization
- Domain registration with payment validation
- Domain activation and resolution
- Domain transfer between accounts
- Domain expiry and extension
- Expired domain cleanup
- Referral system with commission tracking
- Multi-year discount calculations
- Protocol revenue accumulation
- Owner access controls

## Referral System

Referrers can earn commission on registrations:

1. **Set Referral Rate**: Owner sets commission rate (max 25%)
2. **Register with Referrer**: Users include referrer account during registration
3. **Commission Tracking**: Revenue automatically tracked per referrer
4. **Claim Revenue**: Referrers can claim accumulated earnings

```
Commission = Registration Price × Referral Rate (basis points) / 10000
```

## Deployed Contracts (Testnet)

| Contract | Address |
|----------|---------|
| Naming Registry | `0x177e66aab4a3704014a2db204f6d49` |
| Pricing Token (MIDEN) | `0xa62277459b84194055b0d69b449d38` |

## Development

```bash
# Check for compilation errors
cargo check

# Format code
cargo fmt

# Run linter
cargo clippy

# Build documentation
cargo doc --open
```

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| miden-client | 0.12 | Miden network client |
| miden-lib | 0.12 | Miden standard library |
| miden-objects | 0.12 | Miden data structures |
| miden-crypto | 0.18.2 | Cryptographic primitives |
| miden-assembly | 0.19.1 | MASM compiler |
| tokio | 1.46 | Async runtime |
| clap | 4.5 | CLI framework |
| dotenvy | 0.15 | Environment loading |

## License

[Add your license here]

## Contributing

[Add contribution guidelines here]

## Contact

- **Organization**: Digine Labs
- **GitHub**: [diginelabs](https://github.com/diginelabs)
