use std::collections::{HashMap, HashSet};
use candid::{CandidType, Principal};
use ethers_core::types::H160;
use crate::read_state;
use thiserror::Error;
use ::ecdsa::RecoveryId;
use ethers_core::k256::ecdsa::VerifyingKey;
use ecdsa::Signature as Secp256k1Signature;
use crate::state::mutate_state;
use sha3::{Digest, Keccak256};
use hex::decode;
use serde_bytes::ByteBuf;
use crate::declarations::ic_siwe_provider::{ic_siwe_provider, GetAddressResponse};

#[derive(Debug, Error, CandidType)]
pub enum SignInError {
    #[error("Call error: {0}")]
    CallError(String),
    #[error("No session: {0}")]
    NoSession(String),
    #[error("Wallet already linked")]
    WalletAlreadyLinked,
    #[error("Invalid address")]
    InvalidAddress,
}

pub async fn get_siwe_session_address() -> Result<H160, SignInError> {
    let response = ic_siwe_provider
        .get_address(ByteBuf::from(ic_cdk::caller().as_slice()))
        .await;

    let address = match response {
        Ok((GetAddressResponse::Ok(address),)) => address,
        Ok((GetAddressResponse::Err(e),)) => return Err(SignInError::NoSession(e)),
        Err(e) => return Err(SignInError::CallError(e.1))
    };

    Ok(address.parse().map_err(|_| SignInError::InvalidAddress)?)
}