//! ICP signer implementation for Alloy

use std::fmt;

use crate::{
    ecdsa_key_id,
    utils::{address_for_public_key, get_public_key, y_parity},
};
use alloy_consensus::SignableTransaction;
use alloy_primitives::{hex, Address, ChainId, B256};
use alloy_signer::{k256::elliptic_curve, Result, Signature, Signer};
use async_trait::async_trait;

use ic_cdk::api::{
    call::RejectionCode,
    management_canister::ecdsa::{sign_with_ecdsa, EcdsaKeyId, SignWithEcdsaArgument},
};

/// An Icp signer implementation for Alloy
#[derive(Clone)]
pub struct IcpSigner {
    derivation_path: Vec<Vec<u8>>,
    key_id: EcdsaKeyId,
    public_key: Vec<u8>,
    address: Address,
    chain_id: Option<ChainId>,
}

impl fmt::Debug for IcpSigner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IcpSigner")
            .field("derivation_path", &self.derivation_path)
            .field("key_id", &self.key_id)
            .field("public_key", &hex::encode(&self.public_key))
            .field("address", &self.address)
            .field("chain_id", &self.chain_id)
            .finish()
    }
}

/// Errors thrown by [`IcpSigner`].
#[derive(Debug, thiserror::Error)]
pub enum IcpSignerError {
    /// Icp call errors  
    #[error("ICP call error, code: {0:?}, message: {1}")]
    IcpCall(RejectionCode, String),

    /// EllipticCurve errors
    #[error(transparent)]
    EllipticCurve(#[from] elliptic_curve::Error),
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl alloy_network::TxSigner<Signature> for IcpSigner {
    fn address(&self) -> Address {
        self.address
    }

    async fn sign_transaction(
        &self,
        tx: &mut dyn SignableTransaction<Signature>,
    ) -> Result<Signature> {
        let hash = tx.signature_hash();
        self.sign_hash_inner(&hash).await.map_err(alloy_signer::Error::other)
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Signer for IcpSigner {
    async fn sign_hash(&self, hash: &B256) -> Result<Signature> {
        self.sign_hash_inner(hash).await.map_err(alloy_signer::Error::other)
    }

    fn address(&self) -> Address {
        self.address
    }

    fn chain_id(&self) -> Option<ChainId> {
        self.chain_id
    }

    fn set_chain_id(&mut self, chain_id: Option<ChainId>) {
        self.chain_id = chain_id;
    }
}

impl IcpSigner {
    /// Instantiate a new signer from an existing `Client` and key ID.
    ///
    /// Retrieves the public key from AWS and calculates the Ethereum address.
    pub async fn new(
        derivation_path: Vec<Vec<u8>>,
        ecdsa_key_name: &str,
        chain_id: Option<ChainId>,
    ) -> Result<Self, IcpSignerError> {
        let key_id = ecdsa_key_id(ecdsa_key_name);
        let public_key = get_public_key(&derivation_path, &key_id).await?;
        let address = address_for_public_key(&public_key).await?;
        Ok(Self { derivation_path, key_id, public_key, address, chain_id })
    }

    async fn sign_hash_inner(&self, hash: &B256) -> Result<Signature> {
        let (signature_response,) = sign_with_ecdsa(SignWithEcdsaArgument {
            message_hash: hash.to_vec(),
            derivation_path: self.derivation_path.clone(),
            key_id: self.key_id.clone(),
        })
        .await
        .unwrap();

        Ok(Signature::from_bytes_and_parity(
            &signature_response.signature,
            y_parity(hash, &signature_response.signature, &self.public_key),
        )
        .unwrap())
    }

    /// ...
    pub const fn public_key(&self) -> &Vec<u8> {
        &self.public_key
    }

    /// ...    
    pub const fn key_id(&self) -> &EcdsaKeyId {
        &self.key_id
    }

    /// ...
    pub const fn derivation_path(&self) -> &Vec<Vec<u8>> {
        &self.derivation_path
    }
}
