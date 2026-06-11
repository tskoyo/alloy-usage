use alloy::eips::BlockId;
use alloy::primitives::{Address, Bytes, TxKind, U256, address};
use alloy::providers::ProviderBuilder;
use alloy::sol;
use alloy::sol_types::SolCall;
use dotenv::dotenv;
use eyre::{Result, eyre};
use revm::context::TxEnv;
use revm::context::result::{ExecutionResult, Output};
use revm::database::{AlloyDB, CacheDB};
use revm::database_interface::WrapDatabaseAsync;
use revm::{Context, ExecuteEvm, MainBuilder, MainContext};

sol! {
    #[sol(rpc)]
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
    }
}

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

    let pair = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");

    // Encode getReserves() calldata using the sol! binding
    let calldata = IUniswapV2Pair::getReservesCall {}.abi_encode();

    // Build the tx env: a staticcall-equivalent to the pair
    let tx = TxEnv {
        caller: Address::ZERO,
        kind: TxKind::Call(pair),
        data: Bytes::from(calldata),
        value: U256::ZERO,
        gas_limit: 1_000_000,
        ..Default::default()
    };

    // Construct the EVM with the forked DB and run the call
    let mut evm = Context::mainnet().with_db(&mut cache_db).build_mainnet();
    let result = evm.transact(tx)?;

    let output = match result.result {
        ExecutionResult::Success {
            output: Output::Call(bytes),
            ..
        } => bytes,
        other => return Err(eyre!("call failed: {other:?}")),
    };

    // Decode the returned bytes back into the typed struct
    let reserves = IUniswapV2Pair::getReservesCall::abi_decode_returns(&output)?;

    println!("Reserves read THROUGH revm (forked at block 22600000):");
    println!("reserve0 (USDC): {}", reserves.reserve0);
    println!("reserve1 (WETH): {}", reserves.reserve1);

    Ok(())
}
