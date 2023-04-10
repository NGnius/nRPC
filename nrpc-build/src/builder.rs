use std::path::Path;
use std::convert::AsRef;
use std::iter::IntoIterator;

use prost_build::Config;

/// Compile proto files into Rust with server and client implementations
pub fn compile(
    files: impl IntoIterator<Item = impl AsRef<Path>>,
    includes: impl IntoIterator<Item = impl AsRef<Path>>
) {
    let file_descriptors = protox::compile(files, includes).unwrap();
    Config::new()
        .service_generator(Box::new(super::ProtobufServiceGenerator::all()))
        .compile_fds(file_descriptors)
        .unwrap();
}

pub fn compile_clients(
    files: impl IntoIterator<Item = impl AsRef<Path>>,
    includes: impl IntoIterator<Item = impl AsRef<Path>>,
) {
    let file_descriptors = protox::compile(files, includes).unwrap();
    Config::new()
        .service_generator(Box::new(super::ProtobufServiceGenerator::client()))
        .compile_fds(file_descriptors)
        .unwrap();
}

pub fn compile_servers(
    files: impl IntoIterator<Item = impl AsRef<Path>>,
    includes: impl IntoIterator<Item = impl AsRef<Path>>,
) {
    let file_descriptors = protox::compile(files, includes).unwrap();
    Config::new()
        .service_generator(Box::new(super::ProtobufServiceGenerator::server()))
        .compile_fds(file_descriptors)
        .unwrap();
}
