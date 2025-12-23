mod auth;
mod db;
mod handlers;

use axum::{
    routing::{get, post, put},
    Router,
};
use clap::Parser;

use auth::{AppState, IdentityClient};

#[derive(Parser)]
#[command(name = "support-server")]
#[command(about = "Support ticket server for Curadesk")]
struct Args {
    /// Seed the database with test data
    #[arg(long)]
    seed: bool,

    /// SQLCipher database encryption key (or use SUPPORT_DB_KEY env var)
    #[arg(long, env = "SUPPORT_DB_KEY")]
    db_key: String,

    /// Identity service URL (or use IDENTITY_SERVICE_URL env var)
    #[arg(long, env = "IDENTITY_SERVICE_URL", default_value = "http://localhost:3001")]
    identity_url: String,

    /// Database file path
    #[arg(long, default_value = "support.db")]
    db_path: String,

    /// Port to listen on
    #[arg(long, default_value = "3000")]
    port: u16,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let db = db::init_db(&args.db_path, &args.db_key).expect("Failed to initialize database");

    if args.seed {
        db::seed_db(&db).expect("Failed to seed database");
    }

    let identity = IdentityClient::new(args.identity_url.clone());
    let state = AppState { db, identity };

    let user_routes = Router::new()
        .route("/tickets", post(handlers::user::create_ticket))
        .route("/tickets", get(handlers::user::list_tickets))
        .route("/tickets/{id}", get(handlers::user::get_ticket));

    let admin_routes = Router::new()
        .route("/admin/tickets", get(handlers::admin::list_all_tickets))
        .route("/admin/tickets/{id}", get(handlers::admin::get_ticket))
        .route("/admin/tickets/{id}/state", put(handlers::admin::update_state))
        .route("/admin/tickets/{id}/comments", post(handlers::admin::add_comment))
        .route("/admin/tickets/{id}/zip", get(handlers::admin::download_zip));

    let app = Router::new()
        .merge(user_routes)
        .merge(admin_routes)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", args.port);
    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    println!("Support server running on http://{}", addr);
    println!("Identity service: {}", args.identity_url);
    axum::serve(listener, app).await.unwrap();
}
