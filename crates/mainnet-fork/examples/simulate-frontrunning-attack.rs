use alloy::primitives::{U256, address};
use alloy::sol_types::{GenericRevertReason, SolCall};
use alloy::{eips::BlockId, providers::ProviderBuilder};
use dotenv::dotenv;
use eyre::{Result, eyre};
use revm::context::TxEnv;
use revm::context::result::ExecutionResult;
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};
use revm::primitives::{Bytes, TxKind};
use revm::{Context, ExecuteCommitEvm, MainBuilder, MainContext};

#[path = "common/mod.rs"]
mod common;
use common::*;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    env_logger::init();

    let rpc_url = std::env::var("ALCHEMY_RPC_URL").expect("ALCHEMY_RPC_URL must be set in .env");
    let provider = ProviderBuilder::new().connect(&rpc_url).await?;

    // Fork at a specific block. AlloyDB lazily fetches state from the provider.
    // WrapDatabaseAsync bridges the async provider into revm's sync Database trait.
    let alloy_db = WrapDatabaseAsync::new(AlloyDB::new(provider, BlockId::number(22600000)))
        .ok_or_else(|| eyre!("failed to build AlloyDB"))?;

    let mut cache_db = CacheDB::new(alloy_db);

    // let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
    let usdc = address!("A0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48");
    let pair = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");
    let my_addr = address!("0000000000000000000000000000000000000001");

    let usdc_in = U256::from(50_000u64) * U256::from(10u64).pow(U256::from(6u64));
    let slot = balance_slot(my_addr, 9); // Using 9 as it's a storage slot for balances in USDC smart contract
    cache_db.insert_account_storage(usdc, slot, usdc_in)?;

    let (r0_before, r1_before) = read_reserves(&mut cache_db, pair)?;
    println!(
        "BEFORE  USDC reserve: {:.2}  WETH reserve: {:.4}",
        r0_before.to::<u128>() as f64 / 1e6,
        r1_before.to::<u128>() as f64 / 1e18
    );

    let weth_out = get_amount_out(usdc_in, r0_before, r1_before);
    println!("Expected weth out: {}", weth_out.to::<u128>() as f64 / 1e18);

    println!(
        "Balance of my_addr before transfer: {:?} USDC",
        read_balance(&mut cache_db, usdc, my_addr)?.to::<u128>() as f64 / 1e6
    );

    let mut evm = Context::mainnet().with_db(&mut cache_db).build_mainnet();

    let transfer_cd = IERC20::transferCall {
        to: pair,
        amount: usdc_in,
    }
    .abi_encode();

    // evm.transact_commit(TxEnv {
    //     caller: my_addr,
    //     kind: TxKind::Call(usdc),
    //     data: Bytes::from(transfer_cd),
    //     ..Default::default()
    // })?;

    let swap_cd = IUniswapV2Pair::swapCall {
        amount0Out: U256::ZERO,
        amount1Out: weth_out,
        to: my_addr,
        data: Bytes::new(),
    }
    .abi_encode();

    let swap_tx = evm.transact_commit(TxEnv {
        caller: my_addr,
        kind: TxKind::Call(pair),
        data: Bytes::from(swap_cd),
        ..Default::default()
    })?;

    match swap_tx {
        ExecutionResult::Success {
            reason: _reason,
            gas: _gas,
            logs: _logs,
            output: _output,
        } => {
            println!("Success")
        }
        ExecutionResult::Revert {
            gas: _gas,
            logs: _logs,
            output: _output,
        } => {
            if let Some(reason) = GenericRevertReason::decode(&_output) {
                println!("Revert reason: {reason}");
            }
        }
        other => println!("Other: {other:?}"),
    }

    println!(
        "Balance of my_addr after transfer: {:?} USDC",
        read_balance(&mut cache_db, usdc, my_addr)?.to::<u128>() as f64 / 1e6
    );

    let (r0_after, r1_after) = read_reserves(&mut cache_db, pair)?;
    println!(
        "AFTER   USDC: {:.2}  WETH: {:.4}",
        r0_after.to::<u128>() as f64 / 1e6,
        r1_after.to::<u128>() as f64 / 1e18
    );

    Ok(())
}
