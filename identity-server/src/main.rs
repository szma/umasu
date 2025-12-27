mod cli;
mod crypto;
mod db;
mod email;
mod handlers;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::{Router, routing::post};
use clap::{Parser, Subcommand};
use tower_governor::GovernorLayer;
use tower_governor::governor::GovernorConfigBuilder;

use db::DbPool;
use email::EmailService;

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub email: Option<Arc<EmailService>>,
}

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

    /// Resend API key for sending emails (or use RESEND_API_KEY env var)
    #[arg(long, env = "RESEND_API_KEY")]
    resend_api_key: Option<String>,

    /// Email sender address
    #[arg(long, default_value = "CuraDesk <kontakt@curadesk.de>")]
    email_from: String,

    /// Custom email template file path (optional, defaults to embedded template)
    #[arg(long)]
    email_template: Option<String>,

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
    /// Create an activation code for a user
    CreateActivationCode {
        #[arg(long)]
        user_id: i64,
    },
    /// List all activation codes
    ListActivationCodes,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let db = db::init_db(&args.db_path, &args.db_key).expect("Failed to initialize database");

    match args.command {
        Some(Command::Serve { port }) => {
            // Load email template
            let template = match &args.email_template {
                Some(path) => std::fs::read_to_string(path).expect("Failed to read email template"),
                None => include_str!("templates/activation_email.html").to_string(),
            };

            // Create email service if API key is provided
            let email_service = args
                .resend_api_key
                .as_ref()
                .map(|key| Arc::new(EmailService::new(key, args.email_from.clone(), template)));

            let state = AppState {
                db,
                email: email_service,
            };

            // Rate limiting: 5 burst, replenish 1 per second
            let governor_conf = GovernorConfigBuilder::default()
                .per_second(1)
                .burst_size(5)
                .finish()
                .unwrap();

            let app = Router::new()
                .route("/validate", post(handlers::validate))
                .route("/activate", post(handlers::activate))
                .route("/register", post(handlers::register))
                .layer(GovernorLayer {
                    config: Arc::new(governor_conf),
                })
                .with_state(state);

            let addr = format!("0.0.0.0:{}", port);
            println!("Identity server running on http://{}", addr);
            let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
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
        Some(Command::CreateActivationCode { user_id }) => {
            cli::create_activation_code(&db, user_id).expect("Failed to create activation code");
        }
        Some(Command::ListActivationCodes) => {
            cli::list_activation_codes(&db).expect("Failed to list activation codes");
        }
        None => {
            // Default to serve on port 3001
            let template = match &args.email_template {
                Some(path) => std::fs::read_to_string(path).expect("Failed to read email template"),
                None => include_str!("templates/activation_email.html").to_string(),
            };

            let email_service = args
                .resend_api_key
                .as_ref()
                .map(|key| Arc::new(EmailService::new(key, args.email_from.clone(), template)));

            let state = AppState {
                db,
                email: email_service,
            };

            let governor_conf = GovernorConfigBuilder::default()
                .per_second(1)
                .burst_size(5)
                .finish()
                .unwrap();

            let app = Router::new()
                .route("/validate", post(handlers::validate))
                .route("/activate", post(handlers::activate))
                .route("/register", post(handlers::register))
                .layer(GovernorLayer {
                    config: Arc::new(governor_conf),
                })
                .with_state(state);

            println!("Identity server running on http://0.0.0.0:3001");
            let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
            axum::serve(
                listener,
                app.into_make_service_with_connect_info::<SocketAddr>(),
            )
            .await
            .unwrap();
        }
    }
}
