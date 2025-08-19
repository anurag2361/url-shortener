use actix_web::web;

use crate::handlers::auth_handlers::{create_superuser, login, signup};
use crate::handlers::health_handlers::health_check;
use crate::handlers::qr_handlers::{
    generate_direct_qr, get_all_qr_codes, get_user_qr_codes, regenerate_qr,
};
use crate::handlers::url_handlers::{
    create_short_url, get_all_urls, get_qr_code_direct, get_url_analytics, get_user_urls,
    redirect_to_url,delete_short_url
};
use crate::handlers::user_handlers::{
    create_user, delete_user, edit_user, get_all_users, get_user,
};
use crate::middlewares::authmw::JwtAuth;
use crate::middlewares::res_owner::ResourceOwnership;

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
            .route("/urls", web::get().to(get_all_urls))
            .route("/urls/{code}", web::delete().to(delete_short_url))
            .service(
                web::resource("/users/{user_id}/urls")
                    .wrap(ResourceOwnership {
                        param_name: "user_id".to_string(),
                    })
                    .route(web::get().to(get_user_urls)),
            )
            .service(
                web::resource("/users/{user_id}/qr")
                    .wrap(ResourceOwnership {
                        param_name: "user_id".to_string(),
                    })
                    .route(web::get().to(get_user_qr_codes)),
            )
            .route("/health/check", web::get().to(health_check))
            .route("/qr/{code}/regenerate", web::get().to(regenerate_qr))
            .route("/qr/{code}/info", web::get().to(get_qr_code_direct))
            .route("/analytics/{code}", web::get().to(get_url_analytics))
            .route("/qr", web::post().to(generate_direct_qr))
            .route("/qr", web::get().to(get_all_qr_codes))
            // User management routes
            .service(
                web::scope("/users")
                    .route("", web::get().to(get_all_users))
                    .route("", web::post().to(create_user))
                    .route("/{user_id}", web::get().to(get_user))
                    .route("/{user_id}", web::put().to(edit_user))
                    .route("/{user_id}", web::delete().to(delete_user)),
            ),
    );
}
