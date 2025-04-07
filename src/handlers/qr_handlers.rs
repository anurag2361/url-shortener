use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, Result, error, web};
use mongodb::bson::doc;
use mongodb::options::{FindOneAndUpdateOptions, ReturnDocument};
use qrcode::QrCode as QrCodeGenerator;
use qrcode::render::svg;
use validator::Validate;

use crate::models::qr_code::{QrCode as QrCodeModel, TargetType};
use crate::models::url::ShortenedUrl;
use crate::state::app_state::AppState;
use crate::structs::qr_request::{CreateQrRequest, RegenerateQrParams};
use crate::structs::qr_request::{QrCodeResponse, QrSearchParams};
use crate::utils::jwt::Claims;
use futures_util::TryStreamExt;

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

/// Generate QR code directly from a URL without requiring a short code
pub async fn generate_direct_qr(
    app_state: web::Data<AppState>,
    req: HttpRequest,
    web::Json(req_body): web::Json<CreateQrRequest>,
) -> Result<impl Responder> {
    // Validate the URL
    if let Err(errors) = req_body.validate() {
        return Ok(HttpResponse::BadRequest().json(errors));
    }

    // Get user ID from request extensions
    let user_id = req
        .extensions()
        .get::<Claims>()
        .map(|claims| claims.user_id.clone());

    let db = &app_state.db;
    let qr_codes_collection = db.collection::<QrCodeModel>("qr_codes");

    // First check if we already have a QR code for this URL
    let existing_qr = qr_codes_collection
        .find_one(doc! {
            "original_url": &req_body.url,
            "short_code": { "$regex": "^direct-" }, // Find direct QR codes
            "target_type": "original"
        })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    // Check if QR exists and handle regeneration
    let has_existing_qr = existing_qr.is_some();
    if has_existing_qr {
        if !req_body.force_regenerate.unwrap_or(false) {
            return Ok(HttpResponse::Ok()
                .content_type("image/svg+xml")
                .body(existing_qr.unwrap().svg_content));
        }
    }

    // Set dimensions (default or from request)
    let dimensions = req_body.size.unwrap_or(200);

    // Generate QR code
    let qr_code = QrCodeGenerator::new(req_body.url.as_bytes())
        .map_err(|e| error::ErrorInternalServerError(format!("QR code generation error: {}", e)))?;

    // Render as SVG
    let svg_output = qr_code
        .render::<svg::Color>()
        .min_dimensions(dimensions, dimensions)
        .quiet_zone(true)
        .build();

    // Generate a unique ID for this direct QR code
    let unique_id = format!(
        "direct-{}",
        uuid::Uuid::new_v4().to_string().split('-').next().unwrap()
    );

    // Create the QR code model
    let qr_model = QrCodeModel::new(
        unique_id.clone(),
        req_body.url.clone(),
        svg_output.clone(),
        TargetType::Original, // Direct QR codes always point to the original URL
        user_id.clone(),
    );

    // Save the QR code to the database (upsert if it already exists)
    if existing_qr.is_some() {
        // Update existing QR code
        qr_codes_collection
            .update_one(
                doc! {
                    "original_url": &req_body.url,
                    "short_code": { "$regex": "^direct-" },
                    "target_type": "original"
                },
                doc! {
                    "$set": {
                        "svg_content": &svg_output,
                        "generated_at": chrono::Utc::now().timestamp_millis(),
                    }
                },
            )
            .await
            .map_err(|e| {
                error::ErrorInternalServerError(format!("Failed to update QR code: {}", e))
            })?;
    } else {
        // Insert new QR code
        qr_codes_collection
            .insert_one(&qr_model)
            .await
            .map_err(|e| {
                error::ErrorInternalServerError(format!("Failed to save QR code: {}", e))
            })?;
    }

    // Return the SVG directly
    Ok(HttpResponse::Ok()
        .content_type("image/svg+xml")
        .body(svg_output))
}

/// Get all QR codes
pub async fn get_all_qr_codes(
    app_state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<QrSearchParams>,
) -> Result<impl Responder> {
    let db = &app_state.db;
    let qr_codes_collection = db.collection::<QrCodeModel>("qr_codes");

    // Get current user ID from request
    let current_user_id = req
        .extensions()
        .get::<Claims>()
        .map(|claims| claims.user_id.clone());

    // Build filter based on search parameters
    let mut filter = doc! {};

    // Filter by search term if provided
    if let Some(search) = &query.search {
        if !search.is_empty() {
            filter = doc! {
                "$or": [
                    { "short_code": { "$regex": search, "$options": "i" } },
                    { "original_url": { "$regex": search, "$options": "i" } }
                ]
            };
        }
    }

    // Filter by target type if provided
    if let Some(target_type) = &query.target_type {
        if target_type == "original" || target_type == "shortened" {
            filter.insert("target_type", target_type);
        }
    }

    // Filter direct QR codes if requested
    if query.direct_only.unwrap_or(false) {
        filter.insert("short_code", doc! { "$regex": "^direct-" });
    }

    // Filter for user's own QR codes if requested
    if query.owned_only.unwrap_or(false) {
        if let Some(user_id) = &current_user_id {
            filter.insert("user_id", user_id);
        }
    }

    // Find QR codes
    let cursor = qr_codes_collection
        .find(filter)
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    // Process results
    let qr_codes = cursor
        .try_collect::<Vec<QrCodeModel>>()
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    // Transform to response objects
    let qr_responses: Vec<QrCodeResponse> = qr_codes
        .into_iter()
        .map(|qr| {
            let owned_by_current_user = match (&current_user_id, &qr.user_id) {
                (Some(current_id), Some(qr_id)) => current_id == qr_id,
                _ => false,
            };
            let is_direct = qr.short_code.starts_with("direct-");

            QrCodeResponse {
                id: qr.id.map_or_else(|| "".to_string(), |id| id.to_hex()),
                short_code: qr.short_code,
                original_url: qr.original_url,
                generated_at: qr.generated_at,
                target_type: match qr.target_type {
                    TargetType::Original => "original".to_string(),
                    TargetType::Shortened => "shortened".to_string(),
                },
                is_direct,
                owned_by_current_user,
                user_id: qr.user_id,
                svg_content: qr.svg_content,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(qr_responses))
}

// New handler that lists QR codes for a specific user
pub async fn get_user_qr_codes(
    app_state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<QrSearchParams>,
) -> Result<impl Responder> {
    let user_id = path.into_inner();
    let db = &app_state.db;
    let qr_codes_collection = db.collection::<QrCodeModel>("qr_codes");

    // Get current user ID from request
    let current_user_id = req
        .extensions()
        .get::<Claims>()
        .map(|claims| claims.user_id.clone());

    // Build filter based on search parameters
    let mut filter = doc! { "user_id": &user_id };

    // Filter by search term if provided
    if let Some(search) = &query.search {
        if !search.is_empty() {
            // Combine user_id with search
            filter = doc! {
                "$and": [
                    { "user_id": &user_id },
                    { "$or": [
                        { "short_code": { "$regex": search, "$options": "i" } },
                        { "original_url": { "$regex": search, "$options": "i" } }
                    ]}
                ]
            };
        }
    }

    // Filter by target type if provided
    if let Some(target_type) = &query.target_type {
        if target_type == "original" || target_type == "shortened" {
            if let Some(and_array) = filter.get_array_mut("$and").ok() {
                and_array.push(doc! { "target_type": target_type }.into());
            } else {
                filter.insert("target_type", target_type);
            }
        }
    }

    // Filter direct QR codes if requested
    if query.direct_only.unwrap_or(false) {
        if let Some(and_array) = filter.get_array_mut("$and").ok() {
            and_array.push(doc! { "short_code": { "$regex": "^direct-" } }.into());
        } else {
            filter.insert("short_code", doc! { "$regex": "^direct-" });
        }
    }

    // Find QR codes
    let cursor = qr_codes_collection
        .find(filter)
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    // Process results
    let qr_codes = cursor
        .try_collect::<Vec<QrCodeModel>>()
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    // Transform to response objects
    let qr_responses: Vec<QrCodeResponse> = qr_codes
        .into_iter()
        .map(|qr| {
            let owned_by_current_user = match (&current_user_id, &qr.user_id) {
                (Some(current_id), Some(qr_id)) => current_id == qr_id,
                _ => false,
            };

            let short_code = qr.short_code.clone();
            QrCodeResponse {
                id: qr.id.map_or_else(|| "".to_string(), |id| id.to_hex()),
                short_code: short_code.clone(),
                original_url: qr.original_url,
                generated_at: qr.generated_at,
                target_type: match qr.target_type {
                    TargetType::Original => "original".to_string(),
                    TargetType::Shortened => "shortened".to_string(),
                },
                is_direct: short_code.starts_with("direct-"),
                owned_by_current_user,
                user_id: qr.user_id,
                svg_content: qr.svg_content,
            }
        })
        .collect();

    Ok(HttpResponse::Ok().json(qr_responses))
}
