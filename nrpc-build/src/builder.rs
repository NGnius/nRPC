use std::path::Path;
use std::convert::AsRef;
use std::iter::IntoIterator;

use prost_build::Config;
use prost_build::{Service, ServiceGenerator};
use prost_types::FileDescriptorSet;

/// Proto -> Rust transpiler configurator
pub struct Transpiler {
    prost_config: Config,
    files: FileDescriptorSet,
    service_generator: MergedServiceGenerator,
}

impl Transpiler {
    pub fn new(
        files: impl IntoIterator<Item = impl AsRef<Path>>,
        includes: impl IntoIterator<Item = impl AsRef<Path>>
    ) -> Result<Self, impl std::error::Error> {
        Ok::<_, protox::Error>(Self {
            prost_config: Config::new(),
            files: protox::compile(files, includes)?,
            service_generator: MergedServiceGenerator::empty()
        })
    }

    /// Generate client and server service implementations
    pub fn generate_all(mut self) -> Self {
        self.service_generator.add_service(super::ProtobufServiceGenerator::all());
        self
    }

    /// Generate server services implementations
    pub fn generate_server(mut self) -> Self {
        self.service_generator.add_service(super::ProtobufServiceGenerator::server());
        self
    }

    /// Generate client services implementations
    pub fn generate_client(mut self) -> Self {
        self.service_generator.add_service(super::ProtobufServiceGenerator::client());
        self
    }

    /// Add additional custom service generator
    pub fn with_service_generator<S: ServiceGenerator + 'static>(mut self, gen: S) -> Self {
        self.service_generator.add_service(gen);
        self
    }

    /// Actually generate code
    pub fn transpile(mut self) -> std::io::Result<()> {
        self.prost_config
            .service_generator(Box::new(self.service_generator))
            .compile_fds(self.files)
    }
}

struct MergedServiceGenerator {
    generators: Vec<Box<dyn ServiceGenerator + 'static>>,
}

impl MergedServiceGenerator {
    fn empty() -> Self {
        Self {
            generators: Vec::new(),
        }
    }

    fn add_service<S: ServiceGenerator + 'static>(&mut self, service: S) -> &mut Self {
        self.generators.push(Box::new(service));
        self
    }
}

impl ServiceGenerator for MergedServiceGenerator {
    fn generate(&mut self, service: Service, buf: &mut String) {
        for gen in &mut self.generators {
            gen.generate(service.clone(), buf);
        }
    }
}

/// Compile proto files into Rust with server and client implementations
pub fn compile(
    files: impl IntoIterator<Item = impl AsRef<Path>>,
    includes: impl IntoIterator<Item = impl AsRef<Path>>
) {
    Transpiler::new(files, includes).unwrap()
        .generate_all()
        .transpile()
        .unwrap();
}

/// Compile proto files into Rust with only client implementations
pub fn compile_clients(
    files: impl IntoIterator<Item = impl AsRef<Path>>,
    includes: impl IntoIterator<Item = impl AsRef<Path>>,
) {
    Transpiler::new(files, includes).unwrap()
        .generate_client()
        .transpile()
        .unwrap();
}

/// Compile proto files into Rust with only server implementations
pub fn compile_servers(
    files: impl IntoIterator<Item = impl AsRef<Path>>,
    includes: impl IntoIterator<Item = impl AsRef<Path>>,
) {
    Transpiler::new(files, includes).unwrap()
        .generate_server()
        .transpile()
        .unwrap();
}
