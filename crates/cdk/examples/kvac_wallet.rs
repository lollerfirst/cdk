use std::sync::Arc;
use cdk::cdk_database::WalletMemoryDatabase;
use cdk::wallet::Wallet;
use cdk_common::CurrencyUnit;
use rand::Rng;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate a random seed for the wallet
    let seed = rand::thread_rng().gen::<[u8; 32]>();

    // Mint URL and currency unit
    let mint_url = "http://127.0.0.1:3338";
    let unit = CurrencyUnit::Sat;

    // Initialize the memory store
    let localstore = WalletMemoryDatabase::default();

    // Create a new wallet
    let wallet = Wallet::new(mint_url, unit, Arc::new(localstore), &seed, None)?;


    // Request bootstrap coins
    let bootstrap_coins = wallet.bootstrap(None).await?;

    println!("bootstrap coins: {}", serde_json::to_string_pretty(&bootstrap_coins).unwrap());

    Ok(())
}
