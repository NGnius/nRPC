use std::error::Error;

use prost::Message;
use nrpc::ServerService;

pub mod helloworld {
    include!(concat!(env!("OUT_DIR"), "/helloworld.rs"));
}

#[tokio::main]
async fn main() {
    let req = helloworld::HelloRequest {
        name: "World".into(),
    };
    let resp = helloworld::HelloReply {
        message: "Hello World".into(),
    };
    // server
    let mut service_impl = helloworld::server::GreeterServiceImpl::new(GreeterService);
    let mut input_buf = bytes::BytesMut::new();
    let mut output_buf = bytes::BytesMut::new();
    req.encode(&mut input_buf).unwrap();
    service_impl.call("say_hello", input_buf.into(), &mut output_buf).await.unwrap();
    let actual_resp = helloworld::HelloReply::decode(output_buf).unwrap();
    assert_eq!(resp, actual_resp);

    // client
    let mut client_impl = helloworld::client::GreeterService::new(ClientHandler);
    let resp = client_impl.say_hello(req).await.unwrap();
    assert_eq!(resp, actual_resp);
}

struct GreeterService;

#[async_trait::async_trait]
impl helloworld::server::GreeterService for GreeterService {
    async fn say_hello(&mut self, input: helloworld::HelloRequest) -> Result<helloworld::HelloReply, Box<dyn Error>> {
        let result = helloworld::HelloReply {
            message: format!("Hello {}", input.name),
        };
        println!("{}", result.message);
        Ok(result)
    }
}

struct ClientHandler;

#[async_trait::async_trait]
impl nrpc::ClientHandler for ClientHandler {
    async fn call(&mut self,
            service: &str,
            method: &str,
            input: bytes::Bytes,
            output: &mut bytes::BytesMut) -> Result<(), nrpc::ServiceError> {
                println!("call {}/{} with data {:?}", service, method, input);
                // This is ok to hardcode ONLY because it's for testing
                Ok(helloworld::HelloReply {
                    message: "Hello World".into(),
                }.encode(output)?)
            }
}
