use crate::models::role::Role;
use crate::models::user::User;
use serde::{Deserialize, Serialize};

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
