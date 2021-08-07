use actix_web::http;

use node::controllers::status::status;

#[actix_rt::test]
async fn test_status() {
    let resp = status().await;

    assert_eq!(resp.status(), http::StatusCode::OK);
}
