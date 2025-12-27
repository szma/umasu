use axum::{Json, extract::State, http::StatusCode};
use serde::{Deserialize, Serialize};

use std::time::{SystemTime, UNIX_EPOCH};

use crate::AppState;
use crate::crypto::{generate_activation_code, generate_key, hash_key};

#[derive(Deserialize)]
pub struct ValidateRequest {
    pub api_key: String,
}

#[derive(Serialize)]
pub struct ValidateResponse {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<UserInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct UserInfo {
    pub id: i64,
    pub email: String,
    pub role: String,
    pub subscription_status: String,
}

pub async fn validate(
    State(state): State<AppState>,
    Json(req): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, (StatusCode, String)> {
    let key_hash = hash_key(&req.api_key);

    let conn = state.db.lock().unwrap();

    let result = conn.query_row(
        "SELECT u.id, u.email, u.role, u.subscription_status
         FROM api_keys k
         JOIN users u ON k.user_id = u.id
         WHERE k.key_hash = ? AND k.revoked_at IS NULL",
        [&key_hash],
        |row| {
            Ok(UserInfo {
                id: row.get(0)?,
                email: row.get(1)?,
                role: row.get(2)?,
                subscription_status: row.get(3)?,
            })
        },
    );

    match result {
        Ok(user) => Ok(Json(ValidateResponse {
            valid: true,
            user: Some(user),
            error: None,
        })),
        Err(_) => Ok(Json(ValidateResponse {
            valid: false,
            user: None,
            error: Some("Invalid or revoked API key".into()),
        })),
    }
}

// --- Activation endpoint ---

#[derive(Deserialize)]
pub struct ActivateRequest {
    pub activation_code: String,
}

#[derive(Serialize)]
pub struct ActivateResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

fn now_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Exchange an activation code for an API key.
/// The activation code is marked as used and cannot be reused.
pub async fn activate(
    State(state): State<AppState>,
    Json(req): Json<ActivateRequest>,
) -> Result<Json<ActivateResponse>, (StatusCode, String)> {
    let code_hash = hash_key(&req.activation_code);
    let now = now_timestamp();

    let conn = state.db.lock().unwrap();

    // Find unused activation code and get user_id
    let result = conn.query_row(
        "SELECT id, user_id FROM activation_codes WHERE code_hash = ? AND used_at IS NULL",
        [&code_hash],
        |row| Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?)),
    );

    let (code_id, user_id) = match result {
        Ok(data) => data,
        Err(_) => {
            return Ok(Json(ActivateResponse {
                success: false,
                api_key: None,
                error: Some("Invalid or already used activation code".into()),
            }));
        }
    };

    // Mark activation code as used
    if let Err(e) = conn.execute(
        "UPDATE activation_codes SET used_at = ? WHERE id = ?",
        rusqlite::params![now, code_id],
    ) {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    }

    // Generate new API key for the user
    let key = generate_key();

    if let Err(e) = conn.execute(
        "INSERT INTO api_keys (key_hash, key_prefix, user_id, created_at) VALUES (?, ?, ?, ?)",
        rusqlite::params![key.hash, key.prefix, user_id, now],
    ) {
        return Err((StatusCode::INTERNAL_SERVER_ERROR, e.to_string()));
    }

    Ok(Json(ActivateResponse {
        success: true,
        api_key: Some(key.full_key),
        error: None,
    }))
}

// --- Registration endpoint ---

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
}

#[derive(Serialize)]
pub struct RegisterResponse {
    pub success: bool,
    pub message: String,
}

/// Basic email format validation
fn is_valid_email(email: &str) -> bool {
    let email = email.trim();
    if email.len() > 254 || email.is_empty() {
        return false;
    }
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return false;
    }
    let (local, domain) = (parts[0], parts[1]);
    !local.is_empty() && !domain.is_empty() && domain.contains('.')
}

/// Register a new user or resend activation code for existing user.
/// Always returns success to prevent email enumeration.
pub async fn register(
    State(state): State<AppState>,
    Json(req): Json<RegisterRequest>,
) -> Result<Json<RegisterResponse>, (StatusCode, String)> {
    let generic_response = RegisterResponse {
        success: true,
        message: "If this email is valid, you will receive an activation code shortly".into(),
    };

    // Check if email service is configured
    let email_service = match &state.email {
        Some(svc) => svc.clone(),
        None => {
            eprintln!("Email service not configured");
            return Err((
                StatusCode::SERVICE_UNAVAILABLE,
                "Email service not configured".into(),
            ));
        }
    };

    // Validate email format
    let email = req.email.trim().to_lowercase();
    if !is_valid_email(&email) {
        // Return generic response to prevent enumeration
        return Ok(Json(generic_response));
    }

    let now = now_timestamp();

    // Find or create user
    let user_id: i64 = {
        let conn = state.db.lock().unwrap();

        match conn.query_row("SELECT id FROM users WHERE email = ?", [&email], |row| {
            row.get(0)
        }) {
            Ok(id) => {
                // User exists - invalidate all unused activation codes
                if let Err(e) = conn.execute(
                    "UPDATE activation_codes SET used_at = ? WHERE user_id = ? AND used_at IS NULL",
                    rusqlite::params![now, id],
                ) {
                    eprintln!("Failed to invalidate old codes: {}", e);
                }
                id
            }
            Err(_) => {
                // Create new user with role=customer, subscription_status=trial
                if let Err(e) = conn.execute(
                    "INSERT INTO users (email, role, subscription_status, created_at) VALUES (?, 'customer', 'trial', ?)",
                    rusqlite::params![email, now],
                ) {
                    eprintln!("Failed to create user: {}", e);
                    return Ok(Json(generic_response));
                }
                conn.last_insert_rowid()
            }
        }
    };

    // Generate new activation code
    let code = generate_activation_code();

    {
        let conn = state.db.lock().unwrap();
        if let Err(e) = conn.execute(
            "INSERT INTO activation_codes (code_hash, code_prefix, user_id, created_at) VALUES (?, ?, ?, ?)",
            rusqlite::params![code.hash, code.prefix, user_id, now],
        ) {
            eprintln!("Failed to create activation code: {}", e);
            return Ok(Json(generic_response));
        }
    }

    // Send email (async)
    if let Err(e) = email_service
        .send_activation_code(&email, &code.full_code)
        .await
    {
        eprintln!("Failed to send email to {}: {}", email, e);
        // Still return success to prevent enumeration
    }

    Ok(Json(generic_response))
}
