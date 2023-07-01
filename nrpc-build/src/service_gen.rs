use std::path::PathBuf;

use prost_build::{Service, ServiceGenerator};
use quote::quote;

pub(crate) struct ProtobufServiceGenerator {
    generate_server: bool,
    generate_client: bool,
    client_reexports: Vec<proc_macro2::TokenStream>,
    server_reexports: Vec<proc_macro2::TokenStream>,
    modules: Vec<String>,
    out_dir: PathBuf,
}

impl ProtobufServiceGenerator {
    pub fn all(out_dir: PathBuf) -> Self {
        Self {
            generate_server: true,
            generate_client: true,
            client_reexports: Vec::new(),
            server_reexports: Vec::new(),
            modules: Vec::new(),
            out_dir: out_dir,
        }
    }

    pub fn client(out_dir: PathBuf) -> Self {
        Self {
            generate_server: false,
            generate_client: true,
            client_reexports: Vec::new(),
            server_reexports: Vec::new(),
            modules: Vec::new(),
            out_dir: out_dir,
        }
    }

    pub fn server(out_dir: PathBuf) -> Self {
        Self {
            generate_server: true,
            generate_client: false,
            client_reexports: Vec::new(),
            server_reexports: Vec::new(),
            modules: Vec::new(),
            out_dir: out_dir,
        }
    }
}

fn stream_type(item_type: &syn::Ident) -> proc_macro2::TokenStream {
    quote::quote!{
        ::nrpc::ServiceStream<'a, #item_type>
    }
}

/*fn stream_type_static_lifetime(item_type: &syn::Ident) -> proc_macro2::TokenStream {
    quote::quote!{
        ::nrpc::ServiceStream<'static, #item_type>
    }
}*/

fn trait_methods_server(descriptors: &Vec<prost_build::Method>) -> proc_macro2::TokenStream {
    let mut gen_methods = Vec::with_capacity(descriptors.len());
    let mut gen_method_match_arms = Vec::with_capacity(descriptors.len());
    for descriptor in descriptors {
        let input_ty = quote::format_ident!("{}", descriptor.input_type);
        let output_ty = quote::format_ident!("{}", descriptor.output_type);
        let fn_name = quote::format_ident!("{}", descriptor.name);
        let method_name = &descriptor.name;
        match (descriptor.client_streaming, descriptor.server_streaming) {
            (false, false) => {
                // no streaming; 1->1
                gen_methods.push(
                    quote! {
                        async fn #fn_name(&mut self, input: #input_ty) -> Result<#output_ty, Box<dyn std::error::Error + Send>>;
                    }
                );

                gen_method_match_arms.push(quote! {
                    #method_name => {
                        if let Some(item1_payload) = stream_in.next().await {
                            let item = #input_ty::decode(item1_payload?)?;
                            // TODO does it need to be enforced that there are no more items in the stream?
                            let mut buffer = ::nrpc::_helpers::bytes::BytesMut::new();
                            self.#fn_name(item).await?.encode(&mut buffer)?;
                            Ok(Box::new(::nrpc::OnceStream::once(Ok(buffer.freeze()))))
                        } else {
                            Err(::nrpc::ServiceError::StreamLength { want: 1, got: 0 })
                        }
                    }
                });
            }
            (false, true) => {
                // client streaming; 1 -> many
                //let stream_out_ty = stream_type_static_lifetime(&output_ty);
                let stream_out_ty = stream_type(&output_ty);
                gen_methods.push(
                    quote! {
                        async fn #fn_name<'a>(&mut self, input: #input_ty) -> Result<#stream_out_ty, Box<dyn std::error::Error + Send>>;
                    }
                );

                gen_method_match_arms.push(quote! {
                    #method_name => {
                        if let Some(item1_payload) = stream_in.next().await {
                            let item = #input_ty::decode(item1_payload?)?;
                            // TODO does it need to be enforced that there are no more items in the stream?
                            let result = self.#fn_name(item).await?;
                            Ok(Box::new(
                                result.map(
                                    |item_result| item_result.and_then(|item| {
                                        let mut buffer = ::nrpc::_helpers::bytes::BytesMut::new();
                                        item.encode(&mut buffer)
                                            .map(|_| buffer.freeze())
                                            .map_err(|e| ::nrpc::ServiceError::from(e))
                                    })
                                )
                            ))
                        } else {
                            Err(::nrpc::ServiceError::StreamLength { want: 1, got: 0 })
                        }
                    }
                });
            }
            (true, false) => {
                // server streaming; many -> 1
                let stream_in_ty = stream_type(&input_ty);
                gen_methods.push(
                    quote! {
                        async fn #fn_name<'a>(&mut self, input: #stream_in_ty) -> Result<#output_ty, Box<dyn std::error::Error + Send>>;
                    }
                );

                gen_method_match_arms.push(quote! {
                    #method_name => {
                        let item_stream = stream_in.map(|item_result| item_result.and_then(|item1_payload| {
                            #input_ty::decode(item1_payload)
                                .map_err(|e| ::nrpc::ServiceError::from(e))
                        }));
                        let mut buffer = ::nrpc::_helpers::bytes::BytesMut::new();
                        self.#fn_name(Box::new(item_stream)).await?.encode(&mut buffer)?;
                        Ok(Box::new(::nrpc::OnceStream::once(Ok(buffer.freeze()))))
                    }
                });
            }
            (true, true) => {
                // all streaming; many -> many
                let stream_in_ty = stream_type(&input_ty);
                let stream_out_ty = stream_type(&output_ty);
                gen_methods.push(
                    quote! {
                        async fn #fn_name<'a>(&mut self, input: #stream_in_ty) -> Result<#stream_out_ty, Box<dyn std::error::Error + Send>>;
                    }
                );

                gen_method_match_arms.push(quote! {
                    #method_name => {
                        let item_stream = stream_in.map(|item_result| item_result.and_then(|item1_payload| {
                            #input_ty::decode(item1_payload)
                                .map_err(|e| ::nrpc::ServiceError::from(e))
                        }));
                        let result = self.#fn_name(Box::new(item_stream)).await?;
                        Ok(Box::new(
                            result.map(
                                |item_result| item_result.and_then(|item| {
                                    let mut buffer = ::nrpc::_helpers::bytes::BytesMut::new();
                                    item.encode(&mut buffer)
                                        .map(|_| buffer.freeze())
                                        .map_err(|e| ::nrpc::ServiceError::from(e))
                                })
                            )
                        ))
                    }
                });
            }
        }
    }

    quote! {
        #(#gen_methods)*

        /*async fn call(&mut self, method: &str, payload: ::nrpc::_helpers::bytes::Bytes, buffer: &mut ::nrpc::_helpers::bytes::BytesMut) -> Result<(), ::nrpc::ServiceError> {
            match method {
                #(#gen_method_match_arms)*
                _ => Err(::nrpc::ServiceError::MethodNotFound)
            }
        }*/

        async fn call<'a>(
            &mut self,
            method: &str,
            mut stream_in: ::nrpc::ServiceStream<'a, ::nrpc::_helpers::bytes::Bytes>,
        ) -> Result<::nrpc::ServiceStream<'a, ::nrpc::_helpers::bytes::Bytes>, ::nrpc::ServiceError> {
            match method {
                #(#gen_method_match_arms)*
                _ => Err(::nrpc::ServiceError::MethodNotFound)
            }
        }
    }
}

fn struct_methods_client(
    package_name: &str,
    service_name: &str,
    descriptors: &Vec<prost_build::Method>,
) -> proc_macro2::TokenStream {
    let mut gen_methods = Vec::with_capacity(descriptors.len());
    for descriptor in descriptors {
        let input_ty = quote::format_ident!("{}", descriptor.input_type);
        let output_ty = quote::format_ident!("{}", descriptor.output_type);
        let fn_name = quote::format_ident!("{}", descriptor.name);
        let method_name = &descriptor.name;
        match (descriptor.client_streaming, descriptor.server_streaming) {
            (false, false) => {
                // no streaming; 1->1
                gen_methods.push(
                    quote! {
                        pub async fn #fn_name(&self, input: #input_ty) -> Result<#output_ty, ::nrpc::ServiceError> {
                            let mut in_buf = ::nrpc::_helpers::bytes::BytesMut::new();
                            input.encode(&mut in_buf)?;
                            let in_stream = ::nrpc::OnceStream::once(Ok(in_buf.freeze()));
                            let mut result_stream = self.inner.call(#package_name, #service_name, #method_name, Box::new( in_stream)).await?;
                            if let Some(out_result) = result_stream.next().await {
                                Ok(#output_ty::decode(out_result?)?)
                            } else {
                                Err(::nrpc::ServiceError::StreamLength { want: 1, got: 0 })
                            }

                        }
                    }
                );
            }
            (false, true) => {
                // client streaming; 1 -> many
                let stream_out_ty = stream_type(&output_ty);
                gen_methods.push(
                    quote! {
                        pub async fn #fn_name<'a>(&self, input: #input_ty) -> Result<#stream_out_ty, ::nrpc::ServiceError> {
                            let mut in_buf = ::nrpc::_helpers::bytes::BytesMut::new();
                            input.encode(&mut in_buf)?;
                            let in_stream = ::nrpc::OnceStream::once(Ok(in_buf.freeze()));
                            let result_stream = self.inner.call(#package_name, #service_name, #method_name, Box::new(in_stream)).await?;
                            let item_stream = result_stream.map(|out_result|
                                out_result.and_then(|out_buf| #output_ty::decode(out_buf)
                                    .map_err(|e| ::nrpc::ServiceError::from(e))
                                )
                            );
                            Ok(Box::new(item_stream))
                        }
                    }
                );
            }
            (true, false) => {
                // server streaming; many -> 1
                let stream_in_ty = stream_type(&input_ty);
                gen_methods.push(
                    quote! {
                        pub async fn #fn_name<'a>(&self, input: #stream_in_ty) -> Result<#output_ty, ::nrpc::ServiceError> {
                            let in_stream = input.map(|item_result| {
                                let mut in_buf = ::nrpc::_helpers::bytes::BytesMut::new();
                                item_result.and_then(|item| item.encode(&mut in_buf)
                                    .map(|_| in_buf.freeze())
                                    .map_err(|e| ::nrpc::ServiceError::from(e))
                                )
                            });
                            let mut result_stream = self.inner.call(#package_name, #service_name, #method_name, Box::new(in_stream)).await?;
                            if let Some(out_result) = result_stream.next().await {
                                Ok(#output_ty::decode(out_result?)?)
                            } else {
                                Err(::nrpc::ServiceError::StreamLength { want: 1, got: 0 })
                            }

                        }
                    }
                );
            }
            (true, true) => {
                // all streaming; many -> many
                let stream_in_ty = stream_type(&input_ty);
                let stream_out_ty = stream_type(&output_ty);
                gen_methods.push(
                    quote! {
                        pub async fn #fn_name<'a>(&self, input: #stream_in_ty) -> Result<#stream_out_ty, ::nrpc::ServiceError> {
                            let in_stream = input.map(|item_result| {
                                let mut in_buf = ::nrpc::_helpers::bytes::BytesMut::new();
                                item_result.and_then(|item| item.encode(&mut in_buf)
                                    .map(|_| in_buf.freeze())
                                    .map_err(|e| ::nrpc::ServiceError::from(e))
                                )
                            });
                            let result_stream = self.inner.call(#package_name, #service_name, #method_name, Box::new(in_stream)).await?;
                            let item_stream = result_stream.map(|out_result|
                                out_result.and_then(|out_buf| #output_ty::decode(out_buf)
                                    .map_err(|e| ::nrpc::ServiceError::from(e))
                                )
                            );
                            Ok(Box::new(item_stream))

                        }
                    }
                );
            }
        }
    }

    quote! {
        #(#gen_methods)*
    }
}

fn generate_mod_rs(module_names: &Vec<String>, out_dir: &PathBuf) {
    // generate mod.rs
    let modules = module_names.iter().map(|m| {
        let mod_ident = quote::format_ident!("{}", m);
        quote! { pub mod #mod_ident; }
    });
    let gen_mods: syn::File = syn::parse2(quote! {
        #(#modules)*
    })
    .expect("invalid tokenstream");
    let mod_str = prettyplease::unparse(&gen_mods);
    std::fs::write(out_dir.join("mod.rs"), &mod_str).expect("Failed to write to $OUT_DIR/mod.rs");
    //std::fs::write("/home/ngnius/potato.rs", &mod_str).unwrap();
}

impl ServiceGenerator for ProtobufServiceGenerator {
    fn generate(&mut self, service: Service, buf: &mut String) {
        if self.generate_server {
            let service_mod_name =
                quote::format_ident!("{}_mod_server", service.name.to_lowercase());
            let service_trait_name = quote::format_ident!("{}Service", service.name);
            let service_trait_methods = trait_methods_server(&service.methods);
            let service_struct_name = quote::format_ident!("{}ServiceImpl", service.name);
            let descriptor_str = format!("{}.{}", service.package, service.name);
            let service_struct_rename = quote::format_ident!("{}Server", service.name);
            let service_trait_rename = quote::format_ident!("I{}", service.name);
            let gen_service = quote! {
                mod #service_mod_name {
                    use super::*;
                    use ::nrpc::_helpers::async_trait::async_trait;
                    use ::nrpc::_helpers::prost::Message;
                    use ::nrpc::_helpers::futures::StreamExt;

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

                        async fn call<'a>(
                            &mut self,
                            method: &str,
                            input: ::nrpc::ServiceStream<'a, ::nrpc::_helpers::bytes::Bytes>,
                        ) -> Result<::nrpc::ServiceStream<'a, ::nrpc::_helpers::bytes::Bytes>, ::nrpc::ServiceError> {
                            self.inner.call(method, input).await
                        }
                    }
                }
                pub use #service_mod_name::{
                    #service_struct_name as #service_struct_rename,
                    #service_trait_name as #service_trait_rename,
                };
            };
            self.server_reexports.push(quote! {
                pub use super::#service_mod_name::{#service_struct_name, #service_trait_name};
            });
            let gen_code: syn::File = syn::parse2(gen_service).expect("invalid tokenstream");
            let code_str = prettyplease::unparse(&gen_code);
            buf.push_str(&code_str);
        }
        if self.generate_client {
            let service_mod_name =
                quote::format_ident!("{}_mod_client", service.name.to_lowercase());
            let service_methods =
                struct_methods_client(&service.package, &service.name, &service.methods);
            let service_struct_name = quote::format_ident!("{}Service", service.name);
            let descriptor_str = format!("{}.{}", service.package, service.name);
            let service_rename = quote::format_ident!("{}Client", service.name);
            let gen_client = quote! {
                mod #service_mod_name {
                    use super::*;
                    use ::nrpc::_helpers::prost::Message;
                    use ::nrpc::_helpers::futures::StreamExt;

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
                pub use #service_mod_name::#service_struct_name as #service_rename;
            };

            self.client_reexports.push(quote! {
                pub use super::#service_mod_name::#service_struct_name;
            });
            let gen_code: syn::File = syn::parse2(gen_client).expect("invalid tokenstream");
            let code_str = prettyplease::unparse(&gen_code);
            buf.push_str(&code_str);
        }
        if !self.modules.contains(&service.package) {
            self.modules.push(service.package.clone());
            generate_mod_rs(&self.modules, &self.out_dir);
        }
    }

    fn finalize(&mut self, buf: &mut String) {
        let mut client_tokens = quote! {};
        let mut server_tokens = quote! {};
        if self.generate_client {
            let exports = &self.client_reexports;
            client_tokens = quote! {
                pub mod client {
                    #(#exports)*
                }
            };
        }
        if self.generate_server {
            let exports = &self.server_reexports;
            server_tokens = quote! {
                pub mod server {
                    #(#exports)*
                }
            };
        }
        let gen_code = quote! {
            #client_tokens

            #server_tokens

            pub mod finally {}
        };
        let gen_code: syn::File = syn::parse2(gen_code).expect("invalid tokenstream");
        let code_str = prettyplease::unparse(&gen_code);
        buf.push_str(&code_str);

        self.modules.clear();
        self.client_reexports.clear();
        self.server_reexports.clear();
    }
}
