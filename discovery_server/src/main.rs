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
    key: Key,
    sign: Key,
}

async fn signal(
    data: web::Json<SignalRequest>,
    req: HttpRequest,
    state: web::Data<Arc<Mutex<State>>>,
) -> impl Responder {
    let public_address = PublicAddress::from(&data.key);
    if public_address.verify_signature(&data.sign, data.address.clone()) {
        let req_info = req.connection_info();
        let ip = req_info.host();
        state
            .lock()
            .unwrap()
            .signalers
            .insert(data.address.clone(), ip.to_string());
        serde_json::to_string(&state.lock().unwrap().signalers).unwrap()
    } else {
        "failed".to_string()
    }
}

#[derive(Default)]
struct State {
    signalers: HashMap<String, String>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
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
