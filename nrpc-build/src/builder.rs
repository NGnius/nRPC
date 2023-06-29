use std::convert::AsRef;
use std::iter::IntoIterator;
use std::path::Path;

use prost_build::Config;
use prost_build::{Service, ServiceGenerator};
use prost_types::FileDescriptorSet;

use super::Preprocessor;

/// Proto -> Rust transpiler configurator
pub struct Transpiler<'a> {
    prost_config: Config,
    files: FileDescriptorSet,
    service_generator: MergedServiceGenerator,
    preprocessors: Vec<Box<dyn Preprocessor + 'a>>,
}

impl<'a> Transpiler<'a> {
    pub fn new(
        files: impl IntoIterator<Item = impl AsRef<Path>>,
        includes: impl IntoIterator<Item = impl AsRef<Path>>,
    ) -> Result<Self, impl std::error::Error> {
        let files: Vec<_> = files.into_iter().collect();
        for f in &files {
            println!("cargo:rerun-if-changed={}", f.as_ref().display());
        }
        Ok::<_, protox::Error>(Self {
            prost_config: Config::new(),
            files: protox::compile(files, includes)?,
            service_generator: MergedServiceGenerator::empty(),
            preprocessors: Vec::new(),
        })
    }

    /// Generate client and server service implementations
    pub fn generate_all(mut self) -> Self {
        self.service_generator
            .add_service(super::ProtobufServiceGenerator::all(
                std::env::var("OUT_DIR").unwrap().into(),
            ));
        self
    }

    /// Generate server services implementations
    pub fn generate_server(mut self) -> Self {
        self.service_generator
            .add_service(super::ProtobufServiceGenerator::server(
                std::env::var("OUT_DIR").unwrap().into(),
            ));
        self
    }

    /// Generate client services implementations
    pub fn generate_client(mut self) -> Self {
        self.service_generator
            .add_service(super::ProtobufServiceGenerator::client(
                std::env::var("OUT_DIR").unwrap().into(),
            ));
        self
    }

    /// Add additional custom service generator
    pub fn with_service_generator<S: ServiceGenerator + 'static>(mut self, gen: S) -> Self {
        self.service_generator.add_service(gen);
        self
    }

    /// Add a proto file descriptor preprocessor
    pub fn with_preprocessor<P: Preprocessor + 'a>(mut self, pp: P) -> Self {
        self.preprocessors.push(Box::new(pp));
        self
    }

    /// Actually generate code
    pub fn transpile(mut self) -> std::io::Result<()> {
        let mut files = self.files;
        let mut generated = String::new();
        for mut pp in self.preprocessors {
            pp.process(&mut files, &mut generated);
        }
        self.service_generator
            .add_service(PreprocessedCodeGenInjector {
                generated_str: generated,
            });

        self.prost_config
            .service_generator(Box::new(self.service_generator))
            .compile_fds(files)
    }
}

struct PreprocessedCodeGenInjector {
    generated_str: String,
}

impl ServiceGenerator for PreprocessedCodeGenInjector {
    fn generate(&mut self, _: Service, buf: &mut String) {
        buf.insert_str(0, &self.generated_str);
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
    includes: impl IntoIterator<Item = impl AsRef<Path>>,
) {
    Transpiler::new(files, includes)
        .unwrap()
        .generate_all()
        .transpile()
        .unwrap();
}

/// Compile proto files into Rust with only client implementations
pub fn compile_clients(
    files: impl IntoIterator<Item = impl AsRef<Path>>,
    includes: impl IntoIterator<Item = impl AsRef<Path>>,
) {
    Transpiler::new(files, includes)
        .unwrap()
        .generate_client()
        .transpile()
        .unwrap();
}

/// Compile proto files into Rust with only server implementations
pub fn compile_servers(
    files: impl IntoIterator<Item = impl AsRef<Path>>,
    includes: impl IntoIterator<Item = impl AsRef<Path>>,
) {
    Transpiler::new(files, includes)
        .unwrap()
        .generate_server()
        .transpile()
        .unwrap();
}
