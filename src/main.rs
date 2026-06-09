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

    let reserves = pair.getReserves().call().await?;
    let token0 = pair.token0().call().await?;
    let token1 = pair.token1().call().await?;

    let reserve0 = reserves.reserve0;
    let reserve1 = reserves.reserve1;

    println!("Token0: {token0}");
    println!("Token1: {token1}");

    let usdc = reserve0.to::<u128>() as f64 / 1e6;
    let weth = reserve1.to::<u128>() as f64 / 1e18;

    println!("USDC Reserve: {usdc}");
    println!("WETH Reserve: {weth}");

    let filter = Filter::new()
        .address(pair_addr)
        .event_signature(IUniswapV2Pair::Swap::SIGNATURE_HASH)
        .from_block(22600000)
        .to_block(22600009);

    let logs = provider.get_logs(&filter).await?;

    for log in logs {
        let swap = IUniswapV2Pair::Swap::decode_log(log.as_ref())?;
        let tx_hash = log.transaction_hash.unwrap_or(Default::default());

        let amount0_in: U256 = swap.amount0In.into();
        let amount1_in: U256 = swap.amount1In.into();
        let amount0_out: U256 = swap.amount0Out.into();
        let amount1_out: U256 = swap.amount1Out.into();

        println!("---");
        println!("tx hash:     {}", tx_hash);
        println!("sender:     {}", swap.sender);
        println!("amount0In:  {}", amount0_in);
        println!("amount1In:  {}", amount1_in);
        println!("amount0Out: {}", amount0_out);
        println!("amount1Out: {}", amount1_out);
        println!("to:         {}", swap.to);
    }

    Ok(())
}
