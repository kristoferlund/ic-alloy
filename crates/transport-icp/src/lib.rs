#![doc = include_str!("../README.md")]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/alloy.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/alloy-rs/core/main/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg, doc_auto_cfg))]
#![allow(
    missing_docs,
    missing_debug_implementations,
    non_upper_case_globals,
    non_snake_case,
    clippy::enum_variant_names,
    clippy::large_enum_variant
)]
mod evm_rpc;

use alloy_json_rpc::{RequestPacket, ResponsePacket};
use alloy_transport::{TransportError, TransportFut};
use ic_cdk::api::call::{call_with_payment128, CallResult};
use std::task;
use tower::Service;

pub use evm_rpc::*;

const ETH_DEFAULT_CALL_CYCLES: u128 = 50_000_000_000;

/// Connection details for an ICP transport.
#[derive(Clone, Debug)]
#[doc(hidden)]
pub struct IcpConnect {
    rpc_service: RpcService,
}

impl IcpConnect {
    /// Create a new [`IcpConnect`] with the given URL.
    pub const fn new(rpc_service: RpcService) -> Self {
        Self { rpc_service }
    }

    /// Get a reference to the rpc service.
    pub const fn rcp_service(&self) -> &RpcService {
        &self.rpc_service
    }
}

/// An ICP transport.
///
/// The user must provide an [`RpcService`] that specifies what
/// chain and provider to use
#[derive(Clone, Debug)]
pub struct IcpTransport {
    rpc_service: RpcService,
}

impl IcpTransport {
    /// Create a new [`IcpTransport`] transport with the specified [`RpcService`].
    pub const fn with_service(rpc_service: RpcService) -> Self {
        Self { rpc_service }
    }

    /// Set the [`RpcService`] for this transport.
    pub fn set_rpc_service(&mut self, rpc_service: RpcService) {
        self.rpc_service = rpc_service;
    }

    /// Get a reference to the rpc service.
    pub const fn rpc_service(&self) -> &RpcService {
        &self.rpc_service
    }

    /// Check if the transport is local. Always `false` for now.
    pub const fn is_local(&self) -> bool {
        // Currently always returns false. We could add a check here to see
        // which DFX_NETWORK is being used and return true if it's local.
        // Not sure if that's necessary though.
        false
    }

    /// Make an EVM RPC request by calling the `request` method on the EVM RPC canister.
    fn request_icp(&self, request_packet: RequestPacket) -> TransportFut<'static> {
        let rpc_service = self.rpc_service.clone();
        Box::pin(async move {
            let serialized_request = request_packet.serialize().map_err(TransportError::ser_err)?;
            let call_result: CallResult<(RequestResult,)> = call_with_payment128(
                evm_rpc::evm_rpc.0,
                "request",
                (rpc_service, serialized_request.to_string(), 2_000_000_u64),
                ETH_DEFAULT_CALL_CYCLES,
            )
            .await;

            match call_result {
                Ok((request_result,)) => match request_result {
                    RequestResult::Ok(ok_result) => serde_json::from_str(&ok_result)
                        .map_err(|err| TransportError::deser_err(err, &ok_result)),
                    RequestResult::Err(rpc_error) => {
                        Err(TransportError::ErrorResp(alloy_json_rpc::ErrorPayload {
                            code: 6, // RPC error
                            message: format!("{:?}", rpc_error),
                            data: None,
                        }))
                    }
                },
                Err(err) => Err(TransportError::ErrorResp(alloy_json_rpc::ErrorPayload {
                    code: err.0 as i64,
                    message: err.1,
                    data: None,
                })),
            }
        })
    }
}

impl Service<RequestPacket> for IcpTransport {
    type Response = ResponsePacket;
    type Error = TransportError;
    type Future = TransportFut<'static>;

    #[inline]
    fn poll_ready(&mut self, _cx: &mut task::Context<'_>) -> task::Poll<Result<(), Self::Error>> {
        // The IcpTransport is always ready to make requests.
        task::Poll::Ready(Ok(()))
    }

    #[inline]
    fn call(&mut self, req: RequestPacket) -> Self::Future {
        self.request_icp(req)
    }
}
