use alloy::{eips::BlockId, providers::ProviderBuilder};
use eyre::{Result, eyre};
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};

pub async fn call() -> Result<impl revm::Database> {
    let rpc_url = std::env::var("ALCHEMY_RPC_URL").expect("ALCHEMY_RPC_URL must be set in .env");
    let provider = ProviderBuilder::new().connect(&rpc_url).await?;

    let alloy_db = WrapDatabaseAsync::new(AlloyDB::new(provider, BlockId::number(22600000 - 1)))
        .ok_or_else(|| eyre!("failed to build AlloyDB"))?;
    let cache_db = CacheDB::new(alloy_db);

    Ok(cache_db)
}
