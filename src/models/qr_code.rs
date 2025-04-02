use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct QrCode {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub short_code: String,      // Reference to the shortened URL
    pub original_url: String,    // The original URL the QR code points to
    pub svg_content: String,     // The SVG content of the QR code
    pub generated_at: i64,       // When the QR code was generated (timestamp in milliseconds)
    pub target_type: TargetType, // Whether the QR points to the original or shortened URL
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum TargetType {
    #[serde(rename = "original")]
    Original,
    #[serde(rename = "shortened")]
    Shortened,
}

impl QrCode {
    pub fn new(
        short_code: String,
        original_url: String,
        svg_content: String,
        target_type: TargetType,
    ) -> Self {
        Self {
            id: None,
            short_code,
            original_url,
            svg_content,
            generated_at: chrono::Utc::now().timestamp_millis(),
            target_type,
        }
    }
}
