use candid::Nat;
use ethers_core::types::U256;
use ic_cdk::api::call::RejectionCode;
use serde_bytes::ByteBuf;
use std::ops::Add;
use thiserror::Error;

use crate::{chain_fusion::evm_rpc::{BlockTag, FeeHistory, FeeHistoryArgs, FeeHistoryResult, MultiFeeHistoryResult, EVM_RPC}, types::NatExt};
use crate::state::read_network_state;

const MIN_SUGGEST_MAX_PRIORITY_FEE_PER_GAS: u32 = 1_500_000_000;

#[derive(Error, Debug)]
pub enum RpcCallError {
    #[error("Inconsistent responses")]
    InconsistentResponses,
    #[error("RPC error")]
    RpcError,
    #[error("Canister call rejected: {0:?} {1}")]
    CallRejected(RejectionCode, String)
}

#[derive(Error, Debug)]
pub enum FeeHistoryError {
    #[error("RPC call error: {0}")]
    RpcCallError(RpcCallError),
    #[error("No fee history available")]
    NoHistory
}

impl From<(RejectionCode, String)> for RpcCallError {
    fn from((code, message): (RejectionCode, String)) -> Self {
        RpcCallError::CallRejected(code, message)
    }
}

impl From<RpcCallError> for FeeHistoryError {
    fn from(error: RpcCallError) -> Self {
        FeeHistoryError::RpcCallError(error)
    }
}

impl From<(RejectionCode, String)> for FeeHistoryError {
    fn from(rejection: (RejectionCode, String)) -> Self {
        FeeHistoryError::RpcCallError(rejection.into())
    }
}

impl From<MultiFeeHistoryResult> for Result<Option<FeeHistory>, RpcCallError> {
    fn from(result: MultiFeeHistoryResult) -> Self {
        match result {
            MultiFeeHistoryResult::Consistent(fee_history) => match fee_history {
                FeeHistoryResult::Ok(fee_history) => Ok(fee_history),
                FeeHistoryResult::Err(_) => Err(RpcCallError::InconsistentResponses),
            },
            MultiFeeHistoryResult::Inconsistent(_) => Err(RpcCallError::InconsistentResponses),
        }
    }
}

pub async fn fee_history(
    network_id: u32,
    block_count: Nat,
    newest_block: BlockTag,
    reward_percentiles: Option<Vec<u8>>,
) -> Result<FeeHistory, FeeHistoryError> {
    let rpc_providers = read_network_state(network_id, |s| s.rpc_services.clone());
    let fee_history_args: FeeHistoryArgs = FeeHistoryArgs {
        blockCount: block_count,
        newestBlock: newest_block,
        rewardPercentiles: reward_percentiles.map(ByteBuf::from),
    };

    let cycles = 10_000_000_000;

    let (result,) = EVM_RPC
        .eth_fee_history(rpc_providers, None, fee_history_args, cycles)
        .await?;

    let fee_history = Result::<Option<FeeHistory>, RpcCallError>::from(result)?;
    match fee_history {
        Some(fee_history) => Ok(fee_history),
        None => Err(FeeHistoryError::NoHistory),
    }
}

#[derive(Clone)]
pub struct FeeSettings {
    pub max_fee_per_gas: U256,
    pub max_priority_fee_per_gas: U256,
}

fn median_index(length: usize) -> usize {
    if length == 0 {
        panic!("Cannot find a median index for an array of length zero.");
    }
    (length - 1) / 2
}

pub async fn estimate_transaction_fees(network_id: u32, block_count: u8) -> Result<FeeSettings, FeeHistoryError> {
    // we are setting the `max_priority_fee_per_gas` based on this article:
    // https://docs.alchemy.com/docs/maxpriorityfeepergas-vs-maxfeepergas
    // following this logic, the base fee will be derived from the block history automatically
    // and we only specify the maximum priority fee per gas (tip).
    // the tip is derived from the fee history of the last 9 blocks, more specifically
    // from the 95th percentile of the tip.
    let fee_history = fee_history(network_id, Nat::from(block_count), BlockTag::Latest, Some(vec![95])).await?;

    let median_index = median_index(block_count.into());

    // baseFeePerGas
    let base_fee_per_gas = fee_history.baseFeePerGas.last().ok_or(FeeHistoryError::NoHistory)?.clone();

    // obtain the 95th percentile of the tips for the past 9 blocks
    let mut percentile_95: Vec<Nat> = fee_history
        .reward
        .into_iter()
        .flat_map(|x| x.into_iter())
        .collect();
    // sort the tips in ascending order
    percentile_95.sort_unstable();
    // get the median by accessing the element in the middle
    // set tip to 0 if there are not enough blocks in case of a local testnet
    let median_reward = percentile_95
        .get(median_index).unwrap_or(&Nat::from(0_u8))
        .clone();

    let max_priority_fee_per_gas = median_reward
        .clone()
        .add(base_fee_per_gas)
        .max(Nat::from(MIN_SUGGEST_MAX_PRIORITY_FEE_PER_GAS));

    Ok(FeeSettings {
        max_fee_per_gas: max_priority_fee_per_gas.to_u256(),
        max_priority_fee_per_gas: median_reward.to_u256(),
    })
}
