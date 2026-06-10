use alloy::eips::BlockId;
use alloy::primitives::{U256, address};
use alloy::providers::{Provider, ProviderBuilder};
use alloy::rpc::types::Filter;
use alloy::sol;
use alloy::sol_types::SolEvent;
use eyre::Result;

sol! {
    #[sol(rpc)]
    interface IUniswapV2Pair {
        function getReserves() public view returns (uint112 reserve0, uint112 reserve1, uint32 _blockTimestampLast);
        function token0() external view returns (address);
        function token1() external view returns (address);

        event Swap(
            address indexed sender,
            uint amount0In,
            uint amount1In,
            uint amount0Out,
            uint amount1Out,
            address indexed to
        );
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let rpc_url = "https://eth-mainnet.g.alchemy.com/v2/zonUBZKgwbwydPXz3xwGA";
    let provider = ProviderBuilder::new().connect(rpc_url).await?;

    let pair_addr = address!("B4e16d0168e52d35CaCD2c6185b44281Ec28C9Dc");
    let pair = IUniswapV2Pair::new(pair_addr, &provider);

    let token0 = pair.token0().call().await?;
    let token1 = pair.token1().call().await?;

    println!("Token0: {token0}");
    println!("Token1: {token1}");

    let filter = Filter::new()
        .address(pair_addr)
        .event_signature(IUniswapV2Pair::Swap::SIGNATURE_HASH)
        .from_block(22600000)
        .to_block(22600009);

    let logs = provider.get_logs(&filter).await?;

    for log in logs {
        let swap = IUniswapV2Pair::Swap::decode_log(log.as_ref())?;
        let tx_hash = log.transaction_hash.unwrap_or(Default::default());

        let block_num = log.block_number.unwrap_or_default();
        let reserves = pair
            .getReserves()
            .block(BlockId::number(block_num - 1))
            .call()
            .await?;

        let reserve0 = U256::from(reserves.reserve0.to::<u128>());
        let reserve1 = U256::from(reserves.reserve1.to::<u128>());

        let amount0_in: U256 = swap.amount0In.into();
        let amount0_out: U256 = swap.amount0Out.into();
        let amount1_in: U256 = swap.amount1In.into();
        let amount1_out: U256 = swap.amount1Out.into();

        let fee_numerator = U256::from(997u64);
        let fee_denominator = U256::from(1000u64);

        // formula: amountOut = (amountIn * 997 * reserveOut) / (reserveIn * 1000 + amountIn * 997)
        let (computed_out, actual_out, label) = if amount0_in > U256::ZERO {
            let computed_amount1_out = (amount0_in * fee_numerator * reserve1)
                / (reserve0 * fee_denominator + amount0_in * fee_numerator);
            (computed_amount1_out, amount1_out, "amount1Out")
        } else {
            let computed_amount0_out = (amount1_in * fee_numerator * reserve0)
                / (reserve1 * fee_denominator + amount1_in * fee_numerator);
            (computed_amount0_out, amount0_out, "amount0Out")
        };

        if computed_out == actual_out {
            println!("Swap matches Uniswap V2 formula, tx hash: {}", tx_hash);
        }

        if amount0_in > U256::ZERO && computed_out != actual_out {
            println!(
                "Discrepancy detected in swapping (USDC -> WETH), tx hash: {}",
                tx_hash
            );
            println!(
                "Computed {label}: {}",
                computed_out.to::<u128>() as f64 / 1e18
            );
            println!(
                "Actual {label}:   {}",
                actual_out.to::<u128>() as f64 / 1e18
            );
        } else if amount1_in > U256::ZERO && computed_out != actual_out {
            println!(
                "Discrepancy detected in swapping (WETH -> USDC), tx hash: {}",
                tx_hash
            );
            println!(
                "Computed {label}: {}",
                computed_out.to::<u128>() as f64 / 1e6
            );
            println!("Actual {label}:   {}", actual_out.to::<u128>() as f64 / 1e6);
        }

        println!("---");
        println!("tx hash:     {}", tx_hash);
        println!("block number: {}", block_num);
        println!("sender:     {}", swap.sender);
        println!(
            "amount0In:  {}",
            format!("{}", amount0_in.to::<u128>() as f64 / 1e6)
        );
        println!(
            "amount1In:  {}",
            format!("{}", amount1_in.to::<u128>() as f64 / 1e18)
        );
        println!(
            "amount0Out: {}",
            format!("{}", amount0_out.to::<u128>() as f64 / 1e6)
        );
        println!(
            "amount1Out: {}",
            format!("{}", amount1_out.to::<u128>() as f64 / 1e18)
        );
        println!("to:         {}", swap.to);
    }

    Ok(())
}
