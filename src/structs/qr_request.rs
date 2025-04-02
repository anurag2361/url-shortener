use serde::Deserialize;
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
