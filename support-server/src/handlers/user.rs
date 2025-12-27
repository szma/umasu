use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use axum_extra::extract::Multipart;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::auth::{AppState, UserContext};
use support_common::{Comment, Ticket, TicketDetail, TicketState};

pub async fn create_ticket(
    State(state): State<AppState>,
    user: UserContext,
    mut multipart: Multipart,
) -> Result<Json<Ticket>, (StatusCode, String)> {
    let db = &state.db;
    let mut description: Option<String> = None;
    let mut zip_data: Option<Vec<u8>> = None;
    let mut zip_filename: Option<String> = None;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        match name.as_str() {
            "description" => {
                description = Some(
                    field
                        .text()
                        .await
                        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?,
                );
            }
            "zip" => {
                zip_filename = field.file_name().map(|s| s.to_string());
                zip_data = Some(
                    field
                        .bytes()
                        .await
                        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?
                        .to_vec(),
                );
            }
            _ => {}
        }
    }

    let description = description.ok_or((StatusCode::BAD_REQUEST, "Missing description".into()))?;
    let zip_data = zip_data.ok_or((StatusCode::BAD_REQUEST, "Missing zip file".into()))?;
    let zip_filename = zip_filename.unwrap_or_else(|| "upload.zip".to_string());

    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let conn = db.lock().unwrap();
    conn.execute(
        "INSERT INTO tickets (user_id, created_at, description, zip_data, zip_filename, state) VALUES (?, ?, ?, ?, ?, ?)",
        rusqlite::params![user.user_id, created_at, description, zip_data, zip_filename, "new"],
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let id = conn.last_insert_rowid();

    Ok(Json(Ticket {
        id,
        user_id: user.user_id,
        created_at,
        description,
        zip_filename,
        state: TicketState::New,
    }))
}

pub async fn list_tickets(
    State(state): State<AppState>,
    user: UserContext,
) -> Result<Json<Vec<Ticket>>, (StatusCode, String)> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, user_id, created_at, description, zip_filename, state FROM tickets WHERE user_id = ? ORDER BY created_at DESC")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let tickets = stmt
        .query_map([user.user_id], |row| {
            let state_str: String = row.get(5)?;
            Ok(Ticket {
                id: row.get(0)?,
                user_id: row.get(1)?,
                created_at: row.get(2)?,
                description: row.get(3)?,
                zip_filename: row.get(4)?,
                state: TicketState::from_str(&state_str).unwrap_or(TicketState::New),
            })
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(tickets))
}

pub async fn get_ticket(
    State(state): State<AppState>,
    user: UserContext,
    Path(ticket_id): Path<i64>,
) -> Result<Json<TicketDetail>, (StatusCode, String)> {
    let conn = state.db.lock().unwrap();

    let ticket: Ticket = conn
        .query_row(
            "SELECT id, user_id, created_at, description, zip_filename, state FROM tickets WHERE id = ? AND user_id = ?",
            [ticket_id, user.user_id],
            |row| {
                let state_str: String = row.get(5)?;
                Ok(Ticket {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    created_at: row.get(2)?,
                    description: row.get(3)?,
                    zip_filename: row.get(4)?,
                    state: TicketState::from_str(&state_str).unwrap_or(TicketState::New),
                })
            },
        )
        .map_err(|_| (StatusCode::NOT_FOUND, "Ticket not found".into()))?;

    let mut stmt = conn
        .prepare("SELECT id, ticket_id, user_id, created_at, text FROM comments WHERE ticket_id = ? ORDER BY created_at ASC")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let comments = stmt
        .query_map([ticket_id], |row| {
            Ok(Comment {
                id: row.get(0)?,
                ticket_id: row.get(1)?,
                user_id: row.get(2)?,
                created_at: row.get(3)?,
                text: row.get(4)?,
            })
        })
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    Ok(Json(TicketDetail { ticket, comments }))
}
