use actix_web::{
    web,
    App,
    HttpServer,
};

pub mod controllers;

use controllers::status::status;

static PORT: u16 = 8080;
static HOSTNAME: &str = "127.0.0.1";

#[actix_web::main]
pub async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/status", web::get().to(status)))
        .bind((HOSTNAME, PORT))?
        .run()
        .await
}
