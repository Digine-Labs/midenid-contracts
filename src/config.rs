use anyhow::{anyhow, Result};
use std::env;

#[derive(Debug, Clone)]
pub enum Network {
    Testnet,
    Mainnet,
}

impl Network {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "testnet" => Ok(Network::Testnet),
            "mainnet" => Ok(Network::Mainnet),
            _ => Err(anyhow!(
                "Invalid network: '{}'. Must be 'testnet' or 'mainnet'",
                s
            )),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Network::Testnet => "testnet",
            Network::Mainnet => "mainnet",
        }
    }
}

#[derive(Debug, Clone)]
pub struct DeploymentConfig {
    pub network: Network,
    pub initial_price: u64,
    pub contract_address: Option<String>,
}

impl DeploymentConfig {
    /// Load configuration from environment variables
    ///
    /// Required environment variables:
    /// - MIDEN_NETWORK: "testnet" or "mainnet"
    /// - INITIAL_PRICE: Initial registration price (u64)
    ///
    /// Optional environment variables:
    /// - CONTRACT_ADDRESS: Existing contract address (if deploying to existing contract)
    pub fn from_env() -> Result<Self> {
        // Load .env file if it exists (this won't error if file doesn't exist)
        let _ = dotenvy::dotenv();

        let network = env::var("MIDEN_NETWORK")
            .map_err(|_| anyhow!("MIDEN_NETWORK environment variable not set"))?;
        let network = Network::from_str(&network)?;

        let initial_price = env::var("INITIAL_PRICE")
            .map_err(|_| anyhow!("INITIAL_PRICE environment variable not set"))?;
        let initial_price: u64 = initial_price
            .parse()
            .map_err(|_| anyhow!("INITIAL_PRICE must be a valid u64 number"))?;

        let contract_address = env::var("CONTRACT_ADDRESS").ok();

        Ok(DeploymentConfig {
            network,
            initial_price,
            contract_address,
        })
    }

    /// Print configuration details
    pub fn print(&self) {
        println!("📋 Deployment Configuration:");
        println!("   Network: {}", self.network.as_str());
        println!("   Initial Price: {}", self.initial_price);
        if let Some(ref addr) = self.contract_address {
            println!("   Contract Address: {}", addr);
        } else {
            println!("   Contract Address: <will be deployed>");
        }
        println!();
    }
}
