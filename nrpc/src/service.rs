use futures::Stream;
use core::marker::Unpin;

#[cfg(feature = "client-send")]
pub type ServiceClientStream<'a, T> = Box<dyn Stream<Item=Result<T, ServiceError>> + Unpin + Send + 'a>;

#[cfg(not(feature = "client-send"))]
pub type ServiceClientStream<'a, T> = Box<dyn Stream<Item=Result<T, ServiceError>> + Unpin + 'a>;

#[cfg(feature = "server-send")]
pub type ServiceServerStream<'a, T> = Box<dyn Stream<Item=Result<T, ServiceError>> + Unpin + Send + 'a>;

#[cfg(not(feature = "server-send"))]
pub type ServiceServerStream<'a, T> = Box<dyn Stream<Item=Result<T, ServiceError>> + Unpin + 'a>;

#[cfg_attr(feature = "server-send", async_trait::async_trait)]
#[cfg_attr(not(feature = "server-send"), async_trait::async_trait(?Send))]
pub trait ServerService<'b> {
    fn descriptor(&self) -> &'static str;

    async fn call<'a: 'b>(
        &mut self,
        method: &str,
        input: ServiceServerStream<'a, bytes::Bytes>,
    ) -> Result<ServiceServerStream<'a, bytes::Bytes>, ServiceError>;
}

#[cfg_attr(feature = "client-send", async_trait::async_trait)]
#[cfg_attr(not(feature = "client-send"), async_trait::async_trait(?Send))]
pub trait ClientHandler<'b> {
    async fn call<'a: 'b>(
        &self,
        package: &str,
        service: &str,
        method: &str,
        input: ServiceClientStream<'a, bytes::Bytes>,
    ) -> Result<ServiceClientStream<'a, bytes::Bytes>, ServiceError>;
}

pub trait ClientService {
    fn descriptor(&self) -> &'static str;
}

#[derive(Debug)]
pub enum ServiceError {
    Encode(prost::EncodeError),
    Decode(prost::DecodeError),
    MethodNotFound,
    ServiceNotFound,
    Method(Box<dyn std::error::Error + Send + 'static>),
    StreamLength {
        want: u64,
        got: u64,
    }
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encode(en) => write!(f, "Encode error: {}", en),
            Self::Decode(de) => write!(f, "Decode error: {}", de),
            Self::MethodNotFound => write!(f, "Method not found error"),
            Self::ServiceNotFound => write!(f, "Service not found error"),
            Self::Method(e) => write!(f, "Method error: {}", e),
            Self::StreamLength{ want, got } => write!(f, "Stream length error: wanted {}, got {}", want, got),
        }
    }
}

impl std::convert::From<prost::EncodeError> for ServiceError {
    fn from(value: prost::EncodeError) -> Self {
        Self::Encode(value)
    }
}

impl std::convert::From<prost::DecodeError> for ServiceError {
    fn from(value: prost::DecodeError) -> Self {
        Self::Decode(value)
    }
}

impl std::convert::From<Box<dyn std::error::Error + Send>> for ServiceError {
    fn from(value: Box<dyn std::error::Error + Send>) -> Self {
        Self::Method(value)
    }
}

impl std::error::Error for ServiceError {}
