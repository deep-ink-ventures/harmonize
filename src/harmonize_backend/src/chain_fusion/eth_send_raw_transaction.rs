use ethers_core::types::{H160, U256, U64};
use crate::{
    state::read_network_state,
    chain_fusion::{
        evm_rpc::{
            MultiSendRawTransactionResult, RpcServices, SendRawTransactionResult,
            SendRawTransactionStatus, EVM_RPC,
        },
        evm_signer::SignRequest,
        fees::FeeSettings
    }
};

pub async fn create_sign_request(
    network_id: u32,
    value: U256,
    to: Option<H160>,
    from: Option<H160>,
    gas: U256,
    data: Option<Vec<u8>>,
    fee_estimates: FeeSettings,
) -> SignRequest {
    let FeeSettings {
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

pub enum SendRawTransactionError {
    NonceTooLow,
    NonceTooHigh,
    InconsistentResult,
    InsufficientFunds,
    RpcCallFailed,
}

impl From<SendRawTransactionStatus> for Result<Option<String>, SendRawTransactionError> {
    fn from(status: SendRawTransactionStatus) -> Self {
        match status {
            SendRawTransactionStatus::Ok(Some(tx_hash)) => Ok(Some(tx_hash)),
            SendRawTransactionStatus::Ok(None) => Ok(None),
            SendRawTransactionStatus::NonceTooLow => Err(SendRawTransactionError::NonceTooLow),
            SendRawTransactionStatus::NonceTooHigh => Err(SendRawTransactionError::NonceTooHigh),
            SendRawTransactionStatus::InsufficientFunds => Err(SendRawTransactionError::InsufficientFunds),
        }
    }
}

impl From<MultiSendRawTransactionResult> for Result<Option<String>, SendRawTransactionError> {
    fn from(result: MultiSendRawTransactionResult) -> Self {
        match result {
            MultiSendRawTransactionResult::Consistent(SendRawTransactionResult::Ok(status)) => status.into(),
            MultiSendRawTransactionResult::Consistent(SendRawTransactionResult::Err(_)) => Err(SendRawTransactionError::RpcCallFailed),
            MultiSendRawTransactionResult::Inconsistent(_) => Err(SendRawTransactionError::InconsistentResult),
        }
    }
}

pub async fn send_raw_transaction(network_id: u32, tx: String) -> Result<Option<String>, SendRawTransactionError> {
    let rpc_providers = read_network_state(network_id, |s| s.rpc_services.clone());
    let cycles = 10_000_000_000;
    EVM_RPC
        .eth_send_raw_transaction(rpc_providers, None, tx, cycles)
        .await
        .map_err(|_| SendRawTransactionError::RpcCallFailed)
        .and_then(|(r,)| r.into())
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
