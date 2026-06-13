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
    let approve_tx_hash =
        B256::from_str("0xdac3c655b8d035ae32fd44083bc99265674a5e5735c846d5a888959fc580a8b6")?;
    let swap_tx_hash =
        B256::from_str("0xa1e3a2d09160a0a9e41fa95537b9aeedf64a1c316dea603b00ef57a4bf384785")?;

    let approve_tx = provider
        .get_transaction_by_hash(approve_tx_hash)
        .await?
        .ok_or_else(|| eyre!("Tx not found"))?;
    let swap_tx = provider
        .get_transaction_by_hash(swap_tx_hash)
        .await?
        .ok_or_else(|| eyre!("Tx not found!"))?;

    // 2. Fork at block - 1 to ensure the tx is included in the forked state
    let block_number = swap_tx
        .block_number
        .ok_or_else(|| eyre!("Block not found!"))?;

    let alloy_db =
        WrapDatabaseAsync::new(AlloyDB::new(provider, BlockId::number(block_number - 1)))
            .ok_or_else(|| eyre!("failed to build AlloyDB"))?;
    let cache_db = CacheDB::new(alloy_db);

    // 3. Replay the tx's calldata against the forked state

    println!("Approve tx nonce is: {}", approve_tx.nonce());
    let approve_tx = TxEnv {
        caller: approve_tx.from(),
        kind: match approve_tx.to() {
            Some(to) => TxKind::Call(to),
            None => TxKind::Create,
        },
        data: approve_tx.input().clone(),
        value: approve_tx.value(),
        nonce: approve_tx.nonce(), // should be 7
        gas_limit: approve_tx.gas_limit(),
        gas_price: TransactionResponse::gas_price(&approve_tx).unwrap_or_default(),
        ..Default::default()
    };

    let swap_tx = TxEnv {
        caller: swap_tx.from(),
        kind: match swap_tx.to() {
            Some(to) => TxKind::Call(to),
            None => TxKind::Create,
        },
        data: swap_tx.input().clone(),
        value: swap_tx.value(),
        nonce: swap_tx.nonce(),
        gas_limit: swap_tx.gas_limit(),
        gas_price: TransactionResponse::gas_price(&swap_tx).unwrap_or_default(),
        ..Default::default()
    };

    // 4. Execute the tx
    let mut ctx = Context::mainnet().with_db(cache_db);
    ctx.cfg.disable_nonce_check = true;
    // ctx.cfg.disable_nonce_check = true; // Disable nonce check to allow replaying the tx
    let mut evm = ctx.build_mainnet();
    let approve_exec_result = evm.transact_one(approve_tx)?;
    let exec_result = evm.transact_one(swap_tx)?;

    match approve_exec_result {
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
    }

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
        ExecutionResult::Revert { gas, logs, output } => {
            println!("Tx reverted!");
            println!("Gas used: {:?}", gas);
            for (i, log) in logs.iter().enumerate() {
                println!("Log {i}: address={:?}, data={:?}", log.address, log.data);
            }
            println!("Bytes: {}", output);
        }
        other => {
            println!("Tx replay execution failed: {other:?}");
        }
    };

    Ok(())
}
