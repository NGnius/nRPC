#[async_trait::async_trait]
pub trait ServerService {
    fn descriptor(&self) -> &'static str;

    async fn call(&mut self,
            method: &str,
            input: bytes::Bytes,
            output: &mut bytes::BytesMut) -> Result<(), ServiceError>;
}

#[async_trait::async_trait]
pub trait ClientHandler {
    async fn call(&mut self,
            package: &str,
            service: &str,
            method: &str,
            input: bytes::Bytes,
            output: &mut bytes::BytesMut) -> Result<(), ServiceError>;
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
    Method(Box<dyn std::error::Error>),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Encode(en) => write!(f, "Encode error: {}", en),
            Self::Decode(de) => write!(f, "Decode error: {}", de),
            Self::MethodNotFound => write!(f, "Method not found error"),
            Self::ServiceNotFound => write!(f, "Service not found error"),
            Self::Method(e) => write!(f, "Method error: {}", e),
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

impl std::convert::From<Box<dyn std::error::Error>> for ServiceError {
    fn from(value: Box<dyn std::error::Error>) -> Self {
        Self::Method(value)
    }
}

impl std::error::Error for ServiceError {}
