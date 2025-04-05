use actix_web::web;

use crate::handlers::auth_handlers::{create_superuser, login, signup};
use crate::handlers::health_handlers::health_check;
use crate::handlers::qr_handlers::{generate_direct_qr, get_all_qr_codes, regenerate_qr};
use crate::handlers::url_handlers::{
    create_short_url, get_all_urls, get_qr_code_direct, get_url_analytics, redirect_to_url,
};
use crate::handlers::user_handlers::{
    create_user, delete_user, edit_user, get_all_users, get_user,
};
use crate::middlewares::authmw::JwtAuth;

/// Configure the routes
pub fn init_routes(cfg: &mut web::ServiceConfig) {
    // Define redirect route at the root level
    cfg.route("/r/{code}", web::get().to(redirect_to_url));
    // Authentication routes - no auth required
    cfg.service(
        web::scope("/api/auth")
            .route("/login", web::post().to(login))
            .route("/init", web::post().to(create_superuser))
            .route("/signup", web::post().to(signup)),
    );
    // API routes - require authentication
    cfg.service(
        web::scope("/api")
            .wrap(JwtAuth)
            .route("/shorten", web::post().to(create_short_url))
            .service(web::resource("/urls").route(web::get().to(get_all_urls)))
            .route("/health/check", web::get().to(health_check))
            .service(web::resource("/qr/{code}/regenerate").route(web::get().to(regenerate_qr)))
            .service(web::resource("/qr/{code}/info").route(web::get().to(get_qr_code_direct)))
            .service(web::resource("/analytics/{code}").route(web::get().to(get_url_analytics)))
            .service(
                web::resource("/qr")
                    .route(web::post().to(generate_direct_qr))
                    .route(web::get().to(get_all_qr_codes)),
            )
            // User management routes - require superuser role
            .service(
                web::scope("/users")
                    .route("/", web::get().to(get_all_users))
                    .route("/", web::post().to(create_user))
                    .route("/{user_id}", web::get().to(get_user))
                    .route("/{user_id}", web::put().to(edit_user))
                    .route("/{user_id}", web::delete().to(delete_user)),
            ),
    );
}
