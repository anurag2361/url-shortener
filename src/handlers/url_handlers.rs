use actix_web::{HttpResponse, Responder, Result, error, http, web};
use mongodb::bson::doc;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::models::url::ShortenedUrl;
use crate::state::app_state::AppState;

#[derive(Deserialize, Serialize, Validate)]
pub struct UrlRequest {
    #[validate(url(message = "Invalid URL format"))]
    pub url: String,
    pub custom_code: Option<String>,
    pub expires_in_days: Option<u32>,
}

#[derive(Serialize)]
pub struct UrlResponse {
    pub original_url: String,
    pub short_url: String,
    pub short_code: String,
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

/// Create a shortened URL
pub async fn create_short_url(
    app_state: web::Data<AppState>,
    web::Json(req): web::Json<UrlRequest>,
) -> Result<impl Responder> {
    // Validate the URL
    if let Err(errors) = req.validate() {
        return Ok(HttpResponse::BadRequest().json(errors));
    }

    let db = &app_state.db;
    let urls_collection = db.collection::<ShortenedUrl>("urls");

    // Generate short code - either use custom or generate random
    let short_code = match req.custom_code {
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
    let shortened_url = ShortenedUrl::new(req.url.clone(), short_code.clone(), req.expires_in_days);

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
        original_url: req.url,
        short_url,
        short_code,
        expires_at: shortened_url.expires_at,
    };

    Ok(HttpResponse::Created().json(response))
}

/// Redirect to original URL
pub async fn redirect_to_url(
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
                    "error": "This URL has expired"
                })));
            }

            Ok(HttpResponse::Found()
                .append_header((http::header::LOCATION, url.original_url))
                .finish())
        }
        None => Ok(HttpResponse::NotFound().body("Short URL not found")),
    }
}
