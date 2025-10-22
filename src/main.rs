use clap::{Parser, Subcommand};
use midenname_contracts::scripts;

/// Miden Name Registry Deployment CLI
#[derive(Parser)]
#[command(name = "midenname-contracts")]
#[command(about = "Deploy and manage Miden Name Registry contracts", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initializes keystore
    InitKeystore,
    /// Deploy all
    DeployAll,
    /// Set prices on the pricing contract
    SetPrices,
    /// Clean keystore and database
    Clean,
    /// Show current configuration
    Config,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::InitKeystore => scripts::initialize_keystore().await?,
        Commands::DeployAll => scripts::deploy_all().await?,
        Commands::SetPrices => scripts::set_prices().await?,
        Commands::Clean => scripts::clean().await?,
        Commands::Config => scripts::show_config().await?
    }

    Ok(())
}
