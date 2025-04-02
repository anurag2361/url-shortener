use actix_web::{HttpResponse, Responder, Result, error, web};
use mongodb::bson::doc;
use mongodb::options::{FindOneAndUpdateOptions, ReturnDocument};
use qrcode::QrCode;
use qrcode::render::svg;
use serde::Deserialize;

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

            // Check if QR code already exists and use it
            if let Some(existing_svg) = url.qr_code_svg {
                return Ok(HttpResponse::Ok()
                    .content_type("image/svg+xml")
                    .body(existing_svg));
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
            let svg_output = qr_code
                .render::<svg::Color>()
                .min_dimensions(200, 200)
                .quiet_zone(true)
                .build();

            // Save the SVG to the database
            let find_options = FindOneAndUpdateOptions::builder()
                .return_document(ReturnDocument::After)
                .build();

            urls_collection
                .find_one_and_update(
                    doc! {"short_code": &code},
                    doc! {
                        "$set": {
                            "qr_code_svg": &svg_output,
                            "qr_code_generated_at": chrono::Utc::now().timestamp_millis(), // Store timestamp in milliseconds
                        }
                    },
                )
                .with_options(find_options)
                .await
                .map_err(|e| {
                    let error_message = e.to_string();
                    if error_message.contains(
                        "invalid type: map, expected an RFC 3339 formatted date and time string",
                    ) {
                        log::error!("Failed to update QR code due to invalid date format: {}", e); // Log the specific error
                        error::ErrorInternalServerError(
                            "An unexpected error occurred while updating the QR code.",
                        ) // Return a generic error message
                    } else {
                        error::ErrorInternalServerError(format!("Failed to update QR code: {}", e)) // Return other errors as they are
                    }
                })?;

            Ok(HttpResponse::Ok()
                .content_type("image/svg+xml")
                .body(svg_output))
        }
        None => Ok(HttpResponse::NotFound().body("URL not found")),
    }
}

/// Force regenerate QR code
#[derive(Deserialize)]
pub struct RegenerateQrParams {
    pub force: Option<bool>,
}

pub async fn regenerate_qr(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<RegenerateQrParams>,
) -> Result<impl Responder> {
    let code = path.into_inner();
    let force = query.force.unwrap_or(false);

    let db = &app_state.db;
    let urls_collection = db.collection::<ShortenedUrl>("urls");

    // Find the URL by short code
    let url_doc = urls_collection
        .find_one(doc! {"short_code": &code})
        .await
        .map_err(|e| {
            let error_message = e.to_string();
            if error_message
                .contains("invalid type: map, expected an RFC 3339 formatted date and time string")
            {
                log::error!("Failed to update QR code due to invalid date format: {}", e); // Log the specific error
                error::ErrorInternalServerError(
                    "An unexpected error occurred while updating the QR code.",
                ) // Return a generic error message
            } else {
                error::ErrorInternalServerError(format!("Failed to update QR code: {}", e)) // Return other errors as they are
            }
        })?;

    match url_doc {
        Some(url) => {
            // Check if URL has expired
            if url.is_expired() {
                return Ok(HttpResponse::Gone().json(serde_json::json!({
                    "error": "This QR code has expired"
                })));
            }

            // If not forcing regeneration and QR exists, return the existing one
            if !force && url.qr_code_svg.is_some() {
                return Ok(HttpResponse::Ok()
                    .content_type("image/svg+xml")
                    .body(url.qr_code_svg.unwrap()));
            }

            // Generate a new QR code
            let host =
                std::env::var("HOST").unwrap_or_else(|_| String::from("http://localhost:8080"));
            let short_url = format!("{}/r/{}", host, code);

            let qr_code = QrCode::new(short_url.as_bytes()).map_err(|e| {
                error::ErrorInternalServerError(format!("QR code generation error: {}", e))
            })?;

            let svg_output = qr_code
                .render::<svg::Color>()
                .min_dimensions(200, 200)
                .quiet_zone(true)
                .build();

            // Save the regenerated SVG
            let find_options = FindOneAndUpdateOptions::builder()
                .return_document(ReturnDocument::After)
                .build();

            urls_collection
                .find_one_and_update(
                    doc! {"short_code": &code},
                    doc! {
                        "$set": {
                            "qr_code_svg": &svg_output,
                            "qr_code_generated_at": chrono::Utc::now().timestamp_millis(), // Store timestamp in milliseconds
                        }
                    },
                )
                .with_options(find_options)
                .await
                .map_err(|e| {
                    let error_message = e.to_string();
                    if error_message.contains(
                        "invalid type: map, expected an RFC 3339 formatted date and time string",
                    ) {
                        log::error!("Failed to update QR code due to invalid date format: {}", e); // Log the specific error
                        error::ErrorInternalServerError(
                            "An unexpected error occurred while updating the QR code.",
                        ) // Return a generic error message
                    } else {
                        error::ErrorInternalServerError(format!("Failed to update QR code: {}", e)) // Return other errors as they are
                    }
                })?;

            Ok(HttpResponse::Ok()
                .content_type("image/svg+xml")
                .body(svg_output))
        }
        None => Ok(HttpResponse::NotFound().body("URL not found")),
    }
}
