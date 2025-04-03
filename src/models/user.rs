use chrono::{DateTime, Utc};
use mongodb::bson::oid::ObjectId;
use serde::{Deserialize, Serialize};

use crate::models::role::Role;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    #[serde(rename = "_id", skip_serializing_if = "Option::is_none")]
    pub id: Option<ObjectId>,
    pub username: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_name: Option<String>,
    pub password_hash: String,
    pub roles: Vec<Role>,
    pub created_at: i64,
    pub updated_at: i64,
    pub last_login: Option<i64>,
    pub is_active: bool,
}

impl User {
    pub fn new(
        username: String,
        email: Option<String>,
        full_name: Option<String>,
        password_hash: String,
        roles: Vec<Role>,
    ) -> Self {
        let now = chrono::Utc::now().timestamp_millis();

        Self {
            id: None,
            username,
            email,
            full_name,
            password_hash,
            roles,
            created_at: now,
            updated_at: now,
            last_login: None,
            is_active: true,
        }
    }

    pub fn is_superuser(&self) -> bool {
        self.roles.iter().any(|role| role.is_superuser())
    }

    pub fn has_role(&self, role: &Role) -> bool {
        self.is_superuser() || self.roles.contains(role)
    }

    pub fn has_any_role(&self, roles: &[Role]) -> bool {
        self.is_superuser() || self.roles.iter().any(|role| roles.contains(role))
    }

    pub fn update_last_login(&mut self) {
        self.last_login = Some(chrono::Utc::now().timestamp_millis());
    }
}

// For API responses - stripped of sensitive data
#[derive(Serialize)]
pub struct UserResponse {
    pub id: String,
    pub username: String,
    pub email: Option<String>,
    pub full_name: Option<String>,
    pub roles: Vec<Role>,
    pub created_at: i64,
    pub last_login: Option<i64>,
    pub is_active: bool,
}

impl From<User> for UserResponse {
    fn from(user: User) -> Self {
        Self {
            id: user.id.map(|id| id.to_hex()).unwrap_or_default(),
            username: user.username,
            email: user.email,
            full_name: user.full_name,
            roles: user.roles,
            created_at: user.created_at,
            last_login: user.last_login,
            is_active: user.is_active,
        }
    }
}
