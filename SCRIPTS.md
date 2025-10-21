# Deployment Scripts

This project uses a CLI-based script system for deployment and management tasks.

## Available Commands

Run scripts using: `cargo run -- <command>`

### Configuration

```bash
# Show current deployment configuration
cargo run -- config
```

### Deployment

```bash
# Deploy the pricing contract only
cargo run -- deploy-pricing

# Deploy the naming contract only
cargo run -- deploy-naming

# Run full deployment sequence (both contracts + initialization)
cargo run -- deploy-all
```

### Initialization

```bash
# Initialize the pricing contract
cargo run -- init-pricing

# Set prices on the pricing contract
cargo run -- set-prices
```

### Utilities

```bash
# Clean keystore and database
cargo run -- clean

# Show help and available commands
cargo run -- --help
```

## How It Works

The script system is built with:
- **[main.rs](src/main.rs)**: CLI entry point using `clap` for argument parsing
- **[scripts.rs](src/scripts.rs)**: Individual script implementations

Each command calls a specific async function in the scripts module, allowing you to run targeted deployment tasks without executing the entire deployment flow.

## Adding New Scripts

To add a new script:

1. Add a new variant to the `Commands` enum in [main.rs](src/main.rs):
   ```rust
   #[derive(Subcommand)]
   enum Commands {
       // ... existing commands
       /// Your new command description
       MyNewCommand,
   }
   ```

2. Add a match arm in `main()`:
   ```rust
   match cli.command {
       // ... existing matches
       Commands::MyNewCommand => scripts::my_new_command().await?,
   }
   ```

3. Implement the function in [scripts.rs](src/scripts.rs):
   ```rust
   pub async fn my_new_command() -> anyhow::Result<()> {
       println!("\n🚀 Running My New Command\n");
       // Your implementation here
       Ok(())
   }
   ```

## Environment Variables

Configure deployment via environment variables (or `.env` file):

- `MIDEN_NETWORK`: Network to deploy to (testnet/mainnet)
- `NAMING_OWNER_ACCOUNT`: Owner account for naming contract
- `NAMING_TREASURY_ACCOUNT`: Treasury account for naming contract
- `PRICING_SETTER_ACCOUNT`: Account that can set prices
- `PRICING_TOKEN_ADDRESS`: Token used for registration fees
- `PRICE_1_LETTER`, `PRICE_2_LETTER`, etc.: Registration prices

See [config.rs](src/config.rs) for full configuration details.
