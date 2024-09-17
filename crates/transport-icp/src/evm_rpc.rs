use candid::{self, CandidType, Deserialize, Principal};
use ic_cdk::api::call::CallResult as Result;

#[derive(Debug, CandidType, Deserialize, Clone)]
pub enum EthSepoliaService {
    Alchemy,
    BlockPi,
    PublicNode,
    Ankr,
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub enum L2MainnetService {
    Alchemy,
    BlockPi,
    PublicNode,
    Ankr,
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub struct HttpHeader {
    pub value: String,
    pub name: String,
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub struct RpcApi {
    pub url: String,
    pub headers: Option<Vec<HttpHeader>>,
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub enum EthMainnetService {
    Alchemy,
    BlockPi,
    Cloudflare,
    PublicNode,
    Ankr,
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub enum ProviderError {
    TooFewCycles { expected: candid::Nat, received: candid::Nat },
    MissingRequiredProvider,
    ProviderNotFound,
    NoPermission,
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub enum ValidationError {
    CredentialPathNotAllowed,
    HostNotAllowed(String),
    CredentialHeaderNotAllowed,
    UrlParseError(String),
    Custom(String),
    InvalidHex(String),
}

#[derive(Debug, CandidType, Deserialize, Clone, Copy)]
pub enum RejectionCode {
    NoError,
    CanisterError,
    SysTransient,
    DestinationInvalid,
    Unknown,
    SysFatal,
    CanisterReject,
}

impl RejectionCode {
    pub const fn as_i64(&self) -> i64 {
        *self as i64
    }
}

impl From<RejectionCode> for i64 {
    fn from(code: RejectionCode) -> Self {
        code.as_i64()
    }
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub enum HttpOutcallError {
    IcError { code: RejectionCode, message: String },
    InvalidHttpJsonRpcResponse { status: u16, body: String, parsingError: Option<String> },
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub enum RpcError {
    JsonRpcError(JsonRpcError),
    ProviderError(ProviderError),
    ValidationError(ValidationError),
    HttpOutcallError(HttpOutcallError),
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub enum RpcService {
    EthSepolia(EthSepoliaService),
    BaseMainnet(L2MainnetService),
    Custom(RpcApi),
    OptimismMainnet(L2MainnetService),
    ArbitrumOne(L2MainnetService),
    EthMainnet(EthMainnetService),
    Chain(u64),
    Provider(u64),
}

#[derive(Debug, CandidType, Deserialize, Clone)]
pub enum RequestResult {
    Ok(String),
    Err(RpcError),
}

pub struct EvmRpc(pub Principal);
impl EvmRpc {
    pub async fn request(
        &self,
        arg0: RpcService,
        arg1: String,
        arg2: u64,
    ) -> Result<(RequestResult,)> {
        ic_cdk::call(self.0, "request", (arg0, arg1, arg2)).await
    }
}
pub const CANISTER_ID: Principal = Principal::from_slice(&[0, 0, 0, 0, 2, 48, 0, 204, 1, 1]); // 7hfb6-caaaa-aaaar-qadga-cai
pub const evm_rpc: EvmRpc = EvmRpc(CANISTER_ID);
