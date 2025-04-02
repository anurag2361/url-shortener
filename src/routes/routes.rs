use actix_web::web;

use crate::handlers::health_handlers::health_check;
use crate::handlers::qr_handlers::generate_qr;
use crate::handlers::url_handlers::{create_short_url, redirect_to_url};

/// Configure the routes
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/shorten", web::post().to(create_short_url))
            .route("/health", web::get().to(health_check)),
    )
    .service(
        web::scope("")
            .route("/r/{code}", web::get().to(redirect_to_url))
            .route("/qr/{code}", web::get().to(generate_qr)),
    );
}
