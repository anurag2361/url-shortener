use actix_web::{HttpResponse, Result, error, web};
use bcrypt::{DEFAULT_COST, hash};
use mongodb::bson::{doc, oid::ObjectId};
use serde::{Deserialize, Serialize};

use crate::models::role::Role;
use crate::models::user::User;
use crate::state::app_state::AppState;
use crate::utils::jwt::Claims;
use actix_web::HttpMessage;
use futures_util::TryStreamExt;

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub password: String,
    pub roles: Vec<Role>,
}

#[derive(Deserialize)]
pub struct EditUserRequest {
    pub username: Option<String>,
    pub full_name: Option<String>,
    pub password: Option<String>,
    pub roles: Option<Vec<Role>>,
    pub is_active: Option<bool>,
}

#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub roles: Vec<Role>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_login: Option<i64>,
    pub is_active: bool,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id.unwrap().to_hex(),
            username: user.username,
            email: user.email,
            full_name: user.full_name,
            roles: user.roles,
            created_at: user.created_at,
            updated_at: user.updated_at,
            last_login: user.last_login,
            is_active: user.is_active,
        }
    }
}

pub async fn get_all_users(
    app_state: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let db = &app_state.db;
    let users_collection = db.collection::<User>("users");

    // Get current user username from the request extensions
    // The middleware puts the username directly in the extensions
    let current_username = req
        .extensions()
        .get::<String>()
        .ok_or_else(|| error::ErrorInternalServerError("User not found in request"))?
        .clone();

    // Find the current user to get their ID
    let current_user = users_collection
        .find_one(doc! { "username": &current_username })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?
        .ok_or_else(|| error::ErrorInternalServerError("Current user not found"))?;

    let current_user_id = current_user
        .id
        .ok_or_else(|| error::ErrorInternalServerError("Current user ID not found"))?;

    // Find all users except the current user (SuperUser)
    let filter = doc! { "_id": { "$ne": current_user_id } };

    let users = users_collection
        .find(filter)
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?
        .try_collect::<Vec<User>>()
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?;

    let user_responses: Vec<UserResponse> = users.into_iter().map(UserResponse::from).collect();

    Ok(HttpResponse::Ok().json(user_responses))
}

pub async fn get_user(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    let object_id = ObjectId::parse_str(&user_id)
        .map_err(|_| error::ErrorBadRequest("Invalid user ID format"))?;

    let db = &app_state.db;
    let users_collection = db.collection::<User>("users");

    let user = users_collection
        .find_one(doc! { "_id": object_id })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    Ok(HttpResponse::Ok().json(UserResponse::from(user)))
}

pub async fn create_user(
    app_state: web::Data<AppState>,
    web::Json(req): web::Json<CreateUserRequest>,
) -> Result<HttpResponse> {
    let db = &app_state.db;
    let users_collection = db.collection::<User>("users");

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

    // Create new user
    let new_user = User::new(
        req.username,
        req.email,
        req.full_name,
        password_hash,
        req.roles,
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

    Ok(HttpResponse::Created().json(UserResponse::from(inserted_user)))
}

pub async fn edit_user(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
    web::Json(req): web::Json<EditUserRequest>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    let object_id = ObjectId::parse_str(&user_id)
        .map_err(|_| error::ErrorBadRequest("Invalid user ID format"))?;

    let db = &app_state.db;
    let users_collection = db.collection::<User>("users");

    // Check if user exists
    let user = users_collection
        .find_one(doc! { "_id": object_id.clone() })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?
        .ok_or_else(|| error::ErrorNotFound("User not found"))?;

    // Build update document
    let mut update_doc = doc! {
        "$set": {
            "updated_at": chrono::Utc::now().timestamp_millis(),
        }
    };

    if let Some(username) = req.username {
        update_doc
            .get_document_mut("$set")
            .unwrap()
            .insert("username", username);
    }

    if let Some(full_name) = req.full_name {
        update_doc
            .get_document_mut("$set")
            .unwrap()
            .insert("full_name", full_name);
    }

    if let Some(password) = req.password {
        let password_hash = hash(&password, DEFAULT_COST).map_err(|e| {
            error::ErrorInternalServerError(format!("Failed to hash password: {}", e))
        })?;
        update_doc
            .get_document_mut("$set")
            .unwrap()
            .insert("password_hash", password_hash);
    }

    if let Some(roles) = req.roles {
        update_doc
            .get_document_mut("$set")
            .unwrap()
            .insert("roles", mongodb::bson::to_bson(&roles).unwrap());
    }

    if let Some(is_active) = req.is_active {
        update_doc
            .get_document_mut("$set")
            .unwrap()
            .insert("is_active", is_active);
    }

    // Update user
    users_collection
        .update_one(doc! { "_id": object_id.clone() }, update_doc)
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Failed to update user: {}", e)))?;

    // Retrieve updated user
    let updated_user = users_collection
        .find_one(doc! { "_id": object_id })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?
        .ok_or_else(|| error::ErrorInternalServerError("User updated but not found"))?;

    Ok(HttpResponse::Ok().json(UserResponse::from(updated_user)))
}

pub async fn delete_user(
    app_state: web::Data<AppState>,
    path: web::Path<String>,
) -> Result<HttpResponse> {
    let user_id = path.into_inner();
    let object_id = ObjectId::parse_str(&user_id)
        .map_err(|_| error::ErrorBadRequest("Invalid user ID format"))?;

    let db = &app_state.db;
    let users_collection = db.collection::<User>("users");

    // Check if user exists
    let user_exists = users_collection
        .find_one(doc! { "_id": object_id.clone() })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Database error: {}", e)))?
        .is_some();

    if !user_exists {
        return Err(error::ErrorNotFound("User not found"));
    }

    // Delete user
    users_collection
        .delete_one(doc! { "_id": object_id })
        .await
        .map_err(|e| error::ErrorInternalServerError(format!("Failed to delete user: {}", e)))?;

    Ok(HttpResponse::NoContent().finish())
}
