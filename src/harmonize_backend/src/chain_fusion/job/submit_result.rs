use ethers_core::{types::{H160, U256}, utils::keccak256};

use crate::{
    chain_fusion::{
        eth_send_raw_transaction::{create_sign_request, send_raw_transaction},
        evm_rpc::SendRawTransactionStatus,
        evm_signer,
        fees,
    },
    types::H160Ext
};
use ethers_core::abi::AbiEncode;
use crate::state::{read_state, mutate_network_state};

pub async fn submit_result(network_id: u32, result: String, job_id: U256) {
    //TODO: Should probably be hardcoded. Recomputing the hash every time is unnecessary
    let function_signature = "callback(string,uint256)";

    let selector = &keccak256(function_signature.as_bytes())[0..4];
    let args = (result, job_id).encode();
    let mut data = Vec::from(selector);
    data.extend(args);

    let gas_limit = U256::from(5000000);
    let fee_estimates = fees::estimate_transaction_fees(network_id, 9).await;

    let contract_address = read_state(|s| s.get_logs_address[0].clone());

    let request = create_sign_request(
        network_id,
        U256::from(0),
        Some(contract_address),
        None,
        gas_limit,
        Some(data),
        fee_estimates,
    )
    .await;

    let tx = evm_signer::sign_transaction(request).await;

    let status = send_raw_transaction(network_id, tx.clone()).await;

    println!("Transaction sent: {:?}", tx);

    match status {
        SendRawTransactionStatus::Ok(transaction_hash) => {
            println!("Success {transaction_hash:?}");
            mutate_network_state(network_id, |s| {
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

pub async fn transfer_erc20(network_id: u32, token: H160, to: H160, amount: U256) {
    //TODO: Should probably be hardcoded. Recomputing the hash every time is unnecessary
    let function_signature = "transfer(address,uint256)";
    let selector = &keccak256(function_signature.as_bytes())[0..4];
    let args = (to, amount).encode();
    let mut data = Vec::from(selector);
    data.extend(args);

    let gas_limit = U256::from(5000000); // TODO: Adjust this
    let fee_estimates = fees::estimate_transaction_fees(network_id, 9).await;

    let request = create_sign_request(
        network_id,
        U256::from(0),
        Some(token.to_repr()),
        None,
        gas_limit,
        Some(data),
        fee_estimates,
    ).await;

    let tx = evm_signer::sign_transaction(request).await;
    let status = send_raw_transaction(network_id, tx.clone()).await;

    println!("Transaction sent: {:?}", tx);
    match status {
        SendRawTransactionStatus::Ok(transaction_hash) => {
            println!("Success {transaction_hash:?}");
            mutate_network_state(network_id, |s| {
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
