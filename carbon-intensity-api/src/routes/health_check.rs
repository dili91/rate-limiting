use actix_web::{HttpResponse, Responder};

pub async fn health_check() -> std::io::Result<impl Responder> {
    Ok(HttpResponse::Ok().body("I'm up and running."))
}
