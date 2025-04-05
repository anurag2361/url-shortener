use actix_web::{HttpResponse, Result, error, web};
use bcrypt::{DEFAULT_COST, hash, verify};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use crate::models::user::User;
use crate::state::app_state::AppState;
use crate::utils::jwt::create_token;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub username: String,
}

pub async fn login(
    app_state: web::Data<AppState>,
    web::Json(req): web::Json<LoginRequest>,
) -> Result<HttpResponse> {
    let db = &app_state.db;
    let users_collection = db.collection::<User>("users");

    // Find the user
    let user = users_collection
        .find_one(doc! { "username": &req.username, "is_active": true })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    match user {
        Some(mut user) => {
            // Verify password
            let password_matches = verify(&req.password, &user.password_hash)
                .map_err(|_| error::ErrorInternalServerError("Password verification failed"))?;

            if !password_matches {
                return Ok(HttpResponse::Unauthorized().json(serde_json::json!({
                    "error": "Invalid credentials"
                })));
            }

            // Generate JWT token
            let token = create_token(&user.username).map_err(|e| {
                error::ErrorInternalServerError(format!("Token generation failed: {}", e))
            })?;

            // Update last login time
            user.update_last_login();
            users_collection
                .update_one(
                    doc! { "username": &user.username },
                    doc! { "$set": { "last_login": user.last_login } },
                )
                .await
                .map_err(|e| {
                    error::ErrorInternalServerError(format!("Failed to update last login: {}", e))
                })?;

            Ok(HttpResponse::Ok().json(LoginResponse {
                token,
                username: user.username,
            }))
        }
        None => Ok(HttpResponse::Unauthorized().json(serde_json::json!({
            "error": "Invalid credentials"
        }))),
    }
}

// Add endpoint to create initial superuser
pub async fn create_superuser(app_state: web::Data<AppState>) -> Result<HttpResponse> {
    let db = &app_state.db;
    let users_collection = db.collection::<User>("users");

    // Check if any user exists already
    let count = users_collection
        .count_documents(doc! {})
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    if count > 0 {
        return Ok(HttpResponse::BadRequest().json(serde_json::json!({
            "error": "Users already exist, cannot create initial superuser"
        })));
    }

    // Get superuser credentials from environment variables
    let username = std::env::var("SUPERUSER_USERNAME")
        .map_err(|_| error::ErrorInternalServerError("SUPERUSER_USERNAME not set"))?;
    let password = std::env::var("SUPERUSER_PASSWORD")
        .map_err(|_| error::ErrorInternalServerError("SUPERUSER_PASSWORD not set"))?;

    // Hash password
    let password_hash = hash(password, DEFAULT_COST)
        .map_err(|e| error::ErrorInternalServerError(format!("Failed to hash password: {}", e)))?;

    // Create superuser with all roles
    let superuser = User::new(
        username.clone(),
        Some("admin@example.com".to_string()),
        Some("Super User".to_string()),
        password_hash,
    );

    // Insert into database
    users_collection.insert_one(&superuser).await.map_err(|e| {
        error::ErrorInternalServerError(format!("Failed to create superuser: {}", e))
    })?;

    Ok(HttpResponse::Created().json(serde_json::json!({
        "message": "Superuser created successfully",
        "username": username
    })))
}
