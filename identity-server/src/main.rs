mod cli;
mod crypto;
mod db;
mod handlers;

use axum::{routing::post, Router};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "identity-server")]
#[command(about = "Identity and API key management for Curadesk")]
struct Args {
    /// SQLCipher database encryption key (or use IDENTITY_DB_KEY env var)
    #[arg(long, env = "IDENTITY_DB_KEY")]
    db_key: String,

    /// Database file path
    #[arg(long, default_value = "identity.db")]
    db_path: String,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Start the HTTP server
    Serve {
        #[arg(long, default_value = "3001")]
        port: u16,
    },
    /// Create a new user
    CreateUser {
        #[arg(long)]
        email: String,
        #[arg(long, value_parser = ["admin", "support", "customer"])]
        role: String,
    },
    /// Create an API key for a user
    CreateKey {
        #[arg(long)]
        user_id: i64,
    },
    /// Revoke an API key by prefix
    RevokeKey {
        #[arg(long)]
        prefix: String,
    },
    /// List all users
    ListUsers,
    /// List all API keys
    ListKeys,
    /// Seed development data
    Seed,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let db = db::init_db(&args.db_path, &args.db_key).expect("Failed to initialize database");

    match args.command {
        Some(Command::Serve { port }) => {
            let app = Router::new()
                .route("/validate", post(handlers::validate))
                .with_state(db);

            let addr = format!("0.0.0.0:{}", port);
            println!("Identity server running on http://{}", addr);
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        }
        Some(Command::CreateUser { email, role }) => {
            cli::create_user(&db, &email, &role).expect("Failed to create user");
        }
        Some(Command::CreateKey { user_id }) => {
            cli::create_key(&db, user_id).expect("Failed to create key");
        }
        Some(Command::RevokeKey { prefix }) => {
            cli::revoke_key(&db, &prefix).expect("Failed to revoke key");
        }
        Some(Command::ListUsers) => {
            cli::list_users(&db).expect("Failed to list users");
        }
        Some(Command::ListKeys) => {
            cli::list_keys(&db).expect("Failed to list keys");
        }
        Some(Command::Seed) => {
            cli::seed_dev_data(&db).expect("Failed to seed data");
        }
        None => {
            // Default to serve on port 3001
            let app = Router::new()
                .route("/validate", post(handlers::validate))
                .with_state(db);

            println!("Identity server running on http://0.0.0.0:3001");
            let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
            axum::serve(listener, app).await.unwrap();
        }
    }
}
