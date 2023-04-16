use prost_build::Service;
use prost_types::FileDescriptorSet;
use proc_macro2::TokenStream;

/// Higher-level abstraction of prost_build::ServiceGenerator
pub trait IServiceGenerator {
    fn generate(&mut self, service: Service) -> TokenStream;
}

/// Higher-level abstraction of crate::Preprocessor
pub trait IPreprocessor {
    fn process(&mut self, fds: &mut FileDescriptorSet) -> TokenStream;
}

/// Low-level interop for high-level traits IServiceGenerator and IPreprocessor
pub struct AbstractImpl<X>(X);

impl<X> AbstractImpl<X> {
    pub fn inner(self) -> X {
        self.0
    }

    pub fn outer(value: X) -> Self {
        Self(value)
    }
}

impl<X> std::convert::From<X> for AbstractImpl<X> {
    fn from(value: X) -> Self {
        Self(value)
    }
}

impl<X: IServiceGenerator> prost_build::ServiceGenerator for AbstractImpl<X> {
    fn generate(&mut self, service: Service, buf: &mut String) {
        let gen_code: syn::File = syn::parse2(self.0.generate(service)).expect("invalid tokenstream");
        let code_str = prettyplease::unparse(&gen_code);
        buf.push_str(&code_str);
    }
}

impl<X: IPreprocessor> super::Preprocessor for AbstractImpl<X> {
    fn process(&mut self, fds: &mut FileDescriptorSet, buf: &mut String) {
        let gen_code: syn::File = syn::parse2(self.0.process(fds)).expect("invalid tokenstream");
        let code_str = prettyplease::unparse(&gen_code);
        buf.push_str(&code_str);
    }
}
