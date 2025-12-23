use std::io::Cursor;

use anyhow::Result;
use support_common::{Ticket, TicketDetail, TicketState};
use zip::ZipArchive;

use crate::api::ApiClient;

#[derive(Debug, Clone, PartialEq)]
pub enum View {
    TicketList,
    TicketDetail,
    ZipViewer,
    FileContent,
    AddComment,
}

#[derive(Debug, Clone)]
pub struct ZipEntry {
    pub name: String,
    pub size: u64,
    pub is_dir: bool,
}

pub struct App {
    pub api: ApiClient,
    pub running: bool,
    pub view: View,

    // Ticket Liste
    pub tickets: Vec<Ticket>,
    pub selected_ticket: usize,

    // Ticket Detail
    pub current_ticket: Option<TicketDetail>,
    pub detail_scroll: usize,

    // ZIP Viewer
    pub zip_data: Option<Vec<u8>>,
    pub zip_entries: Vec<ZipEntry>,
    pub selected_zip_entry: usize,

    // File Content Viewer
    pub file_content: Option<String>,
    pub file_name: String,
    pub content_scroll: usize,

    // Comment Input
    pub comment_input: String,

    // Status/Error Message
    pub status_message: Option<String>,
}

impl App {
    pub fn new(api: ApiClient) -> Self {
        Self {
            api,
            running: true,
            view: View::TicketList,
            tickets: Vec::new(),
            selected_ticket: 0,
            current_ticket: None,
            detail_scroll: 0,
            zip_data: None,
            zip_entries: Vec::new(),
            selected_zip_entry: 0,
            file_content: None,
            file_name: String::new(),
            content_scroll: 0,
            comment_input: String::new(),
            status_message: None,
        }
    }

    pub fn load_tickets(&mut self) -> Result<()> {
        self.tickets = self.api.list_tickets()?;
        self.selected_ticket = 0;
        Ok(())
    }

    pub fn load_ticket_detail(&mut self, id: i64) -> Result<()> {
        self.current_ticket = Some(self.api.get_ticket(id)?);
        self.detail_scroll = 0;
        self.view = View::TicketDetail;
        Ok(())
    }

    pub fn load_zip(&mut self) -> Result<()> {
        if let Some(detail) = &self.current_ticket {
            let data = self.api.download_zip(detail.ticket.id)?;
            self.zip_entries = Self::parse_zip_entries(&data)?;
            self.zip_data = Some(data);
            self.selected_zip_entry = 0;
            self.view = View::ZipViewer;
        }
        Ok(())
    }

    fn parse_zip_entries(data: &[u8]) -> Result<Vec<ZipEntry>> {
        let cursor = Cursor::new(data);
        let mut archive = ZipArchive::new(cursor)?;
        let mut entries: Vec<ZipEntry> = (0..archive.len())
            .filter_map(|i| {
                let file = archive.by_index(i).ok()?;
                Some(ZipEntry {
                    name: file.name().to_string(),
                    size: file.size(),
                    is_dir: file.is_dir(),
                })
            })
            .collect();
        entries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(entries)
    }

    pub fn open_zip_file(&mut self) -> Result<()> {
        if let Some(data) = &self.zip_data {
            if let Some(entry) = self.zip_entries.get(self.selected_zip_entry) {
                if entry.is_dir {
                    return Ok(());
                }

                let cursor = Cursor::new(data);
                let mut archive = ZipArchive::new(cursor)?;
                let mut file = archive.by_name(&entry.name)?;

                let mut content = Vec::new();
                std::io::Read::read_to_end(&mut file, &mut content)?;

                // Versuche als UTF-8 zu parsen
                self.file_content = Some(
                    String::from_utf8(content.clone())
                        .unwrap_or_else(|_| format!("[Binärdatei: {} Bytes]", content.len())),
                );
                self.file_name = entry.name.clone();
                self.content_scroll = 0;
                self.view = View::FileContent;
            }
        }
        Ok(())
    }

    pub fn update_ticket_state(&mut self, state: TicketState) -> Result<()> {
        let Some(ticket_id) = self.current_ticket.as_ref().map(|t| t.ticket.id) else {
            return Ok(());
        };

        self.api.update_state(ticket_id, state.clone())?;
        // Reload ticket detail
        self.load_ticket_detail(ticket_id)?;
        // Update in list too
        if let Some(t) = self.tickets.iter_mut().find(|t| t.id == ticket_id) {
            t.state = state;
        }
        self.status_message = Some("Status aktualisiert".to_string());
        Ok(())
    }

    pub fn submit_comment(&mut self) -> Result<()> {
        if let Some(detail) = &self.current_ticket {
            if !self.comment_input.trim().is_empty() {
                self.api
                    .add_comment(detail.ticket.id, self.comment_input.clone())?;
                self.comment_input.clear();
                // Reload ticket detail
                self.load_ticket_detail(detail.ticket.id)?;
                self.status_message = Some("Kommentar hinzugefügt".to_string());
            }
        }
        self.view = View::TicketDetail;
        Ok(())
    }

    pub fn move_selection(&mut self, delta: i32) {
        match self.view {
            View::TicketList => {
                let len = self.tickets.len();
                if len > 0 {
                    self.selected_ticket =
                        ((self.selected_ticket as i32 + delta).rem_euclid(len as i32)) as usize;
                }
            }
            View::ZipViewer => {
                let len = self.zip_entries.len();
                if len > 0 {
                    self.selected_zip_entry =
                        ((self.selected_zip_entry as i32 + delta).rem_euclid(len as i32)) as usize;
                }
            }
            View::TicketDetail => {
                self.detail_scroll = (self.detail_scroll as i32 + delta).max(0) as usize;
            }
            View::FileContent => {
                self.content_scroll = (self.content_scroll as i32 + delta).max(0) as usize;
            }
            _ => {}
        }
    }

    pub fn go_back(&mut self) {
        match self.view {
            View::TicketDetail => {
                self.view = View::TicketList;
                self.current_ticket = None;
            }
            View::ZipViewer => {
                self.view = View::TicketDetail;
                self.zip_data = None;
                self.zip_entries.clear();
            }
            View::FileContent => {
                self.view = View::ZipViewer;
                self.file_content = None;
            }
            View::AddComment => {
                self.view = View::TicketDetail;
                self.comment_input.clear();
            }
            _ => {}
        }
    }
}
