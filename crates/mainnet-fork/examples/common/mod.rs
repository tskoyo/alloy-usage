use alloy::primitives::{Address, Bytes, TxKind, U256, keccak256};
use alloy::sol;
use alloy::sol_types::SolCall;
use eyre::{Result, eyre};
use revm::context::TxEnv;
use revm::context::result::ExecutionResult;
use revm::{Context, Database, ExecuteEvm, MainBuilder, MainContext};

sol! {
    #[sol(rpc)]
    interface IERC20 {
        function balanceOf(address account) external view returns (uint256);
        function transfer(address to, uint256 amount) external returns (bool);
    }

    #[sol(rpc)]
    interface IUniswapV2Pair {
        function getReserves() external view returns (uint112 reserve0, uint112 reserve1, uint32 blockTimestampLast);
        function swap(uint amount0Out, uint amount1Out, address to, bytes data) external;
    }
}

pub fn balance_slot(holder: Address, mapping_slot: u64) -> U256 {
    let mut buf = [0u8; 64];
    buf[12..32].copy_from_slice(holder.as_slice());
    buf[63] = mapping_slot as u8;
    U256::from_be_bytes(keccak256(buf).0)
}

pub fn read_balance<DB>(db: DB, token: Address, account: Address) -> Result<U256>
where
    DB: Database,
{
    let mut context = Context::mainnet().with_db(db);
    context.cfg.disable_nonce_check = true;
    let mut evm = context.build_mainnet();

    let calldata = IERC20::balanceOfCall { account: account }.abi_encode();
    let tx = TxEnv {
        caller: account,
        kind: TxKind::Call(token),
        data: Bytes::from(calldata),
        ..Default::default()
    };
    let result = evm.transact_one(tx)?;

    match result {
        ExecutionResult::Success {
            reason: _reason,
            gas: _gas,
            logs: _logs,
            output: _output,
        } => {
            let data = _output.data();
            let balance = IERC20::balanceOfCall::abi_decode_returns(data)?;
            Ok(balance)
        }
        other => Err(eyre!("balanceOf failed: {other:?}")),
    }
}

pub fn read_reserves<DB>(db: DB, pair: Address) -> Result<(U256, U256)>
where
    DB: Database,
{
    let mut context = Context::mainnet().with_db(db);
    context.cfg.disable_nonce_check = true;
    let mut evm = context.build_mainnet();

    let calldata = IUniswapV2Pair::getReservesCall {}.abi_encode();

    let tx = TxEnv {
        caller: Address::ZERO,
        kind: TxKind::Call(pair),
        data: Bytes::from(calldata),
        ..Default::default()
    };
    let result = evm.transact_one(tx)?;

    match result {
        ExecutionResult::Success {
            reason: _reason,
            gas: _gas,
            logs: _logs,
            output: _output,
        } => {
            let data = _output.data();
            let decoded = IUniswapV2Pair::getReservesCall::abi_decode_returns(data)?;
            let r0 = U256::from(decoded.reserve0.to::<u128>());
            let r1 = U256::from(decoded.reserve1.to::<u128>());

            Ok((r0, r1))
        }
        other => Err(eyre!("getReserves failed: {other:?}")),
    }
}

pub fn get_amount_out(amount_in: U256, reserve_in: U256, reserve_out: U256) -> U256 {
    let fee_numerator = U256::from(997u64);
    let fee_denominator = U256::from(1000u64);

    let amount_in_with_fee = amount_in * fee_numerator;
    let numerator = amount_in_with_fee * reserve_out;
    let denominator = reserve_in * fee_denominator + amount_in_with_fee;

    numerator / denominator
}
