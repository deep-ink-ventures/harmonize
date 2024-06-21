use libsecp256k1::{Message, PublicKey, PublicKeyFormat, recover, RecoveryId, Signature};
use candid::Principal;
use ic_cdk::api::management_canister::ecdsa::{EcdsaCurve, EcdsaKeyId, EcdsaPublicKeyArgument, EcdsaPublicKeyResponse, SignWithEcdsaArgument, SignWithEcdsaResponse};

const DEFAULT_ECDSA_SIGN_CYCLES : u64 = 10_000_000_000;

/// Get the EcdsaKeyId from the environment.
///
/// # Arguments
///
/// * `env` - The environment name, e.g. "test", "production", etc.
///
/// # Returns
///
/// * `EcdsaKeyId` - The EcdsaKeyId.
///
/// # Example
///
/// ```
/// use ic_cdk::api::management_canister::ecdsa::EcdsaKeyId;
/// use blend_safe_backend::ecdsa::get_ecdsa_key_id_from_env;
///
/// let key_id = get_ecdsa_key_id_from_env("test");
/// assert_eq!(key_id.curve, EcdsaCurve::Secp256k1);
/// assert_eq!(key_id.name, "test_key_1");
/// ```
pub fn get_ecdsa_key_id_from_env(env: &str) -> EcdsaKeyId {
    EcdsaKeyId {
        curve: EcdsaCurve::Secp256k1,
        name: match env {
            "production" => "key_1",
            "test" => "test_key_1",
            _ => "dfx_test_key",
        }.to_string(),
    }
}

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
pub fn find_recovery_id(msg: &[u8], sig: &[u8], known_pub_key: [u8; 65]) -> Option<u8> {
    let message = Message::parse_slice(msg).expect("Invalid message");

    // Try both possible recovery IDs
    for rec_id in [0u8, 1u8].iter() {
        let recovery_id = RecoveryId::parse(*rec_id).expect("Invalid recovery ID");
        let signature = Signature::parse_overflowing_slice(sig).expect("Invalid signature");

        // Attempt to recover the public key
        if let Ok(pubkey) = recover(&message, &signature, &recovery_id) {
            // Serialize and compare the recovered public key with the known public key
            if pubkey.serialize() == known_pub_key {
                return Some(*rec_id);
            }
        }
    }
    None
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
/// let wallet_id = "my_wallet_id".to_string();
/// let key_id = get_ecdsa_key_id_from_env("test");
/// let public_key = get_public_key(wallet_id, key_id).await?;
/// ```
pub async fn get_public_key(
    wallet_id: String,
    key_id: EcdsaKeyId
) -> Result<[u8; 65], String> {
    let ic = Principal::management_canister();

    let request = EcdsaPublicKeyArgument {
        canister_id: None,
        derivation_path: vec![wallet_id.as_bytes().to_vec()],
        key_id
    };
    let (res,): (EcdsaPublicKeyResponse,) = ic_cdk::call(ic, "ecdsa_public_key", (request,))
        .await
        .map_err(|e| format!("Failed to call ecdsa_public_key {}", e.1))?;

    let uncompressed_pub_key = match PublicKey::parse_slice(&res.public_key, Some(PublicKeyFormat::Compressed)) {
        Ok(key) => { key.serialize() },
        Err(_) => { return Err("decompression public key failed: ".to_string()); },
    };
    Ok(uncompressed_pub_key)
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
/// let wallet_id = "my_wallet_id".to_string();
/// let message = b"example message";
/// let message_hash = keccak256(message);
/// let key_id = get_ecdsa_key_id_from_env("test");
/// let signature = sign_message(wallet_id, message_hash.to_vec(), key_id).await?;
/// ```
pub async fn sign_message(wallet_id: String, message_hash: Vec<u8>, key_id: EcdsaKeyId) -> Result<Vec<u8>, String> {
    let ic = Principal::management_canister();
    let derivation_path = vec![wallet_id.as_bytes().to_vec()];
    let request = SignWithEcdsaArgument {
        message_hash: message_hash.clone(),
        derivation_path: derivation_path.clone(),
        key_id: key_id.clone(),
    };

    let (res,): (SignWithEcdsaResponse,) =
        ic_cdk::api::call::call_with_payment(ic, "sign_with_ecdsa", (request,), DEFAULT_ECDSA_SIGN_CYCLES)
            .await
            .map_err(|e| format!("Failed to call sign_with_ecdsa {}", e.1))?;

    let mut signature = res.signature;
    let pub_key = get_public_key(
        wallet_id,
        key_id
    ).await?;
    let rec_id = find_recovery_id(&message_hash, &signature, pub_key).unwrap();
    signature.push(rec_id);
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
/// let wallet_id = "my_wallet_id".to_string();
/// let key_id = get_ecdsa_key_id_from_env("test");
/// let eth_address = get_eth_address(wallet_id, key_id).await?;
/// println!("Ethereum address: {}", eth_address);
/// ```
pub async fn get_eth_address(wallet_id: String, key_id: EcdsaKeyId) -> Result<String, String> {
    let pub_key = get_public_key(wallet_id, key_id).await?.to_vec();
    let hash = keccak256(&pub_key[1..65]);
    let mut result = [0u8; 20];
    result.copy_from_slice(&hash[12..]);
    Ok(format!("0x{}", hex::encode(result)))
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
/// let wallet_id = "my_wallet_id".to_string();
/// let key_id = get_ecdsa_key_id_from_env("test");
/// let is_valid = is_signature_valid(message.to_vec(), signature, wallet_id, key_id).await?;
/// ```
pub async fn is_signature_valid(message: Vec<u8>, signature: Vec<u8>, wallet_id: String, key_id: EcdsaKeyId) -> Result<bool, String> {
    let pub_key = get_public_key(wallet_id, key_id).await?.to_vec();

    let recovery_id = signature[64];
    let signature_without_rec_id = signature[..64].to_vec();

    let message_obj = Message::parse_slice(message.as_slice()).expect("Invalid message");
    let recovery_obj = RecoveryId::parse(recovery_id).expect("Invalid recovery ID");
    let signature_obj = Signature::parse_overflowing_slice(signature_without_rec_id.as_slice()).expect("Invalid signature");
    let recovered_address = recover(&message_obj, &signature_obj, &recovery_obj).unwrap();

    Ok(recovered_address.serialize().to_vec() == pub_key)
}