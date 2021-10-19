# Overview

This template contains a reference implementation of how to implement a rust binary
that can be consumed by HashiCorp's [go-plugin](https://github.com/hashicorp/go-plugin).

## Current limitations

This project template was extracted from the [nomad-driver-wasm](https://github.com/DerekStrickland/nomad-driver-wasm)
project and refactored to conform with the KV example used in the `go-plugin`
repository. It currently, only implements the server side of the plugin, since
[Nomad](https://github.com/hashicorp/nomad) itself runs the client process in the
project this derived from. Work is currently under way to implement a client process
written in Rust, and when finished, will be submitted for consideration as an official
example in the `go-plugin` repository.

## Usage

- Review the references implementation contained in `main.rs` to get an understanding
  of what you will be doing in your own repository. Ultimately, you will replace
  this code, but it is there to help you get your bearings.
- Create a new repository from this template.
- Locate the `.proto` files you intend to use
- Optionally, copy the `.proto` files to the `proto` directory
- Remove the `kv.proto` file 
- Review and modify the `build.rs` file

```rust
fn main() -> Result<()> {
  tonic_build::configure()
  .build_server(true)
  .out_dir("src/proto")
  .compile_well_known_types(true)
  .include_file("mod.rs")
  .type_attribute(".", "#[derive(serde::Deserialize)]")
  .type_attribute(".", "#[derive(serde::Serialize)]")
  .compile(&["proto/kv.proto"], &["proto"])
  .unwrap();

  Ok(())
}
```

### Build Settings of interest

- `out_dir` - This is the directory where `tonic_build` and `prost` will output
generated code. You can rename this directory if you like. Be mindful that if you
do, you will need to update all example Rust code to reflect the new module structure.
- `include_file` - This instructs `prost` to sort out include paths for any `.proto`
files you add that reference each other. This is highly recommended, though you 
can do this manually.
- `compile_well_known_types` - tells `prost` to generate output for well-known
google protobuf types. This is optional, but I've found it helpful.
- `type_attribute` - This adds the specified derive directive to structs generated 
by `prost`.
- `.compile` - Takes two arguments. 
  - The first is the set of `.proto` files to generate code for. If you do not want
  to replicate your files to the `proto` folder, you can delete it, and use relative
  paths here.
  - The second is the set of directories to use as search roots when trying
  to resolve `.proto` dependencies referenced within the targeted `.proto` files. These
  can also be relative paths.
  
## Generating code

 - Update the `Cargo.toml` file to match your desired package/bin names
 - Run `cargo build --bin <your-package-name>`
 - This will fail, because the reference implementation is still in place.
 - Once it fails, look in the `out_dir` and you should see two or more files
   - `mod.rs` - This is the module file that contains all necessary includes. You
   should reference this file to understand the namespace hierarch of your generated
   code.
   - `*.proto.rs` - These file will contain your generated code. You should have
   one per `proto.rs` file you included as a `compile` argument. You may also
   have a `google.protobuf.rs` file if you kept the `compile_well_known_types`
   settings from the example.
   
     
## Implementing Services

Review the generated `proto/*.proto.rs` files you generated, and look for any
structs named `<TypeName>Server`. For example, the reference implementation
includes the following code.

```rust
pub trait Kv: Send + Sync + 'static {
    async fn get(
        &self,
        request: tonic::Request<super::GetRequest>,
    ) -> Result<tonic::Response<super::GetResponse>, tonic::Status>;
    async fn put(
        &self,
        request: tonic::Request<super::PutRequest>,
    ) -> Result<tonic::Response<super::Empty>, tonic::Status>;
}
#[derive(Debug)]
pub struct KvServer<T: Kv> {
    inner: _Inner<T>,
    accept_compression_encodings: (),
    send_compression_encodings: (),
}
```

Each service you have defined in your `.proto` files should have a corresponding
server entry like this. Notice also, that there is a trait associated with the 
`KvServer` struct. Tonic will generate a great deal of the GRPC plumbing code 
for you, but you will need to implement the trait functions that contain your
service logic.

Here is the `Kv` trait implementation from the current `main.rs` for example.

```rust
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
            Some(value) => Ok(Response::new(GetResponse {
                value: value.clone().to_vec(),
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
```

## Testing

To test the reference implementation, open two terminals at the root of this directory.

In one run:

```shell
RUST_LOG=debug cargo run --bin go-plugin-rs
   Compiling go-plugin-rs v0.1.0 (/Users/derekstrickland/goland/go-plugin-rs)
    Finished dev [unoptimized + debuginfo] target(s) in 4.61s
     Running `target/debug/go-plugin-rs`
1|2|tcp|127.0.0.1:5001|grpc
```

Notice the `1|2|tcp|...` line. `go-plugin` requires this to be written to satisfy
its [handshake protocol](https://github.com/hashicorp/go-plugin/blob/master/docs/guide-plugin-write-non-go.md#4-output-handshake-information).


In the other run:

```shell
$ RUST_LOG=debug cargo run --bin test-client
   Compiling go-plugin-rs v0.1.0 (/Users/derekstrickland/goland/go-plugin-rs)
    Finished dev [unoptimized + debuginfo] target(s) in 3.57s
     Running `target/debug/test-client`
[2021-10-19T11:03:00Z DEBUG hyper::client::connect::http] connecting to 0.0.0.0:5001 
......
[2021-10-19T11:03:00Z INFO  test_client] get response: bar
```


## Further examples

Review the rest of the `main.rs` file, as well as the [`tonic examples`](https://github.com/hyperium/tonic/tree/master/examples/src)
to see how to configure a basic server.

When you are ready, start implementing your own services!
The example in this repository is very simple. For additional examples, including
how to work with streaming endpoints, see the [nomad-driver-wasm](https://github.com/DerekStrickland/nomad-driver-wasm)
repository.

