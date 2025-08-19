use actix_web::{HttpMessage, HttpRequest, HttpResponse, Responder, Result, error, http, web};
use futures_util::StreamExt;
use mongodb::bson::doc;
use nanoid::nanoid;
use validator::Validate;

use crate::models::qr_code::{QrCode, TargetType};
use crate::models::url::ShortenedUrl;
use crate::models::url_visitor::UrlVisitor;
use crate::state::app_state::AppState;
use crate::structs::qr_request::QrRequest;
use crate::structs::url_request::{
    UrlAnalyticsResponse, UrlListResponse, UrlRequest, UrlResponse, UrlSearchParams,
};
use crate::utils::hash_ip::hash_ip;
use crate::utils::jwt::Claims;

/// Create a shortened URL
pub async fn create_short_url(
    app_state: web::Data<AppState>,
    req: HttpRequest,
    web::Json(req_body): web::Json<UrlRequest>,
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
    let urls_collection = db.collection::<ShortenedUrl>("urls");

    // Generate short code - either use custom or generate random
    let short_code = match req_body.custom_code {
        Some(code) if !code.is_empty() => {
            // Check if custom code already exists
            let existing = urls_collection
                .find_one(doc! {"short_code": &code})
                .await
                .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

            if existing.is_some() {
                return Ok(HttpResponse::Conflict().json(serde_json::json!({
                    "error": "Custom code already in use"
                })));
            }

            code
        }
        _ => nanoid!(6), // Generate a 6-character nanoid
    };

    // Create new shortened URL
    let shortened_url = ShortenedUrl::new(
        req_body.url.clone(),
        short_code.clone(),
        req_body.expires_in_days,
        user_id,
    );

    // Save to database
    urls_collection
        .insert_one(&shortened_url)
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    // Construct the short URL
    let host = std::env::var("HOST").unwrap_or_else(|_| String::from("http://localhost:8080"));
    let short_url = format!("{}/r/{}", host, short_code);

    // Return response
    let response = UrlResponse {
        original_url: req_body.url,
        short_url,
        short_code,
        expires_at: shortened_url.expires_at,
        user_id: shortened_url.user_id,
    };

    Ok(HttpResponse::Created().json(response))
}

/// Redirect to original URL
pub async fn redirect_to_url(
    app_state: web::Data<AppState>,
    req: HttpRequest,
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
                    "error": "This URL has expired"
                })));
            }

            // Get visitor's IP address
            let ip = req
                .connection_info()
                .realip_remote_addr()
                .unwrap_or("unknown")
                .to_string();

            // Create a unique visitor identifier by hashing the IP
            let visitor_hash = hash_ip(&ip);

            // Get optional user agent and referrer
            let user_agent = req
                .headers()
                .get(http::header::USER_AGENT)
                .and_then(|v| v.to_str().ok())
                .map(String::from);

            let referrer = req
                .headers()
                .get(http::header::REFERER)
                .and_then(|v| v.to_str().ok())
                .map(String::from);

            // Increment the click counter asynchronously
            // We don't wait for the result to avoid slowing down the redirect
            let original_url = url.original_url.clone();
            let code_clone = code.clone();

            let visitors_collection = db.collection::<UrlVisitor>("visitors");

            // Update click count and unique visitors in the background
            actix_web::rt::spawn(async move {
                // Increment the click counter and add the visitor hash if it's new
                let _ = urls_collection
                    .update_one(
                        doc! {"short_code": &code_clone},
                        doc! {
                            "$inc": {"clicks": 1},
                        },
                    )
                    .await;

                // Then, check if this visitor has already visited this URL
                let existing_visitor = visitors_collection
                    .find_one(doc! {
                        "short_code": &code_clone,
                        "visitor_hash": &visitor_hash
                    })
                    .await;

                if let Ok(None) = existing_visitor {
                    // If this is a new visitor, add to the visitors collection
                    let visitor = UrlVisitor::new(code_clone, visitor_hash, user_agent, referrer);
                    let _ = visitors_collection.insert_one(&visitor).await;
                }
            });

            Ok(HttpResponse::Found()
                .append_header((http::header::LOCATION, original_url))
                .finish())
        }
        None => Ok(HttpResponse::NotFound().body("Short URL not found")),
    }
}

pub async fn get_all_urls(
    app_state: web::Data<AppState>,
    req: HttpRequest,
    query: web::Query<UrlSearchParams>,
) -> Result<impl Responder> {
    let db = &app_state.db;
    let urls_collection = db.collection::<ShortenedUrl>("urls");
    let visitors_collection = db.collection::<UrlVisitor>("visitors");
    let qr_codes_collection = db.collection::<QrCode>("qr_codes");

    // Get current user ID from request
    let current_user_id = req
        .extensions()
        .get::<Claims>()
        .map(|claims| claims.user_id.clone());

    // Build filter
    let mut filter = doc! {};

    // Add search filter if provided
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

    // Filter for user's own URLs if requested
    if query.owned_only.unwrap_or(false) {
        if let Some(user_id) = &current_user_id {
            filter.insert("user_id", user_id);
        }
    }

    // Find URLs matching the filter
    let mut cursor = urls_collection
        .find(filter)
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    let mut urls = Vec::new();

    while let Some(result) = cursor.next().await {
        if let Ok(url) = result {
            // Convert ObjectId to string
            let id_str = url.id.map(|oid| oid.to_hex());

            // Get the short code
            let short_code = url.short_code.clone();

            // Count unique visitors for this URL
            let unique_visitor_count = visitors_collection
                .count_documents(doc! {"short_code": &short_code})
                .await
                .unwrap_or(0) as usize;

            // Check if QR codes exist for this URL
            let has_shortened_qr = qr_codes_collection
                .count_documents(doc! {
                    "short_code": &short_code,
                    "target_type": "shortened"
                })
                .await
                .unwrap_or(0)
                > 0;

            let has_original_qr = qr_codes_collection
                .count_documents(doc! {
                    "short_code": &short_code,
                    "target_type": "original"
                })
                .await
                .unwrap_or(0)
                > 0;

            // Determine if this URL is owned by the current user
            let owned_by_current_user = match (&current_user_id, &url.user_id) {
                (Some(current_id), Some(url_id)) => current_id == url_id,
                _ => false,
            };

            urls.push(UrlListResponse {
                id: id_str,
                original_url: url.original_url,
                short_code,
                created_at: url.created_at,
                expires_at: url.expires_at,
                has_shortened_qr,
                has_original_qr,
                clicks: url.clicks,
                unique_clicks: unique_visitor_count,
                owned_by_current_user,
                user_id: url.user_id,
            });
        }
    }

    Ok(HttpResponse::Ok().json(urls))
}

/// Get QR code as SVG
/// Get QR code as SVG
pub async fn get_qr_code_direct(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
    query: web::Query<QrRequest>,
) -> Result<impl Responder> {
    let code = path.into_inner();
    let db = &app_state.db;

    // Determine target type from query parameter
    let target_type = match query.url_type.as_deref() {
        Some("original") => TargetType::Original,
        _ => TargetType::Shortened,
    };

    let qr_codes_collection = db.collection::<QrCode>("qr_codes");

    // Find the QR code by short code and target type
    let qr_doc = qr_codes_collection
        .find_one(doc! {
            "short_code": &code,
            "target_type": match target_type {
                TargetType::Original => "original",
                TargetType::Shortened => "shortened",
            }
        })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    match qr_doc {
        Some(qr) => {
            // Return the SVG directly with the correct content-type
            Ok(HttpResponse::Ok()
                .content_type("image/svg+xml")
                .body(qr.svg_content))
        }
        None => Ok(HttpResponse::NotFound().body("QR code not found for this URL")),
    }
}

/// Get analytics for a specific URL
pub async fn get_url_analytics(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let code = path.into_inner();
    let db = &app_state.db;
    let urls_collection = db.collection::<ShortenedUrl>("urls");
    let visitors_collection = db.collection::<UrlVisitor>("visitors");
    let qr_codes_collection = db.collection::<QrCode>("qr_codes");

    // Find the URL by short code
    let url_doc = urls_collection
        .find_one(doc! {"short_code": &code})
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    match url_doc {
        Some(url) => {
            // Count unique visitors for this URL
            let unique_visitor_count = visitors_collection
                .count_documents(doc! {"short_code": &code})
                .await
                .unwrap_or(0) as usize;

            // Check if QR codes exist for this URL
            let shortened_qr = qr_codes_collection
                .find_one(doc! {
                    "short_code": &code,
                    "target_type": "shortened"
                })
                .await
                .ok()
                .flatten();

            let original_qr = qr_codes_collection
                .find_one(doc! {
                    "short_code": &code,
                    "target_type": "original"
                })
                .await
                .ok()
                .flatten();

            let has_shortened_qr = shortened_qr.is_some();
            let has_original_qr = original_qr.is_some();

            let shortened_qr_generated_at = shortened_qr.map(|qr| qr.generated_at);
            let original_qr_generated_at = original_qr.map(|qr| qr.generated_at);

            let analytics = UrlAnalyticsResponse {
                short_code: url.short_code,
                original_url: url.original_url,
                created_at: url.created_at,
                expires_at: url.expires_at,
                clicks: url.clicks,
                unique_clicks: unique_visitor_count,
                has_shortened_qr,
                has_original_qr,
                shortened_qr_generated_at,
                original_qr_generated_at,
                user_id: url.user_id,
            };

            Ok(HttpResponse::Ok().json(analytics))
        }
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "URL not found"
        }))),
    }
}

// New handler that lists URLs for a specific user
pub async fn get_user_urls(
    app_state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    query: web::Query<UrlSearchParams>,
) -> Result<impl Responder> {
    let user_id = path.into_inner();
    let db = &app_state.db;
    let urls_collection = db.collection::<ShortenedUrl>("urls");
    let visitors_collection = db.collection::<UrlVisitor>("visitors");
    let qr_codes_collection = db.collection::<QrCode>("qr_codes");

    // Get current user ID from request
    let current_user_id = req
        .extensions()
        .get::<Claims>()
        .map(|claims| claims.user_id.clone());

    // Build filter
    let mut filter = doc! { "user_id": &user_id };

    // Add search filter if provided
    if let Some(search) = &query.search {
        if !search.is_empty() {
            // This is a bit more complex now - we need to combine user_id with search
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

    // Find URLs matching the filter
    let mut cursor = urls_collection
        .find(filter)
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    let mut urls = Vec::new();

    while let Some(result) = cursor.next().await {
        if let Ok(url) = result {
            // Convert ObjectId to string
            let id_str = url.id.map(|oid| oid.to_hex());

            // Get the short code
            let short_code = url.short_code.clone();

            // Count unique visitors for this URL
            let unique_visitor_count = visitors_collection
                .count_documents(doc! {"short_code": &short_code})
                .await
                .unwrap_or(0) as usize;

            // Check if QR codes exist for this URL
            let has_shortened_qr = qr_codes_collection
                .count_documents(doc! {
                    "short_code": &short_code,
                    "target_type": "shortened"
                })
                .await
                .unwrap_or(0)
                > 0;

            let has_original_qr = qr_codes_collection
                .count_documents(doc! {
                    "short_code": &short_code,
                    "target_type": "original"
                })
                .await
                .unwrap_or(0)
                > 0;

            // Determine if this URL is owned by the current user
            let owned_by_current_user = match (&current_user_id, &url.user_id) {
                (Some(current_id), Some(url_id)) => current_id == url_id,
                _ => false,
            };

            urls.push(UrlListResponse {
                id: id_str,
                original_url: url.original_url,
                short_code,
                created_at: url.created_at,
                expires_at: url.expires_at,
                has_shortened_qr,
                has_original_qr,
                clicks: url.clicks,
                unique_clicks: unique_visitor_count,
                owned_by_current_user,
                user_id: url.user_id,
            });
        }
    }

    Ok(HttpResponse::Ok().json(urls))
}

/// Delete a shortened URL
pub async fn delete_short_url(
    app_state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> Result<impl Responder> {
    let code = path.into_inner();
    let db = &app_state.db;

    let urls_collection = db.collection::<ShortenedUrl>("urls");
    let qr_codes_collection = db.collection::<QrCode>("qr_codes");
    let visitors_collection = db.collection::<UrlVisitor>("visitors");

    // Get the current user's ID from the token claims
    let extensions = req.extensions();
    let claims = extensions
        .get::<Claims>()
        .ok_or_else(|| error::ErrorInternalServerError("User claims not found in request"))?;

    // Find the URL to be deleted
    let url_to_delete = urls_collection
        .find_one(doc! { "short_code": &code })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?
        .ok_or_else(|| error::ErrorNotFound("URL not found"))?;

    // --- Ownership Check ---
    // Ensure the user deleting the URL is the one who created it
    if url_to_delete.user_id.as_deref() != Some(&claims.user_id) {
        // You could also allow admins to delete any URL here
        return Err(error::ErrorForbidden("You do not have permission to delete this URL"));
    }

    // Delete the URL document
    urls_collection
        .delete_one(doc! { "_id": url_to_delete.id })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Failed to delete URL: {}", e)))?;

    // Delete associated QR codes
    qr_codes_collection
        .delete_many(doc! { "short_code": &code })
        .await
        .ok(); // Use .ok() to ignore errors if deletion fails

    // Delete associated visitor analytics
    visitors_collection
        .delete_many(doc! { "short_code": &code })
        .await
        .ok(); // Use .ok() to ignore errors if deletion fails

    Ok(HttpResponse::NoContent().finish())
}
