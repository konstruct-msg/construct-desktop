//! The window's character grid. Ratatui draws into a byte buffer as it would into a terminal, and
//! each frame goes to xterm.js in the webview, which only paints it.
//!
//! There is no terminal behind this, so the size the backend reports is whatever the webview last
//! said — crossterm would ask a tty, and there is none (ENXIO on the first frame).

use std::io::{self, Write};

use ratatui::{
    Terminal,
    backend::{Backend, ClearType, CrosstermBackend, WindowSize},
    buffer::Cell,
    crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
    layout::{Position, Size},
};
use tauri::ipc::Channel;
use tokio::sync::mpsc;

use crate::screen::Screen;

/// What the webview sends the grid.
pub enum Input {
    /// A (re)loaded page: draw everything from scratch into this channel.
    Attach {
        cols: u16,
        rows: u16,
        frames: Channel<String>,
    },
    Resize {
        cols: u16,
        rows: u16,
    },
    Event(Event),
}

/// Text from xterm.js — a typed character, an IME result or a paste — as the event the terminal
/// would have produced for it.
pub fn text_event(text: String) -> Option<Event> {
    let mut chars = text.chars();
    match (chars.next(), chars.next()) {
        (None, _) => None,
        (Some(c), None) => Some(Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            modifiers: KeyModifiers::empty(),
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })),
        _ => Some(Event::Paste(text)),
    }
}

/// Collects what ratatui writes during a frame and sends it as one message on flush.
struct FrameSink {
    pending: Vec<u8>,
    frames: Channel<String>,
}

impl Write for FrameSink {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if self.pending.is_empty() {
            return Ok(());
        }
        // Ratatui writes whole strings, so a frame never ends inside a character.
        let frame = String::from_utf8_lossy(&self.pending).into_owned();
        self.pending.clear();
        // A closed channel is a page being reloaded; its successor attaches and is redrawn.
        let _ = self.frames.send(frame);
        Ok(())
    }
}

/// Crossterm's ANSI output with the size the webview reported.
struct GridBackend {
    ansi: CrosstermBackend<FrameSink>,
    size: Size,
}

impl Backend for GridBackend {
    type Error = io::Error;

    fn draw<'a, I>(&mut self, content: I) -> io::Result<()>
    where
        I: Iterator<Item = (u16, u16, &'a Cell)>,
    {
        self.ansi.draw(content)
    }

    fn hide_cursor(&mut self) -> io::Result<()> {
        self.ansi.hide_cursor()
    }

    fn show_cursor(&mut self) -> io::Result<()> {
        self.ansi.show_cursor()
    }

    // Asking where the cursor is means reading the terminal's answer; nothing would answer. Only
    // an inline viewport asks, and this grid is always full-screen.
    fn get_cursor_position(&mut self) -> io::Result<Position> {
        Ok(Position::ORIGIN)
    }

    fn set_cursor_position<P: Into<Position>>(&mut self, position: P) -> io::Result<()> {
        self.ansi.set_cursor_position(position)
    }

    fn clear(&mut self) -> io::Result<()> {
        self.ansi.clear()
    }

    fn clear_region(&mut self, clear_type: ClearType) -> io::Result<()> {
        self.ansi.clear_region(clear_type)
    }

    fn size(&self) -> io::Result<Size> {
        Ok(self.size)
    }

    fn window_size(&mut self) -> io::Result<WindowSize> {
        Ok(WindowSize {
            columns_rows: self.size,
            // Pixel size is for image protocols; nothing here draws images yet.
            pixels: Size::default(),
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Backend::flush(&mut self.ansi)
    }
}

type Grid = Terminal<GridBackend>;

fn attach(cols: u16, rows: u16, frames: Channel<String>) -> io::Result<Grid> {
    let sink = FrameSink {
        pending: Vec::new(),
        frames,
    };
    let mut grid = Terminal::new(GridBackend {
        ansi: CrosstermBackend::new(sink),
        size: Size::new(cols, rows),
    })?;
    grid.clear()?;
    Ok(grid)
}

/// Owns the grid and the screen; redraws after every input.
pub async fn run(mut inputs: mpsc::UnboundedReceiver<Input>, mut screen: Screen) {
    let mut grid: Option<Grid> = None;
    while let Some(input) = inputs.recv().await {
        match input {
            Input::Attach { cols, rows, frames } => match attach(cols, rows, frames) {
                Ok(g) => {
                    screen.on_resize(cols, rows);
                    grid = Some(g);
                }
                Err(e) => tracing::error!("attach the grid: {e}"),
            },
            Input::Resize { cols, rows } => {
                // The next draw sees the new size and resizes its buffers.
                if let Some(g) = grid.as_mut() {
                    g.backend_mut().size = Size::new(cols, rows);
                }
                screen.on_resize(cols, rows);
            }
            Input::Event(event) => screen.on_event(event),
        }
        if let Some(g) = grid.as_mut()
            && let Err(e) = g.draw(|frame| screen.render(frame))
        {
            tracing::error!("draw: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn one_character_is_a_key_press() {
        let Some(Event::Key(key)) = text_event("ж".into()) else {
            panic!("not a key");
        };
        assert_eq!(key.code, KeyCode::Char('ж'));
    }

    #[test]
    fn more_than_one_character_is_a_paste() {
        assert_eq!(
            text_event("привет".into()),
            Some(Event::Paste("привет".into()))
        );
    }

    #[test]
    fn nothing_is_no_event() {
        assert_eq!(text_event(String::new()), None);
    }
}
