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

#[derive(Clone, Debug)]
pub struct SignatureChallenge {
    pub message: String,
    pub expires_at: u64,
}

impl SignatureChallenge {
    pub fn new(expires_at: u64) -> Self {
        Self {
            message: Self::generate(),
            expires_at,
        }
    }
    pub fn generate() -> String {
        "Sign me".to_string()
    }
}

#[derive(Error, Debug, CandidType)]
pub enum AccessControlError {
    #[error("Challenge expired")]
    ChallengeExpired,
    #[error("Challenge not found")]
    ChallengeNotFound,
    #[error("Wallet is already linked")]
    WalletAlreadyLinked,
    #[error("Ecdsa error")]
    EcdsaError,
    #[error("Access denied")]
    AccessDenied,
    #[error("Invalid signature")]
    SignatureInvalid(#[from] SignatureValidationError),
}

#[derive(Default)]
pub struct AccessControl {
    wallets_to_principals: HashMap<H160, HashSet<Principal>>,
    principals_to_wallets: HashMap<Principal, HashSet<H160>>,
    challenges: HashMap<(Principal, H160), SignatureChallenge>,
}

impl AccessControl {
    pub fn new() -> Self {
        Self {
            wallets_to_principals: HashMap::new(),
            principals_to_wallets: HashMap::new(),
            challenges: HashMap::new(),
        }
    }

    pub fn has_access(&self, principal: Principal, wallet: H160) -> bool {
        self.wallets_to_principals
            .get(&wallet)
            .map(|principals| principals.contains(&principal))
            .unwrap_or(false)
    }

    pub fn get_wallets(&self, principal: Principal) -> Option<&HashSet<H160>> {
        self.principals_to_wallets.get(&principal)
    }

    pub fn get_principals(&self, wallet: H160) -> Option<&HashSet<Principal>> {
        self.wallets_to_principals.get(&wallet)
    }

    pub fn link_wallet(&mut self, principal: Principal, wallet: H160) {
        self.wallets_to_principals.entry(wallet).or_insert_with(HashSet::new).insert(principal);
        self.principals_to_wallets.entry(principal).or_insert_with(HashSet::new).insert(wallet);
    }

    pub fn unlink_wallet(&mut self, principal: Principal, wallet: H160) {
        self.wallets_to_principals.remove(&wallet);
        if let Some(wallets) = self.principals_to_wallets.get_mut(&principal) {
            wallets.remove(&wallet);
        }
    }

    pub fn insert_challenge(&mut self, principal: Principal, wallet: H160, challenge: SignatureChallenge) {
        self.challenges.insert((principal, wallet), challenge);
    }

    pub fn get_challenge(&mut self, principal: Principal, wallet: H160) -> Option<SignatureChallenge> {
        self.challenges.remove(&(principal, wallet))
    }
}

#[derive(Debug, Error, CandidType)]
pub enum SignInError {
    #[error("Call error: {0}")]
    CallError(String),
    #[error("No session: {0}")]
    NoSession(String),
    #[error("Wallet already linked")]
    WalletAlreadyLinked,
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

    Ok(address.parse().expect("address should be valid"))
}

pub async fn sign_in_with_ethereum() -> Result<(), SignInError> {
    let address = get_siwe_session_address().await?;
    let principal = ic_cdk::caller();
    let is_linked = read_state(|s|
        s.access_control
            .get_principals(address)
            .map(|principals| principals.contains(&principal))
            .unwrap_or(false)
    );
    if !is_linked {
        mutate_state(|s| s.access_control.link_wallet(principal, address));
    }
    Ok(())
}

#[derive(Error, Debug, CandidType)]
pub enum SignatureValidationError {
    #[error("Signature decode failed")]
    SignatureDecodeFailed,
    #[error("Recovery id parse failed")]
    RecoveryIdParseFailed,
    #[error("Verifying key recovery failed")]
    VerifyingKeyRecoveryFailed,
    #[error("Address does not match")]
    AddressDoesNotMatch,
}

fn verify_eth_signature(address: H160, message: &str, signature: &str) -> Result<(), SignatureValidationError> {
    ic_cdk::println!("Verifying signature: address={}, message={}, signature={}", address, message, signature);
    // Prefix and hash the message as Ethereum does
    let eth_message = format!("\x19Ethereum Signed Message:\n{}{}", message.len(), message);
    let message_hash = Keccak256::digest(eth_message.as_bytes());

    let signature_bytes = match decode(&signature[2..]) {
        Ok(bytes) => bytes,
        Err(e) => {
            ic_cdk::println!("Failed to decode signature: {}", e);
            return Err(SignatureValidationError::SignatureDecodeFailed)
        },
    };

    let signature = match Secp256k1Signature::from_slice(&signature_bytes[..64]) {
        Ok(sig) => sig,
        Err(e) => {
            ic_cdk::println!("Failed to parse signature: {}", e);
            return Err(SignatureValidationError::SignatureDecodeFailed)
        },
    };

    let recovery_id = match signature_bytes[64] as u8 {
        id if id == 27 || id == 28 => id - 27,
        id if id == 0 || id == 1 => id,
        id => {
            ic_cdk::println!("Invalid recovery id: {}", id);
            return Err(SignatureValidationError::RecoveryIdParseFailed)
        }
    };

    let recovery_id = match RecoveryId::from_byte(recovery_id) {
        Some(id) => id,
        None => {
            ic_cdk::println!("Failed to parse recovery id: {}", recovery_id);
            return Err(SignatureValidationError::RecoveryIdParseFailed)
        }
    };

    let verifying_key = match VerifyingKey::recover_from_prehash(&message_hash, &signature, recovery_id) {
        Ok(key) => key,
        Err(e) => {
            ic_cdk::println!("Failed to recover verifying key: {}", e);
            return Err(SignatureValidationError::VerifyingKeyRecoveryFailed)
        },
    };

    let public_key = verifying_key.to_encoded_point(false);
    let public_key_hash = Keccak256::digest(&public_key.as_bytes()[1..]); // Skip the 0x04 prefix
    let public_key_address = H160::from_slice(&public_key_hash[12..]); // Take the last 20 bytes

    if address == public_key_address {
        Ok(())
    } else {
        ic_cdk::println!("Address does not match: expected={}, actual={}", address, public_key_address);
        Err(SignatureValidationError::AddressDoesNotMatch)
    }
}

// Public API

pub async fn sign_in_challenge(address: H160) -> Result<String, AccessControlError> {
    // TODO: Make this configurable
    let nanos = 1_000_000_000;
    let expiration_nanos = 5 * 60 * nanos; // 5 minutes

    let principal = ic_cdk::caller();
    let now = ic_cdk::api::time();
    let key = (principal, address);

    let existing_link = read_state(|s| s.access_control.wallets_to_principals.get(&address).cloned());
    if let Some(_) = existing_link {
        return Err(AccessControlError::WalletAlreadyLinked);
    }

    let existing_challenge = read_state(|s| s.access_control.challenges.get(&key).cloned());
    match existing_challenge {
        Some(challenge) => {
            if challenge.expires_at < now {
                // Challenge is expired - create a new one
                let challenge = SignatureChallenge::new(now + expiration_nanos);
                let message = challenge.message.clone();
                mutate_state(|s| s.access_control.insert_challenge(principal, address, challenge));
                return Ok(message)
            } else {
                // Challenge is still valid
                return Ok(challenge.message);
            }
        }
        None => {
            // Create new challenge
            let challenge = SignatureChallenge::new(now + expiration_nanos);
            let message = challenge.message.clone();
            mutate_state(|s| s.access_control.insert_challenge(principal, address, challenge));
            return Ok(message)
        }
    }
}

pub async fn sign_in_with_signature(address: H160, signature: String) -> Result<bool, AccessControlError> {
    let principal = ic_cdk::caller();

    let challenge = mutate_state(|s| s.access_control.get_challenge(principal, address))
        .ok_or_else(|| AccessControlError::ChallengeNotFound)?;

    if challenge.expires_at < ic_cdk::api::time() {
        return Err(AccessControlError::ChallengeExpired);
    }

    match verify_eth_signature(address, challenge.message.as_str(), signature.as_str()) {
        Ok(_) => {
            mutate_state(|s| {
                s.access_control.link_wallet(principal, address);
            });
            Ok(true)
        }
        Err(e) => {
            Err(AccessControlError::SignatureInvalid(e))
        },
    }
}

pub fn has_access(principal: Principal, wallet: H160) -> Result<(), AccessControlError> {
    let access = read_state(|s| {
        s.access_control.has_access(principal, wallet)
    });
    if access {
        Ok(())
    } else {
        Err(AccessControlError::AccessDenied)
    }
}

pub fn caller_has_access(wallet: H160) -> Result<(), AccessControlError> {
    let principal = ic_cdk::caller();
    has_access(principal, wallet)
}