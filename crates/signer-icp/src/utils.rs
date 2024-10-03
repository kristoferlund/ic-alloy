use core::panic;

use alloy_primitives::{keccak256, Address, B256};
use alloy_signer::k256::{
    ecdsa::{RecoveryId, Signature, VerifyingKey},
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

/// ...
pub fn y_parity(hash: &B256, signature: &[u8], public_key: &Vec<u8>) -> u64 {
    let verifying_key = VerifyingKey::from_sec1_bytes(public_key.as_slice()).unwrap();
    let signature = Signature::try_from(signature).unwrap();
    for parity in [0u8, 1] {
        let recid = RecoveryId::try_from(parity).unwrap();
        let recovered_key = VerifyingKey::recover_from_prehash(&hash.0, &signature, recid).unwrap();
        if recovered_key == verifying_key {
            return parity as u64;
        }
    }

    panic!("Unable to recover the parity bit");
}
