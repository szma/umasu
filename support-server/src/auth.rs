use axum::{
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
};
use serde::{Deserialize, Serialize};

use crate::db::DbPool;

#[derive(Clone)]
pub struct IdentityClient {
    client: reqwest::Client,
    base_url: String,
}

impl IdentityClient {
    pub fn new(base_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
        }
    }

    pub async fn validate(&self, api_key: &str) -> Result<Option<UserInfo>, reqwest::Error> {
        let response = self
            .client
            .post(format!("{}/validate", self.base_url))
            .json(&ValidateRequest {
                api_key: api_key.to_string(),
            })
            .send()
            .await?;

        let validation: ValidateResponse = response.json().await?;

        if validation.valid {
            Ok(validation.user)
        } else {
            Ok(None)
        }
    }
}

#[derive(Serialize)]
struct ValidateRequest {
    api_key: String,
}

#[derive(Deserialize)]
struct ValidateResponse {
    valid: bool,
    user: Option<UserInfo>,
    #[allow(dead_code)]
    error: Option<String>,
}

#[derive(Deserialize, Clone)]
pub struct UserInfo {
    pub id: i64,
    pub email: String,
    pub role: String,
    pub subscription_status: String,
}

#[derive(Clone)]
pub struct AppState {
    pub db: DbPool,
    pub identity: IdentityClient,
}

#[derive(Debug, Clone)]
pub struct UserContext {
    pub user_id: i64,
    pub email: String,
    pub role: String,
    pub is_admin: bool,
}

#[derive(Debug, Clone)]
pub struct AdminContext {
    pub user_id: i64,
    pub email: String,
}

impl<S> FromRequestParts<S> for UserContext
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let api_key = parts
            .headers
            .get("x-api-key")
            .and_then(|v| v.to_str().ok())
            .ok_or((StatusCode::UNAUTHORIZED, "Missing X-API-Key header"))?;

        let app_state = AppState::from_ref(state);

        let user = app_state
            .identity
            .validate(api_key)
            .await
            .map_err(|_| (StatusCode::SERVICE_UNAVAILABLE, "Identity service unavailable"))?
            .ok_or((StatusCode::UNAUTHORIZED, "Invalid API key"))?;

        Ok(UserContext {
            user_id: user.id,
            email: user.email,
            role: user.role.clone(),
            is_admin: user.role == "admin",
        })
    }
}

impl<S> FromRequestParts<S> for AdminContext
where
    S: Send + Sync,
    AppState: FromRef<S>,
{
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let user = UserContext::from_request_parts(parts, state).await?;

        if user.is_admin {
            Ok(AdminContext {
                user_id: user.user_id,
                email: user.email,
            })
        } else {
            Err((StatusCode::FORBIDDEN, "Admin access required"))
        }
    }
}

pub trait FromRef<T> {
    fn from_ref(input: &T) -> Self;
}

impl FromRef<AppState> for AppState {
    fn from_ref(input: &AppState) -> Self {
        input.clone()
    }
}

impl FromRef<AppState> for DbPool {
    fn from_ref(input: &AppState) -> Self {
        input.db.clone()
    }
}
