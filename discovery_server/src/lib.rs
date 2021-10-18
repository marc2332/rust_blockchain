use actix_web::{
    web,
    App,
    HttpRequest,
    HttpServer,
    Responder,
};
use blockchain::{
    Key,
    PublicAddress,
    SignVerifier,
};
use serde::{
    Deserialize,
    Serialize,
};
use std::{
    collections::HashMap,
    sync::{
        Arc,
        Mutex,
    },
};

#[derive(Serialize, Deserialize, Clone)]
struct SignalRequest {
    address: String,
    rpc_port: u16,
    rpc_ws_port: u16,
    key: Key,
    sign: Key,
}

/*
 * Register requester's public address, IP and port into a not-permament hashmap
 */
async fn signal(
    data: web::Json<SignalRequest>,
    req: HttpRequest,
    state: web::Data<Arc<Mutex<State>>>,
) -> impl Responder {
    let public_address = PublicAddress::from(&data.key);
    if public_address.verify_signature(&data.sign, data.address.clone()) {
        let ip = req.peer_addr().unwrap().ip().to_string();
        let response = serde_json::to_string(&state.lock().unwrap().signalers).unwrap();
        state
            .lock()
            .unwrap()
            .signalers
            .insert(data.address.clone(), (ip, data.rpc_port, data.rpc_ws_port));
        response
    } else {
        "failed".to_string()
    }
}

#[derive(Default)]
struct State {
    signalers: HashMap<String, (String, u16, u16)>,
}

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    let state = Arc::new(Mutex::new(State::default()));

    HttpServer::new(move || {
        let state = web::Data::new(state.clone());
        App::new()
            .app_data(state)
            .route("/signal", web::post().to(signal))
    })
    .bind("0.0.0.0:33140")?
    .run()
    .await
}
