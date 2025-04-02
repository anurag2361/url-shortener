use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct CreateQrRequest {
    #[validate(url(message = "Invalid URL format"))]
    pub url: String,
    pub size: Option<u32>,
    pub force_regenerate: Option<bool>,
}
