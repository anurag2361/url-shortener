use actix_web::{HttpResponse, web};
use mongodb::bson::doc;

use crate::state::app_state::AppState;

pub async fn health_check(state: web::Data<AppState>) -> HttpResponse {
    // Perform a simple ping operation to check the database connection
    let ping_result = state.db.run_command(doc! { "ping": 1 }).await;

    match ping_result {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({ "success": true })),
        Err(_) => HttpResponse::InternalServerError()
            .json(serde_json::json!({ "success": false, "error": "Database connection failed" })),
    }
}
