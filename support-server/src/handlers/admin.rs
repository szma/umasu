use axum::{
    Json,
    body::Body,
    extract::{Path, State},
    http::{StatusCode, header},
    response::Response,
};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::auth::{AdminContext, AppState};
use support_common::{
    Comment, CreateCommentRequest, Ticket, TicketDetail, TicketState, UpdateStateRequest,
};

pub async fn list_all_tickets(
    State(state): State<AppState>,
    _admin: AdminContext,
) -> Result<Json<Vec<Ticket>>, (StatusCode, String)> {
    let conn = state.db.lock().unwrap();
    let mut stmt = conn
        .prepare("SELECT id, user_id, created_at, description, zip_filename, state FROM tickets ORDER BY created_at DESC")
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let tickets = stmt
        .query_map([], |row| {
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
    _admin: AdminContext,
    Path(ticket_id): Path<i64>,
) -> Result<Json<TicketDetail>, (StatusCode, String)> {
    let conn = state.db.lock().unwrap();

    let ticket: Ticket = conn
        .query_row(
            "SELECT id, user_id, created_at, description, zip_filename, state FROM tickets WHERE id = ?",
            [ticket_id],
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

pub async fn update_state(
    State(state): State<AppState>,
    _admin: AdminContext,
    Path(ticket_id): Path<i64>,
    Json(req): Json<UpdateStateRequest>,
) -> Result<StatusCode, (StatusCode, String)> {
    let conn = state.db.lock().unwrap();

    let rows = conn
        .execute(
            "UPDATE tickets SET state = ? WHERE id = ?",
            rusqlite::params![req.state.as_str(), ticket_id],
        )
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    if rows == 0 {
        Err((StatusCode::NOT_FOUND, "Ticket not found".into()))
    } else {
        Ok(StatusCode::NO_CONTENT)
    }
}

pub async fn add_comment(
    State(state): State<AppState>,
    admin: AdminContext,
    Path(ticket_id): Path<i64>,
    Json(req): Json<CreateCommentRequest>,
) -> Result<Json<Comment>, (StatusCode, String)> {
    let conn = state.db.lock().unwrap();

    // Check ticket exists
    let exists: bool = conn
        .query_row("SELECT 1 FROM tickets WHERE id = ?", [ticket_id], |_| {
            Ok(true)
        })
        .unwrap_or(false);

    if !exists {
        return Err((StatusCode::NOT_FOUND, "Ticket not found".into()));
    }

    let created_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    conn.execute(
        "INSERT INTO comments (ticket_id, user_id, created_at, text) VALUES (?, ?, ?, ?)",
        rusqlite::params![ticket_id, admin.user_id, created_at, req.text],
    )
    .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let id = conn.last_insert_rowid();

    Ok(Json(Comment {
        id,
        ticket_id,
        user_id: admin.user_id,
        created_at,
        text: req.text,
    }))
}

pub async fn download_zip(
    State(state): State<AppState>,
    _admin: AdminContext,
    Path(ticket_id): Path<i64>,
) -> Result<Response, (StatusCode, String)> {
    let conn = state.db.lock().unwrap();

    let (zip_data, zip_filename): (Vec<u8>, String) = conn
        .query_row(
            "SELECT zip_data, zip_filename FROM tickets WHERE id = ?",
            [ticket_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .map_err(|_| (StatusCode::NOT_FOUND, "Ticket not found".into()))?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", zip_filename),
        )
        .body(Body::from(zip_data))
        .unwrap();

    Ok(response)
}
