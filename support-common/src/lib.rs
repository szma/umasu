use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TicketState {
    New,
    InProgress,
    Done,
}

impl TicketState {
    pub fn as_str(&self) -> &'static str {
        match self {
            TicketState::New => "new",
            TicketState::InProgress => "in_progress",
            TicketState::Done => "done",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "new" => Some(TicketState::New),
            "in_progress" => Some(TicketState::InProgress),
            "done" => Some(TicketState::Done),
            _ => None,
        }
    }
}

impl std::fmt::Display for TicketState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TicketState::New => write!(f, "Neu"),
            TicketState::InProgress => write!(f, "In Bearbeitung"),
            TicketState::Done => write!(f, "Erledigt"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ticket {
    pub id: i64,
    pub user_id: i64,
    pub created_at: i64,
    pub description: String,
    pub zip_filename: String,
    pub state: TicketState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketWithZip {
    pub id: i64,
    pub user_id: i64,
    pub created_at: i64,
    pub description: String,
    pub zip_filename: String,
    pub zip_data: Vec<u8>,
    pub state: TicketState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Comment {
    pub id: i64,
    pub ticket_id: i64,
    pub user_id: i64,
    pub created_at: i64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TicketDetail {
    #[serde(flatten)]
    pub ticket: Ticket,
    pub comments: Vec<Comment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateCommentRequest {
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateStateRequest {
    pub state: TicketState,
}
