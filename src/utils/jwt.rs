use anyhow::{Context, Result};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::env;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,     // Subject (username)
    pub exp: usize,      // Expiration time
    pub iat: usize,      // Issued at
    pub user_id: String, // Optional user ID
}

pub fn create_token(username: &str, user_id: &str) -> Result<String> {
    let expiration = chrono::Utc::now()
        .checked_add_signed(chrono::Duration::days(10))
        .context("Invalid timestamp")?
        .timestamp() as usize;

    let issued_at = chrono::Utc::now().timestamp() as usize;

    let claims = Claims {
        sub: username.to_owned(),
        exp: expiration,
        iat: issued_at,
        user_id: user_id.to_owned(),
    };

    let secret = env::var("JWT_SECRET").context("JWT_SECRET must be set")?;
    let encoding_key = EncodingKey::from_secret(secret.as_bytes());

    encode(&Header::default(), &claims, &encoding_key).context("Failed to create token")
}

pub fn validate_token(token: &str) -> Result<Claims> {
    let secret = env::var("JWT_SECRET").context("JWT_SECRET must be set")?;
    let decoding_key = DecodingKey::from_secret(secret.as_bytes());

    let token_data = decode::<Claims>(token, &decoding_key, &Validation::default())
        .context("Failed to validate token")?;

    Ok(token_data.claims)
}
