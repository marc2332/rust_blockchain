#![feature(async_closure)]

use std::sync::{
    Arc,
    Mutex,
};

use actix_web::{
    App,
    HttpServer,
};
use jsonrpc_http_server::{
    jsonrpc_core::*,
    *,
};

use blockchain::Configuration;

pub mod controllers;
pub mod methods;

use methods::get_chain_length;

static PORT: u16 = 8080;
static HOSTNAME: &str = "127.0.0.1";

#[actix_web::main]
pub async fn start_node(state: Arc<Mutex<Configuration>>) {
    let mut io = IoHandler::default();

    tokio::spawn(async move {
        io.add_method("get_chain_length", get_chain_length(state));

        let server = ServerBuilder::new(io)
            .cors(DomainsValidation::AllowOnly(vec![
                AccessControlAllowOrigin::Null,
            ]))
            .start_http(&"127.0.0.1:3030".parse().unwrap())
            .expect("Unable to start RPC server");

        server.wait();
    });

    HttpServer::new(|| App::new())
        .bind((HOSTNAME, PORT))
        .unwrap()
        .run()
        .await
        .unwrap();
}
