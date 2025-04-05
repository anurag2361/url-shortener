mod db;
mod handlers;
mod middlewares;
mod models;
mod routes;
mod state;
mod structs;
mod utils;

use crate::state::app_state::AppState;
use actix_cors::Cors;
use actix_web::{App, HttpServer, http, middleware::Logger, web};
use db::mongodb::get_database;
use dotenv::dotenv;
use env_logger::Env;
use routes::init_routes;
use std::env;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    let port_string = env::var("PORT").expect("PORT not set.");
    let port = port_string.parse::<u16>().unwrap();
    env_logger::init_from_env(Env::default().default_filter_or("info"));

    // Initialize the database connection
    let db = match get_database().await {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Error connecting to the database: {}", e);
            std::process::exit(1);
        }
    };

    // Create shared state
    let app_state = web::Data::new(AppState { db });

    // Start the Actix Web server
    HttpServer::new(move || {
        // Create a logger with a custom format instead
        let logger = Logger::new("%a \"%r\" %s %b \"%{Referer}i\" \"%{User-Agent}i\" %D ms");
        // Enable CORS for all origins
        let cors = Cors::default()
            .allowed_origin("http://localhost:5173") // Default Bun dev server port
            .allowed_origin("http://localhost:4173")
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![http::header::AUTHORIZATION, http::header::ACCEPT])
            .allowed_header(http::header::CONTENT_TYPE)
            .max_age(3600);
        App::new()
            .wrap(logger)
            .wrap(cors)
            .app_data(app_state.clone())
            .configure(init_routes)
    })
    .bind(("127.0.0.1", port))?
    .run()
    .await
}
