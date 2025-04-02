use actix_web::{HttpResponse, Responder, Result, error, web};
use mongodb::bson::doc;
use qrcode::QrCode;
use qrcode::render::svg;

use crate::models::url::ShortenedUrl;
use crate::state::app_state::AppState;

/// Generate QR code for a shortened URL
pub async fn generate_qr(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let code = path.into_inner();
    let db = &app_state.db;
    let urls_collection = db.collection::<ShortenedUrl>("urls");

    // Find the URL by short code
    let url_doc = urls_collection
        .find_one(doc! {"short_code": &code})
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    match url_doc {
        Some(url) => {
            // Check if URL has expired
            if url.is_expired() {
                return Ok(HttpResponse::Gone().json(serde_json::json!({
                    "error": "This QR code has expired"
                })));
            }
            // Construct the short URL
            let host =
                std::env::var("HOST").unwrap_or_else(|_| String::from("http://localhost:8080"));
            let short_url = format!("{}/r/{}", host, code);

            // Generate QR code
            let qr_code = QrCode::new(short_url.as_bytes()).map_err(|e| {
                error::ErrorInternalServerError(format!("QR code generation error: {}", e))
            })?;

            // Render as SVG
            let svg = qr_code
                .render::<svg::Color>()
                .min_dimensions(200, 200)
                .quiet_zone(true)
                .build();

            Ok(HttpResponse::Ok().content_type("image/svg+xml").body(svg))
        }
        None => Ok(HttpResponse::NotFound().body("URL not found")),
    }
}
