use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct UrlVisitor {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub short_code: String,         // Link to the shortened URL
    pub visitor_hash: String,       // Hashed IP address
    pub timestamp: i64,             // When the visit occurred
    pub user_agent: Option<String>, // Optional user agent info
    pub referrer: Option<String>,   // Optional referrer info
}

impl UrlVisitor {
    pub fn new(
        short_code: String,
        visitor_hash: String,
        user_agent: Option<String>,
        referrer: Option<String>,
    ) -> Self {
        Self {
            id: None,
            short_code,
            visitor_hash,
            timestamp: chrono::Utc::now().timestamp_millis(),
            user_agent,
            referrer,
        }
    }
}
