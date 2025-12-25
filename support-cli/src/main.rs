mod api;
mod app;
mod ui;

use std::io;
use std::time::Duration;

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use support_common::TicketState;

use api::ApiClient;
use app::{App, View};

fn main() -> Result<()> {
    // Config laden
    let api_key =
        std::env::var("SUPPORT_API_KEY").context("SUPPORT_API_KEY Umgebungsvariable fehlt")?;
    let base_url =
        std::env::var("SUPPORT_URL").unwrap_or_else(|_| "http://localhost:3000".to_string());

    let api = ApiClient::new(base_url, api_key);
    let mut app = App::new(api);

    // Initial laden
    app.load_tickets()?;

    // Terminal setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Main loop
    let result = run_app(&mut terminal, &mut app);

    // Cleanup
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, app: &mut App) -> Result<()> {
    while app.running {
        terminal.draw(|f| ui::draw(f, app))?;

        if event::poll(Duration::from_millis(250))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                // Clear status message on any key
                app.status_message = None;

                match app.view {
                    View::TicketList => handle_ticket_list_keys(app, key.code)?,
                    View::TicketDetail => handle_ticket_detail_keys(app, key.code)?,
                    View::ZipViewer => handle_zip_viewer_keys(app, key.code)?,
                    View::FileContent => handle_file_content_keys(app, key.code),
                    View::AddComment => handle_add_comment_keys(app, key.code)?,
                    View::CreateTicket => handle_create_ticket_keys(app, key.code)?,
                }
            }
        }
    }
    Ok(())
}

fn handle_ticket_list_keys(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Char('q') => app.running = false,
        KeyCode::Char('r') => {
            app.load_tickets()?;
            app.status_message = Some("Tickets refreshed".to_string());
        }
        KeyCode::Char('n') => {
            app.view = View::CreateTicket;
        }
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Enter => {
            if let Some(ticket) = app.tickets.get(app.selected_ticket) {
                app.load_ticket_detail(ticket.id)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_ticket_detail_keys(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Esc => app.go_back(),
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Char('z') => {
            app.load_zip()?;
        }
        KeyCode::Char('c') => {
            app.view = View::AddComment;
        }
        KeyCode::Char('1') => {
            app.update_ticket_state(TicketState::New)?;
        }
        KeyCode::Char('2') => {
            app.update_ticket_state(TicketState::InProgress)?;
        }
        KeyCode::Char('3') => {
            app.update_ticket_state(TicketState::Done)?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_zip_viewer_keys(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Esc => app.go_back(),
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::Enter => {
            app.open_zip_file()?;
        }
        _ => {}
    }
    Ok(())
}

fn handle_file_content_keys(app: &mut App, code: KeyCode) {
    match code {
        KeyCode::Esc => app.go_back(),
        KeyCode::Up | KeyCode::Char('k') => app.move_selection(-1),
        KeyCode::Down | KeyCode::Char('j') => app.move_selection(1),
        KeyCode::PageUp => app.move_selection(-20),
        KeyCode::PageDown => app.move_selection(20),
        _ => {}
    }
}

fn handle_add_comment_keys(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Esc => app.go_back(),
        KeyCode::Enter => {
            app.submit_comment()?;
        }
        KeyCode::Backspace => {
            app.comment_input.pop();
        }
        KeyCode::Char(c) => {
            app.comment_input.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_create_ticket_keys(app: &mut App, code: KeyCode) -> Result<()> {
    match code {
        KeyCode::Esc => app.go_back(),
        KeyCode::Enter => {
            app.submit_new_ticket()?;
        }
        KeyCode::Backspace => {
            app.new_ticket_description.pop();
        }
        KeyCode::Char(c) => {
            app.new_ticket_description.push(c);
        }
        _ => {}
    }
    Ok(())
}
