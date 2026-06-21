use alloy::primitives::{U256, address};
use alloy::{eips::BlockId, providers::ProviderBuilder};
use dotenv::dotenv;
use eyre::{Result, eyre};
use revm::DatabaseRef;
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
    let alloy_db = WrapDatabaseAsync::new(AlloyDB::new(provider, BlockId::number(22600000)))
        .ok_or_else(|| eyre!("failed to build AlloyDB"))?;

    let mut cache_db = CacheDB::new(alloy_db);

    let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
    let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
    let pair = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");
    let my_addr = address!("0000000000000000000000000000000000000001");

    let usdc_in = U256::from(50_000u64) * U256::from(10u64).pow(U256::from(6u64));
    let slot = balance_slot(my_addr, 9); // Using 9 as it's a storage slot for balances in USDC smart contract
    cache_db.insert_account_storage(usdc, slot, usdc_in)?;

    let (r0_before, r1_before) = read_reserves(&mut cache_db, pair)?;
    println!(
        "BEFORE  USDC reserve: {}  WETH reserve: {}",
        r0_before.to::<u128>() as f64 / 1e6,
        r1_before.to::<u128>() as f64 / 1e18
    );

    let weth_out = get_amount_out(usdc_in, r0_before, r1_before);
    println!("Expected weth out: {}", weth_out.to::<u128>() as f64 / 1e18);

    let slot0 = cache_db.storage_ref(weth, U256::from(0))?;
    println!("Slot 0 (name): {:#x}", slot0);

    let balance = read_balance(&mut cache_db, weth, my_addr)?;
    println!("Balance of {my_addr} is {balance} WETH");

    Ok(())
}
