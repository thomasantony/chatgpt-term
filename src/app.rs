use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, is_raw_mode_enabled, EnterAlternateScreen,
    LeaveAlternateScreen,
};
use std::borrow::Cow;
use std::fmt::Display;
use std::io;
use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Paragraph};

use tui::Terminal;
use tui_textarea::{CursorMove, Input, Key, TextArea};

use crate::api::{ChatGPTClient, ChatGPTSession, ChatLogEntry};

#[derive(Debug, Clone)]
pub enum UiEvent {
    Quit,
    SendMessage(String),
    SaveSession,
    // Help(String),
}

struct ChatEntryBox<'a> {
    textarea: TextArea<'a>,
}

impl<'a> Default for ChatEntryBox<'a> {
    fn default() -> Self {
        let mut textarea = TextArea::default();
        textarea.set_block(Block::default().borders(Borders::ALL).title("Input"));
        textarea.set_cursor_line_style(Style::default().fg(Color::Red));
        Self { textarea }
    }
}

impl<'a> ChatEntryBox<'a> {
    fn clear(&mut self) {
        // Remove input for next input. Do not recreate `self.textarea` instance to keep undo history so that users can
        // restore previous input easily.
        self.textarea.move_cursor(CursorMove::End);
        self.textarea.delete_line_by_head();
    }

    fn height(&self) -> u16 {
        3
    }

    fn input(&mut self, input: Input) -> Option<String> {
        match input {
            Input {
                key: Key::Enter, ..
            } => {
                let message = self.textarea.lines()[0].trim().to_string();
                self.clear();
                Some(message)
            }
            Input {
                key: Key::Char('m'),
                ctrl: true,
                ..
            } => None, // Disable shortcuts which inserts a newline. See `single_line` example
            input => {
                self.textarea.input(input);
                None
            }
        }
    }

    fn set_error(&mut self, err: Option<impl Display>) {
        let b = if let Some(err) = err {
            Block::default()
                .borders(Borders::ALL)
                .title(format!("Input: {}", err))
                .style(Style::default().fg(Color::Red))
        } else {
            Block::default().borders(Borders::ALL).title("Input")
        };
        self.textarea.set_block(b);
    }
}

struct ChatTermApp<'a> {
    current: usize,
    session: ChatGPTSession,
    message_area: TextArea<'a>,
    term: Terminal<CrosstermBackend<io::Stdout>>,
    error_message: Option<Cow<'static, str>>,
    input: ChatEntryBox<'a>,
}

impl<'a> ChatTermApp<'a> {
    fn new(session: ChatGPTSession) -> io::Result<Self> {
        let mut stdout = io::stdout();
        if !is_raw_mode_enabled()? {
            enable_raw_mode()?;
            crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        }
        let backend = CrosstermBackend::new(stdout);
        let term = Terminal::new(backend)?;

        let message_area = ChatTermApp::create_message_area_from_session(session.get_chatlog());
        Ok(Self {
            current: 0,
            session,
            term,
            error_message: None,
            message_area,
            // TODO: Add help box above input that pops up when typing /help
            input: ChatEntryBox::default(),
        })
    }

    // Add a new entry to the message area
    fn add_line_wrapped(text_area: &mut TextArea, line: &str, width: usize) {
        let wrap_width = if width > 6 { width - 5 } else { width };
        let wrapped_lines = textwrap::wrap(line, wrap_width);
        for (ctr, line) in wrapped_lines.into_iter().enumerate() {
            if ctr > 0 {
                // Prefix with five spaces to indicate a continuation of the previous line
                text_area.insert_str("     ");
            }
            text_area.insert_str(line);
            text_area.insert_newline();
        }
    }
    fn add_chatlog_entry(message_area: &mut TextArea, entry: &ChatLogEntry, width: usize) {
        // Add both message and response to message_area after wrapping them to width
        let message = format!("You: {}", entry.message);
        ChatTermApp::add_line_wrapped(message_area, &message, width);
        let message = format!("Bot: {}", entry.response);
        ChatTermApp::add_line_wrapped(message_area, &message, width);
    }

    // Clear the message area and add all the entries in the chatlog
    fn create_message_area_from_session(chatlog: &[ChatLogEntry]) -> TextArea<'a> {
        let mut message_area = TextArea::default();
        message_area.set_block(Block::default().borders(Borders::ALL).title("Chat Log"));
        message_area.set_style(Style::default().fg(Color::White));
        message_area.set_alignment(Alignment::Left);
        message_area.set_cursor_style(Style::default().fg(Color::Black));

        for entry in chatlog.iter() {
            ChatTermApp::add_chatlog_entry(&mut message_area, entry, 80);
        }
        message_area
    }

    fn update_ui(&mut self) -> Option<UiEvent> {
        let input_height = self.input.height();
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(1),
                    Constraint::Length(input_height),
                    Constraint::Length(1),
                    Constraint::Length(1),
                ]
                .as_ref(),
            );

        self.term
            .draw(|f| {
                let chunks = layout.split(f.size());

                f.render_widget(self.message_area.widget(), chunks[0]);

                // Render status line
                let slot = format!("[{}/{}]", self.current + 1, 10);
                let status_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints(
                        [
                            Constraint::Length(slot.len() as u16),
                            Constraint::Min(1),
                            Constraint::Length(10u16),
                        ]
                        .as_ref(),
                    )
                    .split(chunks[2]);
                let status_style = Style::default().add_modifier(Modifier::REVERSED);
                f.render_widget(Paragraph::new(slot).style(status_style), status_chunks[0]);
                f.render_widget(Paragraph::new("").style(status_style), status_chunks[1]);
                f.render_widget(Paragraph::new("0").style(status_style), status_chunks[2]);

                f.render_widget(self.input.textarea.widget(), chunks[1]);

                // Render message at bottom
                let message = if let Some(message) = self.error_message.take() {
                    Spans::from(Span::raw(message))
                } else {
                    Spans::from(vec![
                        Span::raw("Press "),
                        Span::styled("Esc", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to quit, "),
                        Span::styled("^S", Style::default().add_modifier(Modifier::BOLD)),
                        Span::raw(" to save session "),
                    ])
                };
                f.render_widget(Paragraph::new(message), chunks[3]);
            })
            .ok();
        match crossterm::event::read().ok().map(Into::into) {
            Some(Input { key: Key::Esc, .. }) => Some(UiEvent::Quit),
            Some(Input {
                key: Key::Char('s'),
                ctrl: true,
                alt: false,
            }) => Some(UiEvent::SaveSession),
            // Pass through mousescroll events to the message area
            Some(Input {
                key: Key::MouseScrollDown,
                ..
            }) => {
                self.message_area.input(Input {
                    key: Key::MouseScrollDown,
                    ..Default::default()
                });
                None
            }
            Some(Input {
                key: Key::MouseScrollUp,
                ..
            }) => {
                self.message_area.input(Input {
                    key: Key::MouseScrollUp,
                    ..Default::default()
                });
                None
            }
            Some(input) => self.input.input(input).and_then(|message_str| {
                if !message_str.is_empty() {
                    Some(UiEvent::SendMessage(message_str))
                } else {
                    None
                }
            }),
            _ => None,
        }
    }
}

impl<'a> Drop for ChatTermApp<'a> {
    fn drop(&mut self) {
        self.term.show_cursor().unwrap();
        if !is_raw_mode_enabled().unwrap() {
            return;
        }
        disable_raw_mode().unwrap();
        crossterm::execute!(
            self.term.backend_mut(),
            LeaveAlternateScreen,
            DisableMouseCapture
        )
        .unwrap();
    }
}

pub fn run(client: ChatGPTClient) -> Result<(), Box<dyn std::error::Error>> {
    // Load chat log from chatlog.json file and deserialize it
    // Create a new session
    let session = client.new_session(2000).with_log_file("chatlog.json")?;

    // TODO: Separate threads for input events, UI updates, and chatbot responses
    let mut app = ChatTermApp::new(session)?;
    loop {
        if let Some(ui_event) = app.update_ui() {
            match ui_event {
                UiEvent::SendMessage(message_str) => match app.session.send_message(&message_str) {
                    Ok(entry) => {
                        let width = app.term.get_frame().size().width as usize - 4;
                        ChatTermApp::add_chatlog_entry(&mut app.message_area, &entry, width);
                    }
                    Err(err) => {
                        app.input.set_error(Some(format!("Error: {:?}", err)));
                    }
                },
                UiEvent::SaveSession => match app.session.save_chatlog() {
                    Ok(filename) => {
                        app.error_message = Some(format!("Saved session to {}", filename).into());
                    }
                    Err(err) => {
                        app.error_message = Some(format!("Error: {:?}", err).into());
                    }
                },
                UiEvent::Quit => break,
            }
        }
    }

    Ok(())
}
