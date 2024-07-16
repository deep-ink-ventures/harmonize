use ethers_core::types::{H160, U256, U64};
use ic_cdk::println;

use crate::{
    chain_fusion::{
        evm_rpc::{
            MultiSendRawTransactionResult, RpcServices, SendRawTransactionResult,
            SendRawTransactionStatus, EVM_RPC,
        },
        evm_signer::{self, SignRequest},
        fees::{estimate_transaction_fees, FeeEstimates}
    },
    types::H160Ext,
};
use crate::state::{mutate_network_state, read_network_state};

pub async fn transfer_eth(network_id: u32, value: U256, to: H160, gas: Option<U256>) {
    let gas = gas.unwrap_or(U256::from(21000));
    let fee_estimates = estimate_transaction_fees(network_id, 9).await;
    let request = create_sign_request(network_id, value, Some(to.to_repr()), None, gas, None, fee_estimates).await;
    let tx = evm_signer::sign_transaction(request).await;
    let status = send_raw_transaction(network_id, tx.clone()).await;

    println!("Transaction sent: {:?}", tx);

    match status {
        SendRawTransactionStatus::Ok(transaction_hash) => {
            println!("Success {transaction_hash:?}");
            mutate_network_state(network_id,|s| {
                s.nonce += U256::from(1);
            });
        }
        SendRawTransactionStatus::NonceTooLow => {
            println!("Nonce too low");
        }
        SendRawTransactionStatus::NonceTooHigh => {
            println!("Nonce too high");
        }
        SendRawTransactionStatus::InsufficientFunds => {
            println!("Insufficient funds");
        }
    }
}

pub async fn create_sign_request(
    network_id: u32,
    value: U256,
    to: Option<String>,
    from: Option<String>,
    gas: U256,
    data: Option<Vec<u8>>,
    fee_estimates: FeeEstimates,
) -> SignRequest {
    let FeeEstimates {
        max_fee_per_gas,
        max_priority_fee_per_gas,
    } = fee_estimates;
    let nonce = read_network_state(network_id, |s| s.nonce);
    let rpc_providers = read_network_state(network_id, |s| s.rpc_services.clone());

    SignRequest {
        chain_id: Some(rpc_providers.chain_id()),
        to,
        from,
        gas,
        max_fee_per_gas: Some(max_fee_per_gas),
        max_priority_fee_per_gas: Some(max_priority_fee_per_gas),
        data,
        value: Some(value),
        nonce: Some(nonce),
    }
}

pub async fn send_raw_transaction(network_id: u32, tx: String) -> SendRawTransactionStatus {
    let rpc_providers = read_network_state(network_id, |s| s.rpc_services.clone());
    let cycles = 10_000_000_000;

    match EVM_RPC
        .eth_send_raw_transaction(rpc_providers, None, tx, cycles)
        .await
    {
        Ok((res,)) => match res {
            MultiSendRawTransactionResult::Consistent(status) => match status {
                SendRawTransactionResult::Ok(status) => status,
                SendRawTransactionResult::Err(e) => {
                    ic_cdk::trap(format!("Error: {:?}", e).as_str());
                }
            },
            MultiSendRawTransactionResult::Inconsistent(_) => {
                ic_cdk::trap("Status is inconsistent");
            }
        },
        Err(e) => ic_cdk::trap(format!("Error: {:?}", e).as_str()),
    }
}

impl RpcServices {
    pub fn chain_id(&self) -> U64 {
        match self {
            RpcServices::EthSepolia(_) => U64::from(11155111),
            RpcServices::Custom {
                chainId,
                services: _,
            } => U64::from(*chainId),
            RpcServices::EthMainnet(_) => U64::from(1),
        }
    }
}
