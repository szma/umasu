use axum::{extract::State, http::StatusCode, Json};
use serde::{Deserialize, Serialize};

use crate::crypto::hash_key;
use crate::db::DbPool;

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
    State(db): State<DbPool>,
    Json(req): Json<ValidateRequest>,
) -> Result<Json<ValidateResponse>, (StatusCode, String)> {
    let key_hash = hash_key(&req.api_key);

    let conn = db.lock().unwrap();

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
