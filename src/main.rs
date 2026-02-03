use clap::{Parser, Subcommand};
use midenname_contracts::scripts::{
    consume_single_note, deploy, find_consumable_notes, send_register_note,
};

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
    Deploy {
        /// Is network account
        #[arg(long)]
        as_network: bool,
    },

    /// Initialize the deployed registry with owner and payment token
    Init {
        /// Owner account ID
        #[arg(long)]
        owner: Option<String>,
    },

    /// Register a new name
    Register {
        /// Note sender (Domain registerer) Account Id
        #[arg(long)]
        account: String,

        /// Naming contract account id
        #[arg(long)]
        naming_account: String,

        /// Faucet id to fund the registration
        #[arg(long)]
        faucet_id: String,

        /// Name to register
        #[arg(long)]
        name: String,
    },

    /// Consume Note
    ConsumeNote {
        /// Note ID to consume
        #[arg(long)]
        note_id: String,

        /// Naming contract account id
        #[arg(long)]
        naming_account_id: String,
    },

    /// Find and consume notes
    FindAndConsumeNotes {
        /// Account id
        #[arg(long)]
        account: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Deploy { as_network } => {
            println!("Deploying Miden Name Registry contract...\n");
            deploy(as_network).await?;
        }
        Commands::Init { owner } => {
            println!("Initializing registry...");
            if let Some(owner_id) = owner {
                println!("Owner: {}", owner_id);
                // TODO: Implement initialization logic
                println!("Note: Initialization logic not yet implemented");
            } else {
                println!("Error: --owner is required for initialization");
            }
        }
        Commands::Register {
            name,
            account,
            naming_account,
            faucet_id,
        } => {
            println!("\n");
            println!("=================================================");
            println!("Registering name: {}", name);
            println!(
                "Entered account needs to be funded and added to keystore to send the registration note."
            );
            send_register_note(account, naming_account, faucet_id, name).await?;
        }
        Commands::ConsumeNote {
            note_id,
            naming_account_id,
        } => {
            println!("Consuming note with ID: {}", note_id);
            consume_single_note(note_id, naming_account_id).await?;
        }
        Commands::FindAndConsumeNotes { account } => {
            println!("Finding and consuming notes for account: {}", account);
            find_consumable_notes(account).await?
        }
    }

    Ok(())
}
