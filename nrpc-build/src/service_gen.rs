use prost_build::{Service, ServiceGenerator};
use quote::quote;

pub(crate) struct ProtobufServiceGenerator {
    generate_server: bool,
    generate_client: bool,
}

impl ProtobufServiceGenerator {
    pub fn all() -> Self {
        Self {
            generate_server: true,
            generate_client: true,
        }
    }

    pub fn client() -> Self {
        Self {
            generate_server: false,
            generate_client: true,
        }
    }

    pub fn server() -> Self {
        Self {
            generate_server: true,
            generate_client: false,
        }
    }
}

fn trait_methods_server(descriptors: &Vec<prost_build::Method>) -> proc_macro2::TokenStream {
    let mut gen_methods = Vec::with_capacity(descriptors.len());
    let mut gen_method_match_arms = Vec::with_capacity(descriptors.len());
    for descriptor in descriptors {
        match (descriptor.client_streaming, descriptor.server_streaming) {
            (false, false) => { // no streaming; 1->1
                let input_ty = quote::format_ident!("{}", descriptor.input_type);
                let output_ty = quote::format_ident!("{}", descriptor.output_type);
                let fn_name = quote::format_ident!("{}", descriptor.name);
                let method_name = &descriptor.name;
                gen_methods.push(
                    quote! {
                        async fn #fn_name(&mut self, input: #input_ty) -> Result<#output_ty, Box<dyn std::error::Error>>;
                    }
                );

                gen_method_match_arms.push(
                    quote! {
                        #method_name => {
                            Ok(self.#fn_name(#input_ty::decode(payload)?).await?.encode(buffer)?)
                        }
                    }
                );
            },
            (true, false) => { // client streaming; 1 -> many
                todo!("streaming not supported")
            },
            (false, true) => { // server streaming; many -> 1
                todo!("streaming not supported")
            }
            (true, true) => { // all streaming; many -> many
                todo!("streaming not supported")
            },
        }

    }

    quote! {
        #(#gen_methods)*

        async fn call(&mut self, method: &str, payload: ::nrpc::_helpers::bytes::Bytes, buffer: &mut ::nrpc::_helpers::bytes::BytesMut) -> Result<(), ::nrpc::ServiceError> {
            match method {
                #(#gen_method_match_arms)*
                _ => Err(::nrpc::ServiceError::MethodNotFound)
            }
        }
    }
}

fn struct_methods_client(service_name: &str, descriptors: &Vec<prost_build::Method>) -> proc_macro2::TokenStream {
    let mut gen_methods = Vec::with_capacity(descriptors.len());
    for descriptor in descriptors {
        match (descriptor.client_streaming, descriptor.server_streaming) {
            (false, false) => { // no streaming; 1->1
                let input_ty = quote::format_ident!("{}", descriptor.input_type);
                let output_ty = quote::format_ident!("{}", descriptor.output_type);
                let fn_name = quote::format_ident!("{}", descriptor.name);
                let method_name = &descriptor.name;
                gen_methods.push(
                    quote! {
                        pub async fn #fn_name(&mut self, input: #input_ty) -> Result<#output_ty, ::nrpc::ServiceError> {
                            let mut in_buf = ::nrpc::_helpers::bytes::BytesMut::new();
                            input.encode(&mut in_buf)?;
                            let mut out_buf = ::nrpc::_helpers::bytes::BytesMut::new();
                            self.inner.call(#service_name, #method_name, in_buf.into(), &mut out_buf).await?;
                            Ok(#output_ty::decode(out_buf)?)
                        }
                    }
                );
            },
            (true, false) => { // client streaming; 1 -> many
                todo!("streaming not supported")
            },
            (false, true) => { // server streaming; many -> 1
                todo!("streaming not supported")
            }
            (true, true) => { // all streaming; many -> many
                todo!("streaming not supported")
            },
        }

    }

    quote! {
        #(#gen_methods)*
    }
}

impl ServiceGenerator for ProtobufServiceGenerator {
    fn generate(&mut self, service: Service, buf: &mut String) {
        if self.generate_server {
            let service_mod_name = quote::format_ident!("{}_mod_server", service.name.to_lowercase());
            let service_trait_name = quote::format_ident!("{}Service", service.name);
            let service_trait_methods = trait_methods_server(&service.methods);
            let service_struct_name = quote::format_ident!("{}ServiceImpl", service.name);
            let descriptor_str = format!("{}.{}", service.package, service.name);
            let gen_service = quote! {
                mod #service_mod_name {
                    use super::*;
                    use ::nrpc::_helpers::async_trait::async_trait;
                    use ::nrpc::_helpers::prost::Message;

                    #[async_trait]
                    pub trait #service_trait_name: Send {
                        #service_trait_methods
                    }

                    pub struct #service_struct_name<T: #service_trait_name> {
                        inner: T,
                    }

                    impl <T: #service_trait_name> #service_struct_name<T> {
                        pub fn new(inner: T) -> Self {
                            Self {
                                inner,
                            }
                        }
                    }

                    #[async_trait]
                    impl<T: #service_trait_name> ::nrpc::ServerService for #service_struct_name<T> {
                        fn descriptor(&self) -> &'static str {
                            #descriptor_str
                        }

                        async fn call(&mut self, method: &str, payload: ::nrpc::_helpers::bytes::Bytes, buffer: &mut ::nrpc::_helpers::bytes::BytesMut) -> Result<(), ::nrpc::ServiceError> {
                            self.inner.call(method, payload, buffer).await
                        }
                    }
                }
                pub mod server {
                    pub use super::#service_mod_name::{#service_struct_name, #service_trait_name};
                }
            };
            let gen_code: syn::File = syn::parse2(gen_service).expect("invalid tokenstream");
            let code_str = prettyplease::unparse(&gen_code);
            buf.push_str(&code_str);
        }
        if self.generate_client {
            let service_mod_name = quote::format_ident!("{}_mod_client", service.name.to_lowercase());
            let service_methods = struct_methods_client(&service.name, &service.methods);
            let service_struct_name = quote::format_ident!("{}Service", service.name);
            let descriptor_str = format!("{}.{}", service.package, service.name);
            let gen_client = quote! {
                mod #service_mod_name {
                    use super::*;
                    use ::nrpc::_helpers::prost::Message;

                    //#[derive(core::any::Any)]
                    pub struct #service_struct_name<T: ::nrpc::ClientHandler> {
                        inner: T,
                    }

                    impl <T: ::nrpc::ClientHandler> ::nrpc::ClientService for #service_struct_name<T> {
                        fn descriptor(&self) -> &'static str {
                            #descriptor_str
                        }
                    }

                    impl <T: ::nrpc::ClientHandler> #service_struct_name<T> {
                        pub fn new(inner: T) -> Self {
                            Self {
                                inner,
                            }
                        }

                        #service_methods
                    }
                }
                pub mod client {
                    pub use super::#service_mod_name::#service_struct_name;
                }
            };
            let gen_code: syn::File = syn::parse2(gen_client).expect("invalid tokenstream");
            let code_str = prettyplease::unparse(&gen_code);
            buf.push_str(&code_str);
        }
    }
}
