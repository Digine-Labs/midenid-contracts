use clap::{Parser, Subcommand};
use midenname_contracts::accounts::{create_deployer_account, create_naming_account};
use midenname_contracts::client::initiate_client;
use midenname_contracts::scripts::deploy;

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

    /// Initialize the deployed registry with owner and payment token
    Init {
        /// Owner account ID
        #[arg(long)]
        owner: Option<String>,
    },

    /// Register a new name
    Register {
        /// Name to register
        #[arg(long)]
        name: String,

        /// Account ID to map the name to
        #[arg(long)]
        account: Option<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Deploy => {
            println!("Deploying Miden Name Registry contract...\n");
            println!("=================================================");
            println!("Deleting existing store & keystore (store.sqlite3)");
            let _ = std::fs::remove_file("store.sqlite3");
            let _ = std::fs::remove_dir("keystore");
            println!("Deletion complete.");
            println!("=================================================");
            let mut keystore = midenname_contracts::client::create_keystore()?;
            let mut client = initiate_client(keystore.clone()).await?;

            // Define all account IDs here
            let deployer_account = create_deployer_account(&mut client, &mut keystore).await?;
            let naming_account = create_naming_account(&mut client, false).await?;
            // deploy contracts
            deploy(
                &mut client,
                &mut keystore,
                deployer_account.id(),
                naming_account.id(),
            )
            .await?;
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
        Commands::Register { name, account } => {
            println!("Registering name: {}", name);
            if let Some(account_id) = account {
                println!("Account: {}", account_id);
                // TODO: Implement registration logic
                println!("Note: Registration logic not yet implemented");
            } else {
                println!("Error: --account is required for registration");
            }
        }
    }

    Ok(())
}
