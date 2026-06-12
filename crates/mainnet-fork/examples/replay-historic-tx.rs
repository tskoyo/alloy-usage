use std::str::FromStr;

use alloy::consensus::Transaction;
use alloy::eips::BlockId;
use alloy::network::TransactionResponse;
use alloy::primitives::{B256, TxKind};
use alloy::providers::{Provider, ProviderBuilder};
use dotenv::dotenv;
use eyre::{Result, eyre};
use revm::context::TxEnv;
use revm::context::result::ExecutionResult;
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};
use revm::{Context, ExecuteEvm, MainBuilder, MainContext};

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let rpc_url = std::env::var("ALCHEMY_RPC_URL").expect("ALCHEMY_RPC_URL must be set in .env");
    let provider = ProviderBuilder::new().connect(&rpc_url).await?;

    // 1. Fetch the real historical tx
    let tx_hash =
        B256::from_str("0xa1e3a2d09160a0a9e41fa95537b9aeedf64a1c316dea603b00ef57a4bf384785")?;
    let real_tx = provider
        .get_transaction_by_hash(tx_hash)
        .await?
        .ok_or_else(|| eyre!("Tx not found!"))?;

    // 2. Fork at block - 1 to ensure the tx is included in the forked state
    let block_number = real_tx
        .block_number
        .ok_or_else(|| eyre!("Block not found!"))?;

    let alloy_db =
        WrapDatabaseAsync::new(AlloyDB::new(provider, BlockId::number(block_number - 1)))
            .ok_or_else(|| eyre!("failed to build AlloyDB"))?;
    let cache_db = CacheDB::new(alloy_db);

    // let pair = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");
    // let encoded = getReservesCall::new(()).abi_encode();

    // 3. Replay the tx's calldata against the forked state

    let tx = TxEnv {
        caller: real_tx.from(),
        kind: match real_tx.to() {
            Some(to) => TxKind::Call(to),
            None => TxKind::Create,
        },
        data: real_tx.input().clone(),
        value: real_tx.value(),
        gas_limit: real_tx.gas_limit(),
        gas_price: TransactionResponse::gas_price(&real_tx).unwrap_or_default(),
        nonce: real_tx.nonce(),
        ..Default::default()
    };

    // 4. Execute the tx
    let mut evm = Context::mainnet().with_db(cache_db).build_mainnet();
    let exec_result = evm.transact_one(tx)?;

    match exec_result {
        ExecutionResult::Success {
            reason,
            gas,
            logs,
            output,
        } => {
            println!("Tx replay executed successfully!");
            println!("Reason: {reason:?}");
            println!("Gas used: {gas}");
            for (i, log) in logs.iter().enumerate() {
                println!("Log {i}: address={:?}, data={:?}", log.address, log.data);
            }
            println!("Output: {output:?}");
        }
        other => {
            println!("Tx replay execution failed: {other:?}");
        }
    };

    Ok(())
}
