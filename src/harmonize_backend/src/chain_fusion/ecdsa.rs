use ethers_core::types::H160;
use libsecp256k1::{Message, PublicKey, PublicKeyFormat, recover, RecoveryId, Signature};
use candid::Principal;
use ic_cdk::api::{call::RejectionCode, management_canister::ecdsa::{EcdsaKeyId, EcdsaPublicKeyArgument, EcdsaPublicKeyResponse, SignWithEcdsaArgument, SignWithEcdsaResponse}};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum EcdsaError {
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Invalid public key")]
    InvalidPublicKey,
    #[error("Invalid recovery ID")]
    InvalidRecoveryId,
    #[error("Invalid message")]
    InvalidMessage,
    #[error("Failed to recover public key")]
    RecoveryFailed,
    #[error("Failed to call the management canister")]
    CallFailed(RejectionCode, String)
}

const DEFAULT_ECDSA_SIGN_CYCLES : u64 = 10_000_000_000;

/// Compute the Keccak-256 hash of a given byte array.
///
/// # Arguments
///
/// * `bytes` - A byte array to be hashed.
///
/// # Returns
///
/// * `[u8; 32]` - A 32-byte array representing the Keccak-256 hash of the input.
///
/// # Example
///
/// ```
/// let data = b"hello";
/// let hash = keccak256(data);
/// println!("Keccak-256 hash: {:?}", hash);
/// ```
pub fn keccak256(bytes: &[u8]) -> [u8; 32] {
    use tiny_keccak::{Hasher, Keccak};
    let mut output = [0u8; 32];
    let mut hasher = Keccak::v256();
    hasher.update(bytes);
    hasher.finalize(&mut output);
    output
}

/// Find the recovery ID for a given ECDSA signature.
///
/// # Arguments
///
/// * `msg` - The original message in bytes.
/// * `sig` - The ECDSA signature in bytes.
/// * `known_pub_key` - The known public key in bytes.
///
/// # Returns
///
/// * `Option<u8>` - The recovery ID if found, else `None`.
///
/// # Example
///
/// ```
/// let message = b"example message";
/// let signature = sign_message(...); // Assume this is a valid signature
/// let public_key = ...; // Assume this is a valid public key
/// let recovery_id = find_recovery_id(message, &signature, public_key);
/// ```
pub fn find_recovery_id(msg: &[u8], sig: &[u8], known_pub_key: [u8; 65]) -> Result<u8, EcdsaError> {
    let message = Message::parse_slice(msg)
        .map_err(|_e| EcdsaError::InvalidSignature)?;

    // Try both possible recovery IDs
    for rec_id in [0u8, 1u8].iter() {
        let recovery_id = RecoveryId::parse(*rec_id)
            .map_err(|_e| EcdsaError::InvalidRecoveryId)?;

        let signature = Signature::parse_overflowing_slice(sig)
            .map_err(|_e| EcdsaError::InvalidSignature)?;

        // Attempt to recover the public key
        if let Ok(pubkey) = recover(&message, &signature, &recovery_id) {
            // Serialize and compare the recovered public key with the known public key
            if pubkey.serialize() == known_pub_key {
                return Ok(*rec_id);
            }
        }
    }
    Err(EcdsaError::InvalidPublicKey)
}

/// Asynchronously get the uncompressed public key.
///
/// # Arguments
///
/// * `wallet_id` - The wallet ID as a String.
/// * `key_id` - The EcdsaKeyId.
///
/// # Returns
///
/// * `Result<[u8; 65], String>` - The uncompressed public key or an error message.
///
/// # Example
///
/// ```
/// let key_id = get_ecdsa_key_id_from_env("test");
/// let public_key = get_public_key(wallet_id, key_id).await?;
/// ```
pub async fn get_public_key(key_id: EcdsaKeyId) -> Result<Vec<u8>, EcdsaError> {
    let ic = Principal::management_canister();

    let request = EcdsaPublicKeyArgument {
        canister_id: None,
        derivation_path: vec![],
        key_id
    };

    let (response,): (EcdsaPublicKeyResponse,) = ic_cdk::call(ic, "ecdsa_public_key", (request,))
        .await
        .map_err(|e| EcdsaError::CallFailed(e.0, e.1))?;

    Ok(response.public_key)
}

pub async fn get_compressed_public_key(key_id: EcdsaKeyId) -> Result<[u8; 65], EcdsaError> {
    let key = get_public_key(key_id).await?;
    Ok(PublicKey::parse_slice(&key, Some(PublicKeyFormat::Compressed))
        .map_err(|_e| EcdsaError::InvalidPublicKey)?
        .serialize())
}

/// Asynchronously sign a message with ECDSA.
///
/// # Arguments
///
/// * `wallet_id` - The wallet ID as a String.
/// * `message_hash` - The hash of the message to be signed.
/// * `key_id` - The EcdsaKeyId.
///
/// # Returns
///
/// * `Result<Vec<u8>, String>` - The ECDSA signature or an error message.
///
/// # Example
///
/// ```
/// let message = b"example message";
/// let message_hash = keccak256(message);
/// let key_id = get_ecdsa_key_id_from_env("test");
/// let signature = sign_message(wallet_id, message_hash.to_vec(), key_id).await?;
/// ```
pub async fn sign_message(message_hash: Vec<u8>, key_id: EcdsaKeyId) -> Result<Vec<u8>, EcdsaError> {
    let ic = Principal::management_canister();

    let request = SignWithEcdsaArgument {
        message_hash: message_hash.clone(),
        derivation_path: vec![],
        key_id: key_id.clone(),
    };

    let (response,): (SignWithEcdsaResponse,) = ic_cdk::api::call::call_with_payment(ic, "sign_with_ecdsa", (request,), DEFAULT_ECDSA_SIGN_CYCLES)
        .await
        .map_err(|e| EcdsaError::CallFailed(e.0, e.1))?;

    let mut signature = response.signature;

    let pub_key = get_compressed_public_key(key_id).await?;
    let recovery_id = find_recovery_id(&message_hash, &signature, pub_key)?;

    signature.push(recovery_id);
    Ok(signature)
}

/// Asynchronously get the Ethereum address from a wallet ID and key ID.
///
/// # Arguments
///
/// * `wallet_id` - The wallet ID as a String.
/// * `key_id` - The EcdsaKeyId.
///
/// # Returns
///
/// * `Result<String, String>` - The Ethereum address or an error message.
///
/// # Example
///
/// ```
/// let key_id = get_ecdsa_key_id_from_env("test");
/// let eth_address = get_eth_address(wallet_id, key_id).await?;
/// println!("Ethereum address: {}", eth_address);
/// ```
pub async fn get_eth_address(key_id: EcdsaKeyId) -> Result<H160, EcdsaError> {
    let pub_key = get_public_key(key_id).await?;
    let hash = keccak256(&pub_key);
    let mut result = [0u8; 20];
    result.copy_from_slice(&hash[12..]);
    Ok(H160::from(result))
}

/// Asynchronously validate an ECDSA signature.
///
/// # Arguments
///
/// * `message` - The original message in bytes.
/// * `signature` - The ECDSA signature in bytes.
/// * `wallet_id` - The wallet ID as a String.
/// * `key_id` - The EcdsaKeyId.
///
/// # Returns
///
/// * `Result<bool, String>` - `true` if the signature is valid, else `false`.
///
/// # Example
///
/// ```
/// let message = b"example message";
/// let signature = sign_message(...); // Assume this is a valid signature
/// let key_id = get_ecdsa_key_id_from_env("test");
/// let is_valid = is_signature_valid(message.to_vec(), signature, wallet_id, key_id).await?;
/// ```
pub async fn is_signature_valid(
    message: &[u8],
    signature: &[u8],
    key_id: EcdsaKeyId
) -> Result<bool, EcdsaError> {
    let pub_key = get_compressed_public_key(key_id).await?;

    let message = Message::parse_slice(message)
        .map_err(|_e| EcdsaError::InvalidMessage)?;

    let recovery_id = RecoveryId::parse(signature[64])
        .map_err(|_e| EcdsaError::InvalidRecoveryId)?;

    let signature = Signature::parse_overflowing_slice(&signature[..64])
        .map_err(|_e| EcdsaError::InvalidSignature)?;

    let signer = recover(&message, &signature, &recovery_id)
        .map_err(|_e| EcdsaError::RecoveryFailed)?;

    Ok(signer.serialize() == pub_key)
}