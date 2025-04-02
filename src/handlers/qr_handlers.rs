use actix_web::{HttpResponse, Responder, Result, error, web};
use mongodb::bson::doc;
use mongodb::options::{FindOneAndUpdateOptions, ReturnDocument};
use qrcode::QrCode as QrCodeGenerator;
use qrcode::render::svg;
use serde::Deserialize;

use crate::models::qr_code::{QrCode as QrCodeModel, TargetType};
use crate::models::url::ShortenedUrl;
use crate::state::app_state::AppState;

#[derive(Deserialize)]
pub struct QrRequest {
    pub url_type: Option<String>, // "original" or "shortened" (default)
}

/// Generate QR code for a shortened URL
pub async fn generate_qr(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<QrRequest>,
) -> Result<impl Responder> {
    let code = path.into_inner();
    let db = &app_state.db;
    let urls_collection = db.collection::<ShortenedUrl>("urls");
    let qr_codes_collection = db.collection::<QrCodeModel>("qr_codes");

    // Determine target type from query parameter
    let target_type = match query.url_type.as_deref() {
        Some("original") => TargetType::Original,
        _ => TargetType::Shortened,
    };

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

            // Check if QR code already exists for this URL and target type
            let existing_qr = qr_codes_collection
                .find_one(doc! {
                    "short_code": &code,
                    "target_type": match target_type {
                        TargetType::Original => "original",
                        TargetType::Shortened => "shortened",
                    }
                })
                .await
                .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

            if let Some(qr) = existing_qr {
                return Ok(HttpResponse::Ok()
                    .content_type("image/svg+xml")
                    .body(qr.svg_content));
            }

            // Generate QR code
            let target_url = match target_type {
                TargetType::Original => url.original_url.clone(),
                TargetType::Shortened => {
                    let host = std::env::var("HOST")
                        .unwrap_or_else(|_| String::from("http://localhost:8080"));
                    format!("{}/r/{}", host, code)
                }
            };

            // Generate QR code
            let qr_code = QrCodeGenerator::new(target_url.as_bytes()).map_err(|e| {
                error::ErrorInternalServerError(format!("QR code generation error: {}", e))
            })?;

            // Render as SVG
            let svg_output = qr_code
                .render::<svg::Color>()
                .min_dimensions(200, 200)
                .quiet_zone(true)
                .build();

            // Save the QR code to the database
            let new_qr = QrCodeModel::new(
                code.clone(),
                url.original_url.clone(),
                svg_output.clone(),
                target_type,
            );

            qr_codes_collection.insert_one(&new_qr).await.map_err(|e| {
                error::ErrorInternalServerError(format!("Failed to save QR code: {}", e))
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
    pub url_type: Option<String>, // "original" or "shortened" (default)
}

pub async fn regenerate_qr(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<RegenerateQrParams>,
) -> Result<impl Responder> {
    let code = path.into_inner();
    let force = query.force.unwrap_or(false);

    // Determine target type from query parameter
    let target_type = match query.url_type.as_deref() {
        Some("original") => TargetType::Original,
        _ => TargetType::Shortened,
    };

    let db = &app_state.db;
    let urls_collection = db.collection::<ShortenedUrl>("urls");
    let qr_codes_collection = db.collection::<QrCodeModel>("qr_codes");

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

            // Check if QR code already exists and if force=false, return existing QR
            if !force {
                let existing_qr = qr_codes_collection
                    .find_one(doc! {
                        "short_code": &code,
                        "target_type": match target_type {
                            TargetType::Original => "original",
                            TargetType::Shortened => "shortened",
                        }
                    })
                    .await
                    .map_err(|e| {
                        error::ErrorInternalServerError(format!("Database error: {}", e))
                    })?;

                if let Some(qr) = existing_qr {
                    return Ok(HttpResponse::Ok()
                        .content_type("image/svg+xml")
                        .body(qr.svg_content));
                }
            }

            // Generate QR code
            let target_url = match target_type {
                TargetType::Original => url.original_url.clone(),
                TargetType::Shortened => {
                    let host = std::env::var("HOST")
                        .unwrap_or_else(|_| String::from("http://localhost:8080"));
                    format!("{}/r/{}", host, code)
                }
            };

            let qr_code = QrCodeGenerator::new(target_url.as_bytes()).map_err(|e| {
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

            // Update or insert QR code
            qr_codes_collection
                .find_one_and_update(
                    doc! {
                        "short_code": &code,
                        "target_type": match target_type {
                            TargetType::Original => "original",
                            TargetType::Shortened => "shortened",
                        }
                    },
                    doc! {
                        "$set": {
                            "svg_content": &svg_output,
                            "generated_at": chrono::Utc::now().timestamp_millis(),
                        }
                    },
                )
                .with_options(find_options)
                .await
                .map_err(|e| {
                    error::ErrorInternalServerError(format!("Failed to update QR code: {}", e))
                })?;

            Ok(HttpResponse::Ok()
                .content_type("image/svg+xml")
                .body(svg_output))
        }
        None => Ok(HttpResponse::NotFound().body("URL not found")),
    }
}
