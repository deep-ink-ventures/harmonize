use ethers_core::abi::ethereum_types::{Address, U256, U64};
use ethers_core::types::transaction::eip1559::Eip1559TransactionRequest;
use ethers_core::types::{Bytes, Sign, Signature, H160};
use ethers_core::utils::keccak256;
use thiserror::Error;

use ic_cdk::api::management_canister::ecdsa::{
    ecdsa_public_key, sign_with_ecdsa, EcdsaPublicKeyArgument, SignWithEcdsaArgument, SignWithEcdsaResponse,
};
use std::str::FromStr;

use crate::state::read_state;

pub struct SignRequest {
    pub chain_id: Option<U64>,
    pub from: Option<H160>,
    pub to: Option<H160>,
    pub gas: U256,
    pub max_fee_per_gas: Option<U256>,
    pub max_priority_fee_per_gas: Option<U256>,
    pub value: Option<U256>,
    pub nonce: Option<U256>,
    pub data: Option<Vec<u8>>,
}

#[derive(Error, Debug)]
pub enum SignerError {
    #[error("Failed to sign the transaction with ECDSA")]
    EcdsaError,
    #[error("The public key is not initialized")]
    NotInitialized,
    #[error("Failed to parse the public key")]
    FailedToParsePublicKey,
    #[error("Invalid point representation")]
    InvalidPointRepresentation,
    #[error("Invalid signature representation")]
    InvalidSignatureRepresentation,
    #[error("Invalid recovery ID representation")]
    InvalidRecIdRepresentation,
    #[error("Failed to recover the public key from the signature")]
    FailedToRecoverKey
}

pub async fn sign_transaction(req: SignRequest) -> Result<String, SignerError> {
    const EIP1559_TX_ID: u8 = 2;

    let data = req.data.as_ref().map(|d| Bytes::from(d.clone()));

    let tx = Eip1559TransactionRequest {
        from: req.from,
        to: req.to.map(Into::into),
        gas: Some(req.gas),
        value: req.value,
        data,
        nonce: req.nonce,
        access_list: Default::default(),
        max_priority_fee_per_gas: req.max_priority_fee_per_gas,
        max_fee_per_gas: req.max_fee_per_gas,
        chain_id: req.chain_id,
    };

    let mut unsigned_tx_bytes = tx.rlp().to_vec();
    unsigned_tx_bytes.insert(0, EIP1559_TX_ID);

    let txhash = keccak256(&unsigned_tx_bytes);

    let key_id = read_state(|s| s.ecdsa_key_id.clone());

    let signature = sign_with_ecdsa(SignWithEcdsaArgument {
            message_hash: txhash.to_vec(),
            derivation_path: [].to_vec(),
            key_id,
        })
        .await
        .map_err(|_| SignerError::EcdsaError)?
        .0
        .signature;

    let pubkey = match read_state(|s| (s.ecdsa_pub_key.clone())) {
        Some(pubkey) => pubkey,
        None => return Err(SignerError::NotInitialized),
    };

    let signature = Signature {
        v: y_parity(&txhash, &signature, &pubkey)?,
        r: U256::from_big_endian(&signature[0..32]),
        s: U256::from_big_endian(&signature[32..64]),
    };

    let mut signed_tx_bytes = tx.rlp_signed(&signature).to_vec();
    signed_tx_bytes.insert(0, EIP1559_TX_ID);

    Ok(format!("0x{}", hex::encode(&signed_tx_bytes)))
}

/// Converts the public key bytes to an Ethereum address with a checksum.
pub fn pubkey_bytes_to_address(pubkey_bytes: &[u8]) -> Result<H160, SignerError> {
    use ethers_core::k256::elliptic_curve::sec1::ToEncodedPoint;
    use ethers_core::k256::PublicKey;

    let key = PublicKey::from_sec1_bytes(pubkey_bytes)
        .map_err(|_| SignerError::FailedToParsePublicKey)?;
    let point = key.to_encoded_point(false);
    let point_bytes = point.as_bytes();
    if point_bytes[0] != 0x04 {
        return Err(SignerError::InvalidPointRepresentation);
    }
    let hash = keccak256(&point_bytes[1..]);
    Ok(H160::from_slice(&hash[12..32]))
}

/// Computes the parity bit allowing to recover the public key from the signature.
fn y_parity(prehash: &[u8], sig: &[u8], pubkey: &[u8]) -> Result<u64, SignerError> {
    use ethers_core::k256::ecdsa::{RecoveryId, Signature, VerifyingKey};

    let orig_key = VerifyingKey::from_sec1_bytes(pubkey)
        .map_err(|_| SignerError::FailedToParsePublicKey)?;
    let signature = Signature::try_from(sig)
        .map_err(|_| SignerError::InvalidSignatureRepresentation)?;

    for parity in [0u8, 1] {
        let recovery_id = RecoveryId::try_from(parity)
            .map_err(|_| SignerError::InvalidRecIdRepresentation)?;
        let recovered_key = VerifyingKey::recover_from_prehash(prehash, &signature, recovery_id)
            .map_err(|_| SignerError::FailedToRecoverKey)?;
        if recovered_key == orig_key {
            return Ok(parity as u64);
        }
    }

    // TODO: handle this gracefully
    panic!(
        "failed to recover the parity bit from a signature; sig: {}, pubkey: {}",
        hex::encode(sig),
        hex::encode(pubkey)
    )
}
