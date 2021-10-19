use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tonic::transport::Server;
use tonic::{Request, Response, Status};
use tonic_health::server::HealthReporter;

use crate::proto::proto::kv_server::{Kv, KvServer};
use crate::proto::proto::{Empty, GetRequest, GetResponse, PutRequest};

mod proto;

// Implement a HealthReporter handler for tonic.
async fn driver_service_status(mut reporter: HealthReporter) {
    reporter.set_serving::<KvServer<PluginServer>>().await;
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // go-plugin requires this to be written to satisfy the handshake protocol.
    // https://github.com/hashicorp/go-plugin/blob/master/docs/guide-plugin-write-non-go.md#4-output-handshake-information
    println!("1|2|tcp|127.0.0.1:5001|grpc");

    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    health_reporter
        .set_serving::<KvServer<PluginServer>>()
        .await;

    tokio::spawn(driver_service_status(health_reporter.clone()));

    let addr = "0.0.0.0:5001".parse().unwrap();
    let plugin_server = PluginServer::default();

    Server::builder()
        .add_service(health_service)
        .add_service(KvServer::new(plugin_server))
        .serve(addr)
        .await?;

    Ok(())
}

pub struct PluginServer {
    store: Arc<Mutex<HashMap<String, Vec<u8>>>>,
}

impl core::default::Default for PluginServer {
    fn default() -> Self {
        PluginServer {
            store: Arc::new(Mutex::new(PluginServer::default_store())),
        }
    }
}

impl PluginServer {
    fn default_store() -> HashMap<String, Vec<u8>> {
        let store: HashMap<String, Vec<u8>> = HashMap::new();
        store
    }
}

#[tonic::async_trait]
impl Kv for PluginServer {
    async fn get(&self, request: Request<GetRequest>) -> Result<Response<GetResponse>, Status> {
        let key = request.get_ref().clone().key;
        if key.is_empty() {
            return Err(Status::invalid_argument("key not specified"));
        }

        let store_clone = Arc::clone(&self.store);
        let store = store_clone.lock().unwrap();

        match store.get(&key) {
            Some(value) => Ok(tonic::Response::new(GetResponse {
                value: value.clone(),
            })),
            None => Err(Status::invalid_argument("key not found")),
        }
    }

    async fn put(&self, request: Request<PutRequest>) -> Result<Response<Empty>, Status> {
        let request_ref = request.get_ref().clone();
        if request_ref.key.is_empty() {
            return Err(Status::invalid_argument("key not specified"));
        }

        let store_clone = Arc::clone(&self.store);
        let mut store = store_clone.lock().unwrap();

        store.insert(request_ref.key, request_ref.value);

        Ok(Response::new(Empty {}))
    }
}
