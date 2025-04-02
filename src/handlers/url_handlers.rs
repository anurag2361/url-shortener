use actix_web::{HttpRequest, HttpResponse, Responder, Result, error, http, web};
use futures_util::StreamExt;
use mongodb::bson::doc;
use nanoid::nanoid;
use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::models::url::ShortenedUrl;
use crate::models::url_visitor::UrlVisitor;
use crate::state::app_state::AppState;
use crate::utils::hash_ip::hash_ip;

#[derive(Deserialize, Serialize, Validate)]
pub struct UrlRequest {
    #[validate(url(message = "Invalid URL format"))]
    pub url: String,
    pub custom_code: Option<String>,
    pub expires_in_days: Option<u32>,
}

#[derive(Serialize)]
pub struct UrlListResponse {
    pub id: Option<String>,
    pub original_url: String,
    pub short_code: String,
    pub created_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub has_qr_code: bool,
    pub qr_code_generated_at: Option<i64>,
    pub clicks: i64,          // Add click count to the response
    pub unique_clicks: usize, // Count of unique visitors
}

#[derive(Serialize)]
pub struct UrlResponse {
    pub original_url: String,
    pub short_url: String,
    pub short_code: String,
    pub expires_at: Option<i64>,
}

#[derive(Deserialize)]
pub struct UrlSearchParams {
    pub search: Option<String>,
}

#[derive(Serialize)]
pub struct UrlAnalyticsResponse {
    pub short_code: String,
    pub original_url: String,
    pub created_at: Option<i64>,
    pub expires_at: Option<i64>,
    pub clicks: i64,
    pub has_qr_code: bool,
    pub qr_code_generated_at: Option<i64>,
    pub unique_clicks: usize,
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
    query: web::Query<UrlSearchParams>,
) -> Result<impl Responder> {
    let db = &app_state.db;
    let urls_collection = db.collection::<ShortenedUrl>("urls");
    let visitors_collection = db.collection::<UrlVisitor>("visitors");

    // Create filter based on search parameter
    let filter = match &query.search {
        Some(search_term) if !search_term.is_empty() => {
            // Case-insensitive search in original_url field
            doc! {
                "original_url": {
                    "$regex": search_term,
                    "$options": "i"  // case-insensitive
                }
            }
        }
        _ => doc! {}, // Empty filter returns all documents
    };

    // Find all URLs
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

            // Create a simplified response without the SVG data
            urls.push(UrlListResponse {
                id: id_str,
                original_url: url.original_url,
                short_code: url.short_code,
                created_at: url.created_at,
                expires_at: url.expires_at,
                has_qr_code: url.qr_code_svg.is_some(),
                qr_code_generated_at: url.qr_code_generated_at,
                clicks: url.clicks,                  // Include click count
                unique_clicks: unique_visitor_count, // Count of unique visitors
            });
        }
    }

    Ok(HttpResponse::Ok().json(urls))
}

/// Get QR code as SVG
pub async fn get_qr_code_direct(
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
            match url.qr_code_svg {
                Some(svg) => {
                    // Return the SVG directly with the correct content-type
                    Ok(HttpResponse::Ok().content_type("image/svg+xml").body(svg))
                }
                None => Ok(HttpResponse::NotFound().body("QR code not generated for this URL")),
            }
        }
        None => Ok(HttpResponse::NotFound().body("URL not found")),
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

            let analytics = UrlAnalyticsResponse {
                short_code: url.short_code,
                original_url: url.original_url,
                created_at: url.created_at,
                expires_at: url.expires_at,
                clicks: url.clicks,
                has_qr_code: url.qr_code_svg.is_some(),
                qr_code_generated_at: url.qr_code_generated_at,
                unique_clicks: unique_visitor_count, // Count of unique visitors
            };

            Ok(HttpResponse::Ok().json(analytics))
        }
        None => Ok(HttpResponse::NotFound().json(serde_json::json!({
            "error": "URL not found"
        }))),
    }
}
