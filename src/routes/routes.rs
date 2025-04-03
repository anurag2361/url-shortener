use actix_web::web;

use crate::handlers::health_handlers::health_check;
use crate::handlers::qr_handlers::{generate_direct_qr, regenerate_qr};
use crate::handlers::url_handlers::{
    create_short_url, get_all_urls, get_qr_code_direct, get_url_analytics, redirect_to_url,
};

/// Configure the routes
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    // Define redirect route at the root level
    cfg.route("/r/{code}", web::get().to(redirect_to_url));
    cfg.service(
        web::scope("/api")
            .route("/shorten", web::post().to(create_short_url))
            .route("/urls", web::get().to(get_all_urls))
            .route("/health/check", web::get().to(health_check))
            .route("/qr/{code}/regenerate", web::get().to(regenerate_qr))
            .route("/qr/{code}/info", web::get().to(get_qr_code_direct))
            .route("/analytics/{code}", web::get().to(get_url_analytics))
            .route("/qr", web::post().to(generate_direct_qr)),
    );
}
