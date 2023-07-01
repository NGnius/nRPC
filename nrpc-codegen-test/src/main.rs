use std::error::Error;
use std::fmt::Write;

use nrpc::_helpers::futures::StreamExt;
use nrpc::{ServerService, ServiceError};
use prost::Message;

pub mod generated {
    include!(concat!(env!("OUT_DIR"), "/mod.rs"));
}

pub use generated::*;

#[tokio::main]
async fn main() {
    // NOTE: This doesn't test network functionality
    // it just checks generated code for correctness (compile-time)
    // and tests mock client & server traits implementations
    let req = helloworld::HelloRequest {
        name: "World".into(),
    };
    let resp = helloworld::HelloReply {
        message: "Hello World".into(),
    };
    let original_resp = resp.clone();
    // server
    let mut service_impl = helloworld::GreeterServer::new(GreeterService);

    // server one to one
    let mut input_buf = bytes::BytesMut::new();
    //let mut output_buf = bytes::BytesMut::new();
    req.clone().encode(&mut input_buf).unwrap();
    let stream_in = nrpc::OnceStream::once(Ok(input_buf.into()));
    let mut output_stream = service_impl
        .call("say_hello", Box::new(stream_in))
        .await
        .unwrap();
    let output_buf = output_stream.next().await.unwrap().unwrap();
    let actual_resp = helloworld::HelloReply::decode(output_buf).unwrap();
    assert_eq!(resp, actual_resp);

    // client one to one
    let client_impl = helloworld::GreeterClient::new(ClientHandler);
    let resp = client_impl.say_hello(req.clone()).await.unwrap();
    assert_eq!(resp, actual_resp);

    // server many to one
    let resp = helloworld::HelloReply {
        message: "Hello World0, World1, World2".into(),
    };
    let stream_in = nrpc::VecStream::from_iter([(); 3].iter().enumerate().map(|(i, _)| {
        let mut input_buf = bytes::BytesMut::new();
        helloworld::HelloRequest { name: format!("World{}", i) }.encode(&mut input_buf).expect("Protobuf encoding error");
        Ok(input_buf.freeze())
    }));
    let mut output_stream = service_impl
        .call("say_hello_many_to_one", Box::new(stream_in))
        .await
        .unwrap();
    let output_buf = output_stream.next().await.unwrap().unwrap();
    let actual_resp = helloworld::HelloReply::decode(output_buf).unwrap();
    assert_eq!(resp, actual_resp);

    // client many to one
    let client_impl = helloworld::GreeterClient::new(ClientHandler);
    let stream_in = nrpc::VecStream::from_iter([(); 3].iter().enumerate().map(|(i, _)|
        Ok(helloworld::HelloRequest { name: format!("World{}", i) })));
    let resp = client_impl.say_hello_many_to_one(Box::new(stream_in)).await.unwrap();
    assert_eq!(resp, original_resp);

    // server one to many
    let resp = vec![
        helloworld::HelloReply {
            message: "Hello World".into(),
        },
        helloworld::HelloReply {
            message: "Hello World".into(),
        },
        helloworld::HelloReply {
            message: "Hello World".into(),
        },
    ];
    let mut input_buf = bytes::BytesMut::new();
    //let mut output_buf = bytes::BytesMut::new();
    req.clone().encode(&mut input_buf).unwrap();
    let stream_in = nrpc::OnceStream::once(Ok(input_buf.into()));
    let output_stream = service_impl
        .call("say_hello_one_to_many", Box::new(stream_in))
        .await
        .unwrap();
    let actual_resp: Vec<_> = output_stream.map(|buf_result| helloworld::HelloReply::decode(buf_result.unwrap()).unwrap()).collect().await;
    assert_eq!(resp, actual_resp);

    // client one to many
    let client_impl = helloworld::GreeterClient::new(ClientHandler);
    let resp: Vec<_> = client_impl.say_hello_one_to_many(req.clone()).await.unwrap().map(|item_result| item_result.unwrap()).collect().await;
    assert_eq!(resp, vec![original_resp.clone()]);

    // server many to many
    let resp = vec![
        helloworld::HelloReply {
            message: "Hello World0".into(),
        },
        helloworld::HelloReply {
            message: "Hello World1".into(),
        },
        helloworld::HelloReply {
            message: "Hello World2".into(),
        },
    ];
    let stream_in = nrpc::VecStream::from_iter([(); 3].iter().enumerate().map(|(i, _)| {
        let mut input_buf = bytes::BytesMut::new();
        helloworld::HelloRequest { name: format!("World{}", i) }.encode(&mut input_buf).expect("Protobuf encoding error");
        Ok(input_buf.freeze())
    }));
    let output_stream = service_impl
        .call("say_hello_many_to_many", Box::new(stream_in))
        .await
        .unwrap();
    let actual_resp: Vec<_> = output_stream.map(|buf_result| helloworld::HelloReply::decode(buf_result.unwrap()).unwrap()).collect().await;
    assert_eq!(resp, actual_resp);

    // client many to many
    let client_impl = helloworld::GreeterClient::new(ClientHandler);
    let stream_in = nrpc::VecStream::from_iter([(); 3].iter().enumerate().map(|(i, _)|
        Ok(helloworld::HelloRequest { name: format!("World{}", i) })));
    let resp: Vec<_> = client_impl.say_hello_many_to_many(Box::new(stream_in)).await.unwrap().map(|item_result| item_result.unwrap()).collect().await;
    assert_eq!(resp, vec![original_resp.clone(); 3]);
}

struct GreeterService;

#[async_trait::async_trait]
impl helloworld::IGreeter for GreeterService {
    async fn say_hello(
        &mut self,
        input: helloworld::HelloRequest,
    ) -> Result<helloworld::HelloReply, Box<dyn Error + Send>> {
        let result = helloworld::HelloReply {
            message: format!("Hello {}", input.name),
        };
        println!("{}", result.message);
        Ok(result)
    }

    async fn say_hello_one_to_many<'a>(
        &mut self,
        input: helloworld::HelloRequest,
    ) -> Result<
        ::nrpc::ServiceStream<'a, helloworld::HelloReply>,
        Box<dyn std::error::Error + Send>,
    > {
        let result = helloworld::HelloReply {
            message: format!("Hello {}", input.name),
        };
        println!("{}", result.message);
        Ok(Box::new(::nrpc::VecStream::from_iter([(); 3].iter().map(move |_| Ok(result.clone())))))
    }

    async fn say_hello_many_to_one<'a>(
        &mut self,
        mut input: ::nrpc::ServiceStream<'a, helloworld::HelloRequest>,
    ) -> Result<helloworld::HelloReply, Box<dyn Error + Send>>{
        let mut message = "Hello ".to_string();
        while let Some(item_result) = input.next().await {
            write!(message, "{}, ", item_result.map_err(|e| Box::new(e) as Box<dyn Error + Send>)?.name)
                .map_err(|e| Box::new(e) as Box<dyn Error + Send>)?;
        }
        let result = helloworld::HelloReply { message: message.trim_end_matches(", ").to_string(), };
        println!("{}", result.message);
        Ok(result)
    }

    async fn say_hello_many_to_many<'a>(
        &mut self,
        input: ::nrpc::ServiceStream<'a, helloworld::HelloRequest>,
    ) -> Result<
        ::nrpc::ServiceStream<'a, helloworld::HelloReply>,
        Box<dyn std::error::Error + Send>,
    >{
        Ok(Box::new(input.map(|item_result| item_result.map(|input| {
            let result = helloworld::HelloReply {
                message: format!("Hello {}", input.name),
            };
            println!("(many to many) {}", result.message);
            result
        }))))
    }
}

struct ClientHandler;

#[async_trait::async_trait]
impl nrpc::ClientHandler for ClientHandler {
    /*async fn call(
        &mut self,
        package: &str,
        service: &str,
        method: &str,
        input: bytes::Bytes,
        output: &mut bytes::BytesMut,
    ) -> Result<(), nrpc::ServiceError> {
        println!(
            "call {}.{}/{} with data {:?}",
            package, service, method, input
        );
        // This is ok to hardcode ONLY because it's for testing
        Ok(helloworld::HelloReply {
            message: "Hello World".into(),
        }
        .encode(output)?)
    }*/

    async fn call<'a>(
        &self,
        package: &str,
        service: &str,
        method: &str,
        input: ::nrpc::ServiceStream<'a, ::nrpc::_helpers::bytes::Bytes>,
    ) -> Result<::nrpc::ServiceStream<'a, ::nrpc::_helpers::bytes::Bytes>, ServiceError> {
        println!(
            "call {}.{}/{} with data stream",
            package, service, method
        );
        // This is ok to hardcode ONLY because it's for testing
        Ok(
            Box::new(input.map(|item_result| {
                    let mut output = bytes::BytesMut::new();
                    item_result.and_then(|_item| helloworld::HelloReply {
                        message: format!("Hello World"),
                    }.encode(&mut output).map(|_| output.freeze()).map_err(|e| ServiceError::from(e)))
                }
            ))
        )
    }
}
