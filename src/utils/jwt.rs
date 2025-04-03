use anyhow::{Context, Result};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use std::env;

use crate::models::role::Role;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String, // Subject (username)
    pub roles: Vec<Role>,
    pub exp: usize, // Expiration time (as UTC timestamp)
    pub iat: usize, // Issued at (as UTC timestamp)
}

pub fn create_token(username: &str, roles: &[Role]) -> Result<String> {
    let jwt_secret = env::var("JWT_SECRET").context("JWT_SECRET not set")?;

    let now = chrono::Utc::now();
    let expiry = now + chrono::Duration::days(10); // 10 days validity

    let claims = Claims {
        sub: username.to_string(),
        roles: roles.to_vec(),
        exp: expiry.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(jwt_secret.as_bytes()),
    )
    .context("Failed to encode JWT")?;

    Ok(token)
}

pub fn validate_token(token: &str) -> Result<Claims> {
    let jwt_secret = env::var("JWT_SECRET").context("JWT_SECRET not set")?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .context("Failed to decode JWT")?;

    Ok(token_data.claims)
}
