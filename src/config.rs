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
    pub price_1_letter: u64,
    pub price_2_letter: u64,
    pub price_3_letter: u64,
    pub price_4_letter: u64,
    pub price_5_letter: u64,
    pub pricing_setter_account: String,
    pub naming_owner_account: String,
    pub naming_treasury_account: String,
    pub pricing_token_address: String,
    pub deployer_account: String,
}

impl DeploymentConfig {
    /// Load configuration from environment variables
    ///
    /// Required environment variables:
    /// - MIDEN_NETWORK: "testnet" or "mainnet"
    /// - PRICE_1_LETTER: Price for 1-letter names (u64)
    /// - PRICE_2_LETTER: Price for 2-letter names (u64)
    /// - PRICE_3_LETTER: Price for 3-letter names (u64)
    /// - PRICE_4_LETTER: Price for 4-letter names (u64)
    /// - PRICE_5_LETTER: Price for 5+ letter names (u64)
    /// - PRICING_SETTER_ACCOUNT: Account authorized to set pricing
    /// - NAMING_OWNER_ACCOUNT: Owner account for the naming contract
    /// - NAMING_TREASURY_ACCOUNT: Treasury account for receiving payments
    pub fn from_env() -> Result<Self> {
        // Load .env file if it exists (this won't error if file doesn't exist)
        let _ = dotenvy::dotenv();

        let network = env::var("MIDEN_NETWORK")
            .map_err(|_| anyhow!("MIDEN_NETWORK environment variable not set"))?;
        let network = Network::from_str(&network)?;

        let price_1_letter = env::var("PRICE_1_LETTER")
            .map_err(|_| anyhow!("PRICE_1_LETTER environment variable not set"))?
            .parse()
            .map_err(|_| anyhow!("PRICE_1_LETTER must be a valid u64 number"))?;

        let price_2_letter = env::var("PRICE_2_LETTER")
            .map_err(|_| anyhow!("PRICE_2_LETTER environment variable not set"))?
            .parse()
            .map_err(|_| anyhow!("PRICE_2_LETTER must be a valid u64 number"))?;

        let price_3_letter = env::var("PRICE_3_LETTER")
            .map_err(|_| anyhow!("PRICE_3_LETTER environment variable not set"))?
            .parse()
            .map_err(|_| anyhow!("PRICE_3_LETTER must be a valid u64 number"))?;

        let price_4_letter = env::var("PRICE_4_LETTER")
            .map_err(|_| anyhow!("PRICE_4_LETTER environment variable not set"))?
            .parse()
            .map_err(|_| anyhow!("PRICE_4_LETTER must be a valid u64 number"))?;

        let price_5_letter = env::var("PRICE_5_LETTER")
            .map_err(|_| anyhow!("PRICE_5_LETTER environment variable not set"))?
            .parse()
            .map_err(|_| anyhow!("PRICE_5_LETTER must be a valid u64 number"))?;

        let pricing_setter_account = env::var("PRICING_SETTER_ACCOUNT")
            .map_err(|_| anyhow!("PRICING_SETTER_ACCOUNT environment variable not set"))?;

        let naming_owner_account = env::var("NAMING_OWNER_ACCOUNT")
            .map_err(|_| anyhow!("NAMING_OWNER_ACCOUNT environment variable not set"))?;

        let naming_treasury_account = env::var("NAMING_TREASURY_ACCOUNT")
            .map_err(|_| anyhow!("NAMING_TREASURY_ACCOUNT environment variable not set"))?;

        let pricing_token_address = env::var("PRICING_TOKEN_ADDRESS")
            .map_err(|_| anyhow!("PRICING_TOKEN_ADDRESS environment variable not set"))?;

        let deployer_account = env::var("DEPLOYER_ACCOUNT")
            .map_err(|_| anyhow!("PRICING_TOKEN_ADDRESS environment variable not set"))?;

        Ok(DeploymentConfig {
            network,
            price_1_letter,
            price_2_letter,
            price_3_letter,
            price_4_letter,
            price_5_letter,
            pricing_setter_account,
            naming_owner_account,
            naming_treasury_account,
            pricing_token_address,
            deployer_account
        })
    }

    /// Get pricing setter account as &str
    pub fn pricing_setter_account(&self) -> &str {
        &self.pricing_setter_account
    }

    /// Get naming owner account as &str
    pub fn naming_owner_account(&self) -> &str {
        &self.naming_owner_account
    }

    /// Get naming treasury account as &str
    pub fn naming_treasury_account(&self) -> &str {
        &self.naming_treasury_account
    }

    pub fn pricing_token_address(&self) -> &str {
        &self.pricing_token_address
    }

    pub fn deployer_account(&self) -> &str {
        &self.deployer_account
    }

    /// Print configuration details
    pub fn print(&self) {
        println!("📋 Deployment Configuration:");
        println!("   Network: {}", self.network.as_str());
        println!("   Pricing:");
        println!("     1-letter names: {}", self.price_1_letter);
        println!("     2-letter names: {}", self.price_2_letter);
        println!("     3-letter names: {}", self.price_3_letter);
        println!("     4-letter names: {}", self.price_4_letter);
        println!("     5+ letter names: {}", self.price_5_letter);
        println!("   Pricing Setter Account: {}", self.pricing_setter_account);
        println!("   Naming Owner Account: {}", self.naming_owner_account);
        println!("   Naming Treasury Account: {}", self.naming_treasury_account);
        println!();
    }
}
