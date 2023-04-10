mod builder;
mod service_gen;

pub use builder::{compile, compile_servers, compile_clients};
pub(crate) use service_gen::ProtobufServiceGenerator;
