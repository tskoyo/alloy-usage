use alloy::primitives::{Address, Bytes, U256, address, keccak256};
use alloy::sol_types::SolCall;
use alloy::{eips::BlockId, providers::ProviderBuilder, sol};
use dotenv::dotenv;
use eyre::{Result, eyre};
use revm::context::TxEnv;
use revm::context::result::ExecutionResult;
use revm::database::{AlloyDB, CacheDB, WrapDatabaseAsync};
use revm::primitives::TxKind;
use revm::{Context, DatabaseRef, ExecuteEvm, MainBuilder, MainContext};

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function balanceOf(address account) external view returns (uint256);
    }
}

fn erc20_balance_slot(holder: Address, mapping_slot: u64) -> U256 {
    // keccak256(abi.encode(holder, mapping_slot))
    // abi.encode pads each to 32 bytes
    let mut buf = [0u8; 64];
    buf[12..32].copy_from_slice(holder.as_slice()); // holder, left-padded to 32
    buf[63] = mapping_slot as u8; // slot index in last byte (fine for small indices)
    U256::from_be_bytes(keccak256(buf).0)
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

    let weth = address!("C02aaA39b223FE8D0A0e5C4F27eAD9083C756Cc2");
    let my_addr = address!("0000000000000000000000000000000000000001"); // any address you control

    let slot = erc20_balance_slot(my_addr, 3);
    let amount = U256::from(100u64) * U256::from(10u64).pow(U256::from(18u64));

    let mut cache_db = CacheDB::new(alloy_db);
    cache_db.insert_account_storage(weth, slot, amount)?;

    let slot0 = cache_db.storage_ref(weth, U256::from(0))?;
    println!("Slot 0 (name): {:#x}", slot0);

    let mut evm = Context::mainnet().with_db(cache_db).build_mainnet();
    let calldata = IERC20::balanceOfCall { account: my_addr }.abi_encode();

    let tx = TxEnv {
        caller: my_addr,
        kind: TxKind::Call(weth),
        data: Bytes::from(calldata),
        ..Default::default()
    };
    let result = evm.transact_one(tx)?;
    match result {
        ExecutionResult::Success {
            reason,
            gas,
            logs,
            output,
        } => {
            println!("Tx successfully executed");
            println!("Reason: {reason:?}");
            println!("Gas used: {gas}");
            for (i, log) in logs.iter().enumerate() {
                println!("Log {i}: address={:?}, data={:?}", log.address, log.data);
            }
            println!("Output: {output:?}");
        }
        _ => {
            println!("Tx failed");
        }
    }

    Ok(())
}
