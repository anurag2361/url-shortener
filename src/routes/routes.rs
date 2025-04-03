use actix_web::web;

use crate::handlers::auth_handlers::{create_superuser, login};
use crate::handlers::health_handlers::health_check;
use crate::handlers::qr_handlers::{generate_direct_qr, regenerate_qr};
use crate::handlers::url_handlers::{
    create_short_url, get_all_urls, get_qr_code_direct, get_url_analytics, redirect_to_url,
};
use crate::middlewares::authmw::{JwtAuth, RequireRoles};
use crate::models::role::Role;

/// Configure the routes
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    // Define redirect route at the root level
    cfg.route("/r/{code}", web::get().to(redirect_to_url));
    // Authentication routes - no auth required
    cfg.service(
        web::scope("/api/auth")
            .route("/login", web::post().to(login))
            .route("/init", web::post().to(create_superuser)),
    );
    // API routes - require authentication
    cfg.service(
        web::scope("/api")
            .wrap(JwtAuth)
            .route("/shorten", web::post().to(create_short_url))
            .service(
                web::resource("/urls")
                    .wrap(RequireRoles(vec![Role::UrlViewer, Role::UrlManager]))
                    .route(web::get().to(get_all_urls)),
            )
            .route("/health/check", web::get().to(health_check))
            .service(
                web::resource("/qr/{code}/regenerate")
                    .wrap(RequireRoles(vec![Role::QrManager]))
                    .route(web::get().to(regenerate_qr)),
            )
            .service(
                web::resource("/qr/{code}/info")
                    .wrap(RequireRoles(vec![Role::QrViewer, Role::QrManager]))
                    .route(web::get().to(get_qr_code_direct)),
            )
            .service(
                web::resource("/analytics/{code}")
                    .wrap(RequireRoles(vec![
                        Role::AnalyticsViewer,
                        Role::AnalyticsManager,
                    ]))
                    .route(web::get().to(get_url_analytics)),
            )
            .service(
                web::resource("/qr")
                    .wrap(RequireRoles(vec![Role::QrCreator, Role::QrManager]))
                    .route(web::post().to(generate_direct_qr)),
            ),
    );
}
