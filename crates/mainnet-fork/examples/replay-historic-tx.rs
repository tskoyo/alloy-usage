use alloy::primitives::{Address, TxKind, U256, address};
use alloy::sol;
use alloy::sol_types::SolCall;
use dotenv::dotenv;
use eyre::{Result, eyre};
use mainnet_fork::fork_mainnet;
use revm::context::TxEnv;
use revm::context::result::{ExecutionResult, Output};
use revm::{Context, ExecuteEvm, MainBuilder, MainContext};

sol! {
    function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let db = fork_mainnet::call().await?;

    let pair = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");

    let encoded = getReservesCall::new(()).abi_encode();

    let tx = TxEnv {
        caller: Address::ZERO,
        kind: TxKind::Call(pair),
        data: encoded.into(),
        value: U256::ZERO,
        gas_limit: 1_000_000,
        ..Default::default()
    };

    let mut evm = Context::mainnet().with_db(db).build_mainnet();
    let exec_result = evm.transact_one(tx)?;

    let output = match exec_result {
        ExecutionResult::Success {
            output: Output::Call(bytes),
            ..
        } => bytes,
        other => return Err(eyre!("call failed: {other:?}")),
    };

    let reserves = getReservesCall::abi_decode_returns(&output)?;

    println!("Reserves 0: {}", reserves.reserve0);
    println!("Reserves 1: {}", reserves.reserve1);
    println!("blockTimestampLast: {}", reserves.blockTimestampLast);

    Ok(())
}
