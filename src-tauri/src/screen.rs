//! A placeholder screen that proves the pipeline: input in, frames out, resize, the client crate
//! linked. The konstruct screens replace it once they live in a crate both shells share.

use construct_client::config::SessionState;
use ratatui::{
    Frame,
    crossterm::event::{Event, KeyEvent, KeyModifiers},
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

// The Phosphor slots this screen needs, from construct-tui's `theme.rs`. They go away with it.
const BACKGROUND: Color = Color::Rgb(6, 32, 56);
const FOREGROUND: Color = Color::Rgb(234, 242, 244);
const MUTED: Color = Color::Rgb(122, 157, 176);
const BORDER: Color = Color::Rgb(82, 125, 144);
const ACCENT: Color = Color::Rgb(84, 180, 255);
const SELECTION: Color = Color::Rgb(0, 140, 255);
const SELECTION_TEXT: Color = Color::Rgb(3, 30, 30);

pub struct Screen {
    session: &'static str,
    cols: u16,
    rows: u16,
    last_key: String,
    last_paste: usize,
}

impl Screen {
    pub fn new(session: &SessionState) -> Self {
        Self {
            session: match session {
                SessionState::Encrypted => "encrypted session on disk",
                SessionState::Plaintext => "plaintext session on disk",
                SessionState::None => "no session on disk",
            },
            cols: 0,
            rows: 0,
            last_key: "—".into(),
            last_paste: 0,
        }
    }

    pub fn on_resize(&mut self, cols: u16, rows: u16) {
        self.cols = cols;
        self.rows = rows;
    }

    pub fn on_event(&mut self, event: Event) {
        match event {
            Event::Key(key) => self.last_key = describe(&key),
            Event::Paste(text) => self.last_paste = text.chars().count(),
            _ => {}
        }
    }

    pub fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        frame.render_widget(
            Block::default().style(Style::default().bg(BACKGROUND)),
            area,
        );
        let [body, status] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(area);

        let row = |label: &str, value: String| {
            Line::from(vec![
                Span::styled(format!(" {label:<10}"), Style::default().fg(MUTED)),
                Span::styled(value, Style::default().fg(FOREGROUND)),
            ])
        };
        let lines = vec![
            Line::styled(
                " This window is the desktop shell skeleton.",
                Style::default().fg(FOREGROUND),
            ),
            Line::styled(
                " The konstruct screens are not wired in yet.",
                Style::default().fg(MUTED),
            ),
            Line::default(),
            row("grid", format!("{}×{}", self.cols, self.rows)),
            row("account", self.session.into()),
            row("last key", self.last_key.clone()),
            row("pasted", format!("{} chars", self.last_paste)),
        ];
        let panel = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(BORDER))
            .title(Span::styled("[ KONSTRUCT ]", Style::default().fg(ACCENT)))
            .title_top(Line::styled("desktop ", Style::default().fg(MUTED)).right_aligned());
        frame.render_widget(Paragraph::new(lines).block(panel), body);

        frame.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(" READY ", Style::default().fg(SELECTION_TEXT).bg(SELECTION)),
                Span::styled("  keys and paste echo above", Style::default().fg(MUTED)),
            ])),
            status,
        );
    }
}

fn describe(key: &KeyEvent) -> String {
    let mut parts = Vec::new();
    for (flag, name) in [
        (KeyModifiers::CONTROL, "Ctrl"),
        (KeyModifiers::ALT, "Alt"),
        (KeyModifiers::SUPER, "Super"),
        (KeyModifiers::SHIFT, "Shift"),
    ] {
        if key.modifiers.contains(flag) {
            parts.push(name.to_string());
        }
    }
    parts.push(key.code.to_string());
    parts.join("+")
}
