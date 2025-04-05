use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct CreateQrRequest {
    #[validate(url(message = "Invalid URL format"))]
    pub url: String,
    pub size: Option<u32>,
    pub force_regenerate: Option<bool>,
}

/// Force regenerate QR code
#[derive(Deserialize)]
pub struct RegenerateQrParams {
    pub force: Option<bool>,
    pub url_type: Option<String>, // "original" or "shortened" (default)
}

#[derive(Deserialize)]
pub struct QrRequest {
    pub url_type: Option<String>, // "original" or "shortened" (default)
}

// New struct for QR code response
#[derive(Serialize)]
pub struct QrCodeResponse {
    pub id: String,
    pub short_code: String,
    pub original_url: String,
    pub generated_at: i64,
    pub target_type: String,
    pub is_direct: bool,
    pub svg_content: String,
}

// New struct for QR code search parameters
#[derive(Deserialize)]
pub struct QrSearchParams {
    pub search: Option<String>,
    pub target_type: Option<String>,
    pub direct_only: Option<bool>,
}
