use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShortenedUrl {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub original_url: String,
    pub short_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_code_svg: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub qr_code_generated_at: Option<i64>,
    #[serde(default)]
    pub clicks: i64, // Number of clicks/redirects tracked
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unique_visitors: Vec<String>, // Store hashed IPs for unique visitors
}

impl ShortenedUrl {
    pub fn new(original_url: String, short_code: String, expires_in_days: Option<u32>) -> Self {
        let now = chrono::Utc::now().timestamp_millis();
        let expires_at = expires_in_days.map(|days| now + (days as i64 * 24 * 60 * 60 * 1000)); // Add days in milliseconds

        Self {
            id: None,
            original_url,
            short_code,
            created_at: Some(now),
            expires_at,
            qr_code_svg: None,
            qr_code_generated_at: None,
            clicks: 0,
            unique_visitors: Vec::new(), // Initialize with an empty vector
        }
    }

    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expiry) => chrono::Utc::now().timestamp_millis() > expiry,
            None => false, // No expiration date means it never expires
        }
    }

    pub fn unique_visitor_count(&self) -> usize {
        self.unique_visitors.len()
    }
}
