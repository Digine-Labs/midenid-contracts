use clap::{Parser, Subcommand};
use midenname_contracts::scripts::{deploy, register_name};

#[derive(Parser)]
#[command(name = "midenname-contracts")]
#[command(about = "Miden Name Registry CLI", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Deploy the naming contract to the network
    Deploy,

    /// Register a new name
    Register {
        /// Name to register (lowercase alphanumeric, max 20 chars)
        #[arg(long)]
        name: String,

        /// Naming contract account ID (hex)
        #[arg(long)]
        naming: String,

        /// Deployer/sender account ID (hex)
        #[arg(long)]
        deployer: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Deploy => {
            println!("Deploying Miden Name Registry contract...\n");
            deploy().await?;
        }
        Commands::Register {
            name,
            naming,
            deployer,
        } => {
            register_name(name, naming, deployer).await?;
        }
    }

    Ok(())
}
