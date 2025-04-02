use serde::{Deserialize, Serialize};
use validator::Validate;

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
    pub has_shortened_qr: bool,
    pub has_original_qr: bool,
    pub clicks: i64,
    pub unique_clicks: usize,
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
    pub unique_clicks: usize,
    pub has_shortened_qr: bool,
    pub has_original_qr: bool,
    pub shortened_qr_generated_at: Option<i64>,
    pub original_qr_generated_at: Option<i64>,
}
