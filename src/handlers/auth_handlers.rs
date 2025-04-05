use actix_web::{HttpResponse, Result, error, web};
use bcrypt::{DEFAULT_COST, hash, verify};
use mongodb::bson::doc;
use serde::{Deserialize, Serialize};

use crate::models::user::User;
use crate::state::app_state::AppState;
use crate::structs::user::SignupRequest;
use crate::structs::user::UserResponse;
use crate::utils::jwt::create_token;

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub token: String,
    pub user: UserResponse,
}

pub async fn login(
    app_state: web::Data<AppState>,
    web::Json(req): web::Json<LoginRequest>,
) -> Result<HttpResponse> {
    let db = &app_state.db;
    let users_collection = db.collection::<User>("users");

    // Find user
    let user = users_collection
        .find_one(doc! { "username": &req.username })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?
        .ok_or_else(|| error::ErrorUnauthorized("Invalid username or password"))?;

    // Check if user is active
    if !user.is_active {
        return Err(error::ErrorUnauthorized("Account is disabled"));
    }

    // Verify password
    let is_valid = verify(&req.password, &user.password_hash).map_err(|e| {
        error::ErrorInternalServerError(format!("Failed to verify password: {}", e))
    })?;

    if !is_valid {
        return Err(error::ErrorUnauthorized("Invalid username or password"));
    }

    // Get user ID for the token
    let user_id = user.id.unwrap().to_hex();

    // Create JWT token
    let token = create_token(&user.username, &user_id)
        .map_err(|e| error::ErrorInternalServerError(format!("Failed to create token: {}", e)))?;

    // Update last login
    users_collection
        .update_one(
            doc! { "username": &req.username },
            doc! { "$set": { "last_login": chrono::Utc::now().timestamp_millis() } },
        )
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    let response = LoginResponse {
        token,
        user: UserResponse {
            id: user_id,
            username: user.username,
            email: user.email,
            full_name: user.full_name,
            created_at: user.created_at,
            updated_at: user.updated_at,
            last_login: user.last_login,
            is_active: user.is_active,
        },
    };

    Ok(HttpResponse::Ok().json(response))
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

pub async fn signup(
    app_state: web::Data<AppState>,
    web::Json(req): web::Json<SignupRequest>,
) -> Result<HttpResponse> {
    let db = &app_state.db;
    let users_collection = db.collection::<User>("users");

    // Check if signup is allowed (configuration option)
    let allow_signup = std::env::var("ALLOW_PUBLIC_SIGNUP")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase()
        == "true";

    if !allow_signup {
        return Err(error::ErrorForbidden("Public signup is disabled"));
    }

    // Check if username already exists
    let existing_user = users_collection
        .find_one(doc! { "username": &req.username })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    if existing_user.is_some() {
        return Err(error::ErrorBadRequest("Username already exists"));
    }

    // Hash password
    let password_hash = hash(&req.password, DEFAULT_COST)
        .map_err(|e| error::ErrorInternalServerError(format!("Failed to hash password: {}", e)))?;

    // Create new user with default permissions
    let new_user = User::new(
        req.username.clone(),
        req.email,
        req.full_name,
        password_hash,
    );

    // Insert into database
    let result = users_collection
        .insert_one(&new_user)
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Failed to create user: {}", e)))?;

    let id = result.inserted_id.as_object_id().unwrap();

    // Retrieve the inserted user
    let inserted_user = users_collection
        .find_one(doc! { "_id": id })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?
        .ok_or_else(|| error::ErrorInternalServerError("User created but not found"))?;

    // Create JWT token for the new user
    let user_id = inserted_user.id.unwrap().to_hex();
    let token = create_token(&req.username, &user_id)
        .map_err(|e| error::ErrorInternalServerError(format!("Failed to create token: {}", e)))?;

    // Return the new user details and token
    let response = LoginResponse {
        token,
        user: UserResponse::from(inserted_user),
    };

    Ok(HttpResponse::Created().json(response))
}
