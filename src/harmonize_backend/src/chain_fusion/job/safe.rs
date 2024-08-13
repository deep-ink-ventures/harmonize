use candid::CandidType;
use ethers_core::{types::{H160, U256}, utils::keccak256};

use crate::{
    chain_fusion::{
        eth_send_raw_transaction::{create_sign_request, send_raw_transaction},
        evm_rpc::{self, GetTransactionReceiptResult, MultiGetTransactionReceiptResult, SendRawTransactionStatus, TransactionReceipt, EVM_RPC},
        evm_signer,
        fees::{self, FeeHistoryError},
    }, state::read_network_state, types::H160Ext
};
use ethers_core::abi::AbiEncode;
use thiserror::Error;
use crate::state::{read_state, mutate_network_state};

#[derive(Error, Debug)]
pub enum TransactionError {
    #[error("The nonce is too low")]
    NonceTooLow,
    #[error("The nonce is too high")]
    NonceTooHigh,
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("No transaction was created")]
    NoTransaction,
    #[error("No receipt was returned")]
    NoReceipt,
    #[error("The receipt is inconsistent")]
    InconsistentReceipt,
    #[error("Failed to get the receipt")]
    FailedToGetReceipt,
    #[error("An RPC call failed")]
    RpcCallFailed,
    #[error("Failed to get the fee history: {0}")]
    FeeHistoryError(#[from] fees::FeeHistoryError),
    #[error("Failed to sign the transaction: {0}")]
    SignTransactionError(#[from] evm_signer::SignerError),
}

/// The number of historical blocks to use for fee estimation.
pub const FEE_ESTIMATE_BLOCKS: u8 = 9;

impl From<SendRawTransactionStatus> for Result<String, TransactionError> {
    fn from(status: SendRawTransactionStatus) -> Self {
        match status {
            SendRawTransactionStatus::Ok(Some(tx_hash)) => Ok(tx_hash),
            SendRawTransactionStatus::Ok(None) =>  Err(TransactionError::NoTransaction),
            SendRawTransactionStatus::NonceTooLow => Err(TransactionError::NonceTooLow),
            SendRawTransactionStatus::NonceTooHigh => Err(TransactionError::NonceTooHigh),
            SendRawTransactionStatus::InsufficientFunds => Err(TransactionError::InsufficientFunds),
        }
    }
}

impl From<MultiGetTransactionReceiptResult> for Result<TransactionReceipt, TransactionError> {
    fn from(result: MultiGetTransactionReceiptResult) -> Self {
        match result {
            MultiGetTransactionReceiptResult::Consistent(receipt) => {
                match receipt {
                    GetTransactionReceiptResult::Ok(Some(receipt)) => Ok(receipt),
                    GetTransactionReceiptResult::Ok(None) => Err(TransactionError::NoReceipt),
                    GetTransactionReceiptResult::Err(_) => Err(TransactionError::FailedToGetReceipt)
                }
            }
            MultiGetTransactionReceiptResult::Inconsistent(_) => Err(TransactionError::InconsistentReceipt)
        }
    }
}

async fn send(network_id: u32, tx: String) -> Result<TransactionReceipt, TransactionError> {
    let status = send_raw_transaction(network_id, tx.clone()).await;
    println!("Placed transaction on network {}: {:?}", network_id, tx);
    let tx_hash = match status.into() {
        Ok(Some(tx_hash)) => tx_hash,
        Ok(None) => {
            return Err(TransactionError::NoTransaction)
        },
        Err(e) => {
            return Err(TransactionError::RpcCallFailed)
        }
    };
    mutate_network_state(network_id, |s| {
        s.nonce += U256::from(1);
    });

    let rpc_providers = read_network_state(network_id, |s| s.rpc_services.clone());
    let (result,) = EVM_RPC
        .eth_get_transaction_receipt(rpc_providers, None, tx_hash)
        .await
        .map_err(|_| TransactionError::RpcCallFailed)?;

    result.into()
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

lazy_static! {
    static ref ETH_TRANSFER_GAS_LIMIT: U256 = U256::from(21000);
}

pub async fn transfer_eth_tx(network_id: u32, to: H160, amount: U256, gas_limit: Option<U256>, fee_settings: Option<fees::FeeSettings>) -> Result<PreparedTransaction, TransactionError> {
    let gas_limit = gas_limit.unwrap_or(*ETH_TRANSFER_GAS_LIMIT);
    let fee_settings = match fee_settings {
        Some(fee_settings) => fee_settings,
        None => fees::estimate_transaction_fees(network_id, FEE_ESTIMATE_BLOCKS).await?,
    };
    let request = create_sign_request(
        network_id,
        amount,
        None,
        Some(to),
        gas_limit,
        None,
        fee_settings.clone(),
    ).await;
    Ok(PreparedTransaction {
        network_id,
        signed_tx: evm_signer::sign_transaction(request).await?,
        gas_limit,
        fee_settings,
    })
}

pub async fn transfer_eth(network_id: u32, to: H160, amount: U256, gas_limit: Option<U256>, fee_settings: Option<fees::FeeSettings>) -> Result<TransactionReceipt, TransactionError> {
    let tx = transfer_eth_tx(network_id, to, amount, gas_limit, fee_settings).await?;
    send_with_gas_payment_by_user(tx).await
}

use lazy_static::lazy_static;

pub const ERC20_TRANSFER_SIGNATURE: &str = "transfer(address,uint256)";

lazy_static! {
    static ref ERC20_TRANSFER_SELECTOR: Vec<u8> = keccak256(ERC20_TRANSFER_SIGNATURE.as_bytes())[0..4].to_vec();
    static ref ERC20_TRANSFER_GAS_LIMIT: U256 = U256::from(5000000);
}

pub async fn transfer_erc20_tx(network_id: u32, token: H160, to: H160, amount: U256, gas_limit: Option<U256>, fee_settings: Option<fees::FeeSettings>) -> Result<PreparedTransaction, TransactionError> {
    let args = (to, amount).encode();
    let mut data = ERC20_TRANSFER_SELECTOR.clone();
    data.extend(args);

    let fee_settings = match fee_settings {
        Some(fee_settings) => fee_settings,
        None => fees::estimate_transaction_fees(network_id, FEE_ESTIMATE_BLOCKS).await?,
    };
    let gas_limit = gas_limit.unwrap_or(*ERC20_TRANSFER_GAS_LIMIT);

    let request = create_sign_request(
        network_id,
        U256::from(0),
        Some(token),
        None,
        gas_limit,
        Some(data),
        fee_settings.clone(),
    ).await;
    Ok(PreparedTransaction {
        network_id,
        signed_tx: evm_signer::sign_transaction(request).await?,
        gas_limit,
        fee_settings,
    })
}

pub async fn transfer_erc20(network_id: u32, token: H160, to: H160, amount: U256, gas_limit: Option<U256>, fee_settings: Option<fees::FeeSettings>) -> Result<TransactionReceipt, TransactionError> {
    let tx = transfer_erc20_tx(network_id, token, to, amount, gas_limit, fee_settings).await?;
    send_with_gas_payment_by_user(tx).await
}
