# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Miden ID contracts project built on the Miden rollup blockchain. It implements:

- **Miden ID Registry**: A name resolution system that maps human-readable names to Miden addresses
- **Smart Contracts**: Written in Miden Assembly (.masm) for on-chain execution
- **Testing Infrastructure**: Rust code for contract testing and validation only

## Architecture

### Core Components

- `masm/accounts/`: Smart contract implementations in Miden Assembly
  - `miden_id.masm`: The main registry contract with name-to-address mapping
  - `miden_id_registry.masm`: Deprecated old contract, do not use this one
- `masm/scripts/`: Transaction scripts for contract interactions
- `masm/notes/`: Note scripts for cross-account communication
- `src/`: Rust testing utilities (for testing contracts only)
  - `src/main.rs`: Test runner and contract deployment demos
  - `src/common.rs`: Testing utilities for client instantiation and contract creation


### Key Architecture Patterns

- **Public Immutable Contracts**: Main contracts are deployed as public and immutable for transparency
- **NoAuth Component**: Contracts use no-auth authentication to allow public access
- **Storage Slots**: Registry uses specific storage slots for different data types:
  - SLOT[0]: Initialization flag
  - SLOT[1-2]: Owner data (prefix/suffix)
  - SLOT[3]: Fee configuration
  - SLOT[10]: Name-to-address mapping
  - SLOT[11]: Address-to-name reverse mapping

## Common Commands

### Contract Development
Primary development work is done in Miden Assembly (.masm) files. The Rust code is only for testing.

### Testing Commands

```bash
# Run contract tests
cargo test --release -- --nocapture --test-threads=1

# Run test demos (deploys contracts and executes transactions for validation)
cargo run

# Format Rust test code
cargo fmt

# Check Rust test code for issues
cargo check

# Run clippy on Rust test code
cargo clippy
```

## Miden Assembly Development

### Contract Structure
- Contracts are written in `.masm` files using Miden Assembly syntax
- Each contract defines exports for public functions
- Storage is managed through numbered slots
- Contracts can import other contracts as libraries

### Testing Integration
- Rust test infrastructure validates contract functionality
- Tests deploy contracts to Miden testnet
- Uses keystore in `./keystore/` directory
- Uses SQLite database `store.sqlite3` for local state
- Automatically cleans and recreates storage between test runs

## File Structure Notes

- All `.masm` files contain Miden Assembly smart contract code
- The project follows Miden's account-based architecture
- Storage is managed through numbered slots with specific purposes
- Notes enable asynchronous communication between accounts