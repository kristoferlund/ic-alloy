use alloy_primitives::{keccak256, Address};
use alloy_signer::k256::{
    elliptic_curve::{sec1::ToEncodedPoint, PublicKey},
    Secp256k1,
};
use ic_cdk::api::management_canister::ecdsa::{
    self, ecdsa_public_key, EcdsaKeyId, EcdsaPublicKeyArgument,
};

use crate::signer::IcpSignerError;

/// Construct a `EcdsaKeyId` on the `Secp256k1` curve with the specified name
pub fn ecdsa_key_id(name: &str) -> EcdsaKeyId {
    EcdsaKeyId { curve: ecdsa::EcdsaCurve::Secp256k1, name: name.to_string() }
}

/// Return a SEC1 encoded ECDSA public key for current canister using the given derivation path and key id.
pub async fn get_public_key(
    derivation_path: &[Vec<u8>],
    key_id: &EcdsaKeyId,
) -> Result<Vec<u8>, IcpSignerError> {
    let response = ecdsa_public_key(EcdsaPublicKeyArgument {
        canister_id: None,
        derivation_path: derivation_path.to_vec(),
        key_id: key_id.clone(),
    })
    .await
    .map_err(|(code, msg)| IcpSignerError::IcpCall(code, msg))?;
    Ok(response.0.public_key)
}

/// Returns the Ethereum address for the given public key.
pub async fn address_for_public_key(public_key: &[u8]) -> Result<Address, IcpSignerError> {
    let key: PublicKey<Secp256k1> = PublicKey::from_sec1_bytes(public_key)?;
    let point = key.to_encoded_point(false);
    let point_bytes = point.as_bytes();
    let hash = keccak256(&point_bytes[1..]);
    Ok(Address::from_slice(&hash[12..32]))
}

/// Calculates the y_parity bit of an ETH signature
pub const fn y_parity(signature: &[u8]) -> bool {
    let v = signature[64]; // Will panic if signature has wrong length
    match v {
        27 | 28 => (v - 27) != 0,
        _ => {
            // For EIP-155 signatures, v = CHAIN_ID * 2 + {35,36}
            // Y parity is determined by whether v is even or odd
            (v % 2) != 0
        }
    }
}
