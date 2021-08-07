use actix_web::{
    HttpRequest,
    HttpResponse,
};
use serde_derive::Serialize;

#[derive(Serialize)]
struct StatusResponse {
    OK: bool,
}

pub async fn status() -> HttpResponse {
    HttpResponse::Ok().json(StatusResponse { OK: true })
}
