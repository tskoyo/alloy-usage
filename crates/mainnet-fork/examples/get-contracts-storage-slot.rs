use alloy::primitives::{U256, address};
use alloy::{eips::BlockId, providers::ProviderBuilder};
use dotenv::dotenv;
use eyre::{Result, eyre};
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};

#[path = "common/mod.rs"]
mod common;
use common::*;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let rpc_url = std::env::var("ALCHEMY_RPC_URL").expect("ALCHEMY_RPC_URL must be set in .env");
    let provider = ProviderBuilder::new().connect(&rpc_url).await?;

    // Fork at a specific block. AlloyDB lazily fetches state from the provider.
    // WrapDatabaseAsync bridges the async provider into revm's sync Database trait.
    let alloy_db = WrapDatabaseAsync::new(AlloyDB::new(provider, BlockId::latest()))
        .ok_or_else(|| eyre!("failed to build AlloyDB"))?;
    let mut cache_db = CacheDB::new(alloy_db);

    let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
    let known_holder = address!("28C6c06298d514Db089934071355E5743bf21d60");
    let simulated_acc = address!("0x4c3F387806b6C474E5BC2E30Be061E11473fB7D9");

    let before = read_balance(&mut cache_db, usdc, known_holder)?;
    println!("BEFORE: {} USDC", before.to::<u128>() as f64 / 1e6);

    let simulated_balance = read_balance(&mut cache_db, usdc, simulated_acc)?;
    println!(
        "Simualted acc balance: {}",
        simulated_balance.to::<u128>() as f64 / 1e6
    );

    let slot = balance_slot(known_holder, 9);
    let new_value = U256::from(100u64) * U256::from(10).pow(U256::from(6)); // 100 usdc
    cache_db.insert_account_storage(usdc, slot, new_value)?;

    let after = read_balance(cache_db, usdc, known_holder)?;
    println!("AFTER {} USDC", after.to::<u128>() as f64 / 1e6);

    Ok(())
}
