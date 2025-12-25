use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
};
use support_common::TicketState;

use crate::app::{App, View};

fn format_timestamp(ts: i64) -> String {
    use std::time::{Duration, UNIX_EPOCH};
    let datetime = UNIX_EPOCH + Duration::from_secs(ts as u64);
    let secs = datetime
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Simple formatting (YYYY-MM-DD HH:MM)
    let days_since_epoch = secs / 86400;
    let remaining_secs = secs % 86400;
    let hours = remaining_secs / 3600;
    let minutes = (remaining_secs % 3600) / 60;

    // Calculate year, month, day from days since epoch
    let mut year = 1970;
    let mut days = days_since_epoch;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if days < days_in_year {
            break;
        }
        days -= days_in_year;
        year += 1;
    }

    let months_days: [u64; 12] = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    for &d in &months_days {
        if days < d {
            break;
        }
        days -= d;
        month += 1;
    }
    let day = days + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}",
        year, month, day, hours, minutes
    )
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

fn state_color(state: &TicketState) -> Color {
    match state {
        TicketState::New => Color::Yellow,
        TicketState::InProgress => Color::Cyan,
        TicketState::Done => Color::Green,
    }
}

pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(3)])
        .split(frame.area());

    match app.view {
        View::TicketList => draw_ticket_list(frame, app, chunks[0]),
        View::TicketDetail => draw_ticket_detail(frame, app, chunks[0]),
        View::ZipViewer => draw_zip_viewer(frame, app, chunks[0]),
        View::FileContent => draw_file_content(frame, app, chunks[0]),
        View::AddComment => draw_add_comment(frame, app, chunks[0]),
        View::CreateTicket => draw_create_ticket(frame, app, chunks[0]),
    }

    draw_status_bar(frame, app, chunks[1]);
}

fn draw_ticket_list(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .tickets
        .iter()
        .map(|t| {
            let state_span = Span::styled(
                format!("[{}]", t.state),
                Style::default().fg(state_color(&t.state)),
            );
            let line = Line::from(vec![
                Span::raw(format!("#{:<4} ", t.id)),
                state_span,
                Span::raw(format!(
                    " {} - {}",
                    format_timestamp(t.created_at),
                    t.description.lines().next().unwrap_or("")
                )),
            ]);
            ListItem::new(line)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Support Tickets ")
                .borders(Borders::ALL),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_ticket));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_ticket_detail(frame: &mut Frame, app: &App, area: Rect) {
    let Some(detail) = &app.current_ticket else {
        return;
    };

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(6), Constraint::Min(0)])
        .split(area);

    // Header info
    let header_text = vec![
        Line::from(vec![
            Span::styled("Ticket #", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(format!("{}", detail.ticket.id)),
            Span::raw("  "),
            Span::styled(
                format!("[{}]", detail.ticket.state),
                Style::default().fg(state_color(&detail.ticket.state)),
            ),
        ]),
        Line::from(vec![
            Span::styled("Erstellt: ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(format_timestamp(detail.ticket.created_at)),
            Span::raw(format!("  (User: {})", detail.ticket.user_id)),
        ]),
        Line::from(vec![
            Span::styled("Datei: ", Style::default().add_modifier(Modifier::DIM)),
            Span::raw(&detail.ticket.zip_filename),
        ]),
        Line::from(""),
        Line::from(detail.ticket.description.as_str()),
    ];

    let header = Paragraph::new(header_text)
        .block(Block::default().title(" Details ").borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(header, chunks[0]);

    // Comments
    let comment_items: Vec<ListItem> = detail
        .comments
        .iter()
        .map(|c| {
            let header_line = Line::from(vec![
                Span::styled(
                    format!("User {} - ", c.user_id),
                    Style::default().add_modifier(Modifier::DIM),
                ),
                Span::styled(
                    format_timestamp(c.created_at),
                    Style::default().add_modifier(Modifier::DIM),
                ),
            ]);
            let text_line = Line::from(c.text.as_str());
            ListItem::new(vec![header_line, text_line, Line::from("")])
        })
        .collect();

    let comments_title = format!(" Kommentare ({}) ", detail.comments.len());
    let comments = List::new(comment_items)
        .block(Block::default().title(comments_title).borders(Borders::ALL));

    let mut state = ListState::default();
    if !detail.comments.is_empty() {
        state.select(Some(app.detail_scroll.min(detail.comments.len() - 1)));
    }
    frame.render_stateful_widget(comments, chunks[1], &mut state);
}

fn draw_zip_viewer(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .zip_entries
        .iter()
        .map(|e| {
            let icon = if e.is_dir { "📁" } else { "📄" };
            let size_str = if e.is_dir {
                String::new()
            } else {
                format_size(e.size)
            };
            ListItem::new(Line::from(format!("{} {} {}", icon, e.name, size_str)))
        })
        .collect();

    let title = format!(" ZIP Inhalt ({} Einträge) ", app.zip_entries.len());
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    state.select(Some(app.selected_zip_entry));
    frame.render_stateful_widget(list, area, &mut state);
}

fn draw_file_content(frame: &mut Frame, app: &App, area: Rect) {
    let content = app.file_content.as_deref().unwrap_or("");
    let lines: Vec<Line> = content
        .lines()
        .skip(app.content_scroll)
        .take(area.height.saturating_sub(2) as usize)
        .map(Line::from)
        .collect();

    let title = format!(" {} ", app.file_name);
    let paragraph = Paragraph::new(lines)
        .block(Block::default().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });
    frame.render_widget(paragraph, area);
}

fn draw_add_comment(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(0)])
        .split(area);

    let input = Paragraph::new(app.comment_input.as_str())
        .block(
            Block::default()
                .title(" Enter comment (Enter = Send, Esc = Cancel) ")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::Yellow));
    frame.render_widget(input, chunks[0]);

    // Show ticket context
    if let Some(detail) = &app.current_ticket {
        let context = Paragraph::new(format!(
            "Ticket #{}: {}",
            detail.ticket.id,
            detail.ticket.description.lines().next().unwrap_or("")
        ))
        .block(Block::default().title(" Context ").borders(Borders::ALL));
        frame.render_widget(context, chunks[1]);
    }
}

fn draw_create_ticket(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(5), Constraint::Min(0)])
        .split(area);

    let input = Paragraph::new(app.new_ticket_description.as_str())
        .block(
            Block::default()
                .title(" New Ticket - Enter description (Enter = Create, Esc = Cancel) ")
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::Yellow))
        .wrap(Wrap { trim: false });
    frame.render_widget(input, chunks[0]);

    let help_text = vec![
        Line::from("A minimal ZIP file will be created automatically."),
        Line::from("Use this to report bugs from beta testers or for testing."),
    ];
    let help = Paragraph::new(help_text)
        .block(Block::default().title(" Info ").borders(Borders::ALL))
        .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(help, chunks[1]);
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let help_text = match app.view {
        View::TicketList => "↑/↓: Select | Enter: Details | n: New ticket | r: Refresh | q: Quit",
        View::TicketDetail => {
            "↑/↓: Scroll | z: Open ZIP | c: Comment | 1/2/3: Status | Esc: Back"
        }
        View::ZipViewer => "↑/↓: Select | Enter: Open | Esc: Back",
        View::FileContent => "↑/↓: Scroll | Esc: Back",
        View::AddComment => "Enter: Send | Esc: Cancel",
        View::CreateTicket => "Enter: Create | Esc: Cancel",
    };

    let status = if let Some(msg) = &app.status_message {
        format!("{} | {}", msg, help_text)
    } else {
        help_text.to_string()
    };

    let bar = Paragraph::new(status)
        .block(Block::default().borders(Borders::ALL))
        .style(Style::default().fg(Color::White));
    frame.render_widget(bar, area);
}

fn format_size(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;

    if bytes >= MB {
        format!("({:.1} MB)", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("({:.1} KB)", bytes as f64 / KB as f64)
    } else {
        format!("({} B)", bytes)
    }
}
