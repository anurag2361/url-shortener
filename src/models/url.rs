use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShortenedUrl {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub original_url: String,
    pub short_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub created_at: Option<chrono::DateTime<chrono::Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<chrono::DateTime<chrono::Utc>>,
}

impl ShortenedUrl {
    pub fn new(original_url: String, short_code: String, expires_in_days: Option<u32>) -> Self {
        let now = chrono::Utc::now();
        let expires_at = expires_in_days.map(|days| now + chrono::Duration::days(days as i64));

        Self {
            id: None,
            original_url,
            short_code,
            created_at: Some(now),
            expires_at,
        }
    }

    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expiry) => chrono::Utc::now() > expiry,
            None => false, // No expiration date means it never expires
        }
    }
}
