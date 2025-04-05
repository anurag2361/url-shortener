use crate::models::user::User;
use crate::state::app_state::AppState;
use crate::structs::user::{CreateUserRequest, EditUserRequest, UserResponse};
use crate::utils::jwt::Claims;
use actix_web::HttpMessage;
use actix_web::{HttpResponse, Result, error, web};
use bcrypt::{DEFAULT_COST, hash};
use futures_util::TryStreamExt;
use mongodb::bson::{doc, oid::ObjectId};

pub async fn get_all_users(
    app_state: web::Data<AppState>,
    req: actix_web::HttpRequest,
) -> Result<HttpResponse> {
    let db = &app_state.db;
    let users_collection = db.collection::<User>("users");

    // Get current user claims from the request extensions
    let extensions = req.extensions();
    let claims = extensions
        .get::<Claims>()
        .ok_or_else(|| error::ErrorInternalServerError("User claims not found in request"))?;

    // Get current user ID directly from claims
    let current_user_id = ObjectId::parse_str(&claims.user_id)
        .map_err(|_| error::ErrorInternalServerError("Invalid user ID in token"))?;

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
    let new_user = User::new(req.username, req.email, req.full_name, password_hash);

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
    let _user = users_collection
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
