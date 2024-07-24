use ethers_core::{types::{H160, U256}, utils::keccak256};

use crate::{
    chain_fusion::{
        eth_send_raw_transaction::{create_sign_request, send_raw_transaction},
        evm_rpc::{self, GetTransactionReceiptResult, MultiGetTransactionReceiptResult, SendRawTransactionStatus, TransactionReceipt, EVM_RPC},
        evm_signer,
        fees,
    }, state::read_network_state, types::H160Ext
};
use ethers_core::abi::AbiEncode;
use crate::state::{read_state, mutate_network_state};

pub enum TransactionError {
    NonceTooLow,
    NonceTooHigh,
    InsufficientFunds,
    NoTransaction,
    NoReceipt,
    InconsistentReceipt,
    FailedToGetReceipt,
    RpcCallFailed,
}

/// The number of historical blocks to use for fee estimation.
pub const FEE_ESTIMATE_BLOCKS: u8 = 9;

async fn send(network_id: u32, tx: String) -> Result<TransactionReceipt, TransactionError> {
    let status = send_raw_transaction(network_id, tx.clone()).await;
    println!("Placed transaction on network {}: {:?}", network_id, tx);
    let tx_hash = match status {
        SendRawTransactionStatus::Ok(tx_hash) => {
            let tx_hash = match tx_hash {
                Some(tx_hash) => tx_hash,
                None => {
                    return Err(TransactionError::NoTransaction)
                }
            };
            mutate_network_state(network_id, |s| {
                s.nonce += U256::from(1);
            });
            tx_hash
        }
        SendRawTransactionStatus::NonceTooLow => {
            return Err(TransactionError::NonceTooLow)
        }
        SendRawTransactionStatus::NonceTooHigh => {
            return Err(TransactionError::NonceTooHigh)
        }
        SendRawTransactionStatus::InsufficientFunds => {
            return Err(TransactionError::InsufficientFunds)
        }
    };

    let rpc_providers = read_network_state(network_id, |s| s.rpc_services.clone());
    let (result,) = EVM_RPC
        .eth_get_transaction_receipt(rpc_providers, None, tx_hash)
        .await
        .map_err(|_| TransactionError::RpcCallFailed)?;

    match result {
        MultiGetTransactionReceiptResult::Consistent(receipt) => {
            match receipt {
                GetTransactionReceiptResult::Ok(Some(receipt)) => {
                    return Ok(receipt)
                }
                GetTransactionReceiptResult::Ok(None) => {
                    return Err(TransactionError::FailedToGetReceipt)
                }
                GetTransactionReceiptResult::Err(_) => {
                    return Err(TransactionError::NoReceipt)
                }
            }
        },
        MultiGetTransactionReceiptResult::Inconsistent(_) => {
            return Err(TransactionError::InconsistentReceipt)
        }
    }
}

pub struct PreparedTransaction {
    pub network_id: u32,
    pub signed_tx: String,
    pub gas_limit: U256,
    pub fee_settings: fees::FeeSettings,
}

pub async fn send_with_gas_payment_by_safe(tx: PreparedTransaction) -> Result<TransactionReceipt, TransactionError> {
    send(tx.network_id, tx.signed_tx.clone()).await
}

pub async fn send_with_gas_payment_by_user(tx: PreparedTransaction) -> Result<TransactionReceipt, TransactionError> {
    // TODO: Make user actually pay for the gas
    let _max_gas_cost = tx.gas_limit.checked_add(tx.fee_settings.max_fee_per_gas.checked_add(tx.fee_settings.max_priority_fee_per_gas).expect("Fee settings are invalid")).expect("Fee settings are invalid");
    send(tx.network_id, tx.signed_tx.clone()).await
}

pub async fn transfer_eth_tx(network_id: u32, to: H160, amount: U256, gas_limit: Option<U256>, fee_settings: Option<fees::FeeSettings>) -> PreparedTransaction {
    // TODO: Gas limit should be configurable per network
    let gas_limit = gas_limit.unwrap_or(U256::from(5000000));
    let fee_settings = match fee_settings {
        Some(fee_settings) => fee_settings,
        None => fees::estimate_transaction_fees(network_id, FEE_ESTIMATE_BLOCKS).await,
    };
    let request = create_sign_request(
        network_id,
        amount,
        None,
        Some(to.to_repr()),
        gas_limit,
        None,
        fee_settings.clone(),
    ).await;

    PreparedTransaction {
        network_id,
        signed_tx: evm_signer::sign_transaction(request).await,
        gas_limit,
        fee_settings,
    }
}

pub async fn transfer_eth(network_id: u32, to: H160, amount: U256, gas_limit: Option<U256>, fee_settings: Option<fees::FeeSettings>) -> Result<TransactionReceipt, TransactionError> {
    let tx = transfer_eth_tx(network_id, to, amount, gas_limit, fee_settings).await;
    send_with_gas_payment_by_user(tx).await
}

pub async fn transfer_erc20_tx(network_id: u32, token: H160, to: H160, amount: U256, gas_limit: Option<U256>, fee_settings: Option<fees::FeeSettings>) -> PreparedTransaction {
    //TODO: Should probably be hardcoded. Recomputing the hash every time is unnecessary
    let function_signature = "transfer(address,uint256)";
    let selector = &keccak256(function_signature.as_bytes())[0..4];
    let args = (to, amount).encode();
    let mut data = Vec::from(selector);
    data.extend(args);

    let gas_limit = gas_limit.unwrap_or(U256::from(5000000));
    let fee_settings = match fee_settings {
        Some(fee_settings) => fee_settings,
        None => fees::estimate_transaction_fees(network_id, FEE_ESTIMATE_BLOCKS).await,
    };

    let request = create_sign_request(
        network_id,
        U256::from(0),
        Some(token.to_repr()),
        None,
        gas_limit,
        Some(data),
        fee_settings.clone(),
    ).await;

    PreparedTransaction {
        network_id,
        signed_tx: evm_signer::sign_transaction(request).await,
        gas_limit,
        fee_settings,
    }
}

pub async fn transfer_erc20(network_id: u32, token: H160, to: H160, amount: U256, gas_limit: Option<U256>, fee_settings: Option<fees::FeeSettings>) -> Result<TransactionReceipt, TransactionError> {
    let tx = transfer_erc20_tx(network_id, token, to, amount, gas_limit, fee_settings).await;
    send_with_gas_payment_by_user(tx).await
}
