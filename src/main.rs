use crossterm::event::{ DisableMouseCapture, EnableMouseCapture };
use crossterm::terminal::{
    disable_raw_mode,
    enable_raw_mode,
    EnterAlternateScreen,
    LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{ Constraint, Direction, Flex, Layout };
use ratatui::style::{ Color, Modifier, Style };
use ratatui::widgets::{ Block, Borders };
use ratatui::Terminal;
use std::io;
use tui_textarea::{ Input, Key, TextArea };

struct EntryField<'a> {
    textarea: TextArea<'a>,
    title: String,
}

impl EntryField<'_> {
    fn inactivate(&mut self) {
        self.textarea.set_cursor_line_style(Style::default());
        self.textarea.set_cursor_style(Style::default());
        self.textarea.set_block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::DarkGray))
                .title(self.title.clone())
        );
        self.textarea.set_placeholder_text("");
    }
    fn activate(&mut self) {
        let title = self.title.clone();
        self.textarea.set_cursor_line_style(Style::default().add_modifier(Modifier::UNDERLINED));
        self.textarea.set_cursor_style(Style::default().add_modifier(Modifier::REVERSED));
        self.textarea.set_placeholder_text(format!("Please enter your {title}"));
        self.textarea.set_block(
            Block::default().borders(Borders::ALL).style(Style::default()).title(title)
        );
    }
}

fn main() -> io::Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let usernamearea = EntryField {
        textarea: TextArea::default(),
        title: "Username".to_string(),
    };

    let mut passwordarea = EntryField {
        textarea: TextArea::default(),
        title: "Password".to_string(),
    };
    passwordarea.textarea.set_mask_char('*');

    let mut textareas = [usernamearea, passwordarea];
    let mut which = 0;
    textareas[0].activate();
    textareas[1].inactivate();

    loop {
        term.draw(|f| {
            let outer_layout = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Max(40)])
                .flex(Flex::Center)
                .split(f.area());

            let inner_layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Length(3)].as_ref())
                .flex(Flex::Center)
                .split(outer_layout[0]);

            for (textarea, chunk) in textareas.iter().zip(inner_layout.iter()) {
                f.render_widget(&textarea.textarea, *chunk);
            }
        })?;
        
        match crossterm::event::read()?.into() {
            Input { key: Key::Esc, .. } => {
                break;
            }
            Input { key: Key::Enter, .. } => {
                textareas[which].inactivate();
                which = (which + 1) % 2;
                textareas[which].activate();
            }
            input => {
                textareas[which].textarea.input(input);
            }
        }
    }

    disable_raw_mode()?;
    crossterm::execute!(term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
    term.show_cursor()?;

    println!("Username: {:?}", textareas[0].textarea.lines());
    println!("Password: {:?}", textareas[1].textarea.lines());
    Ok(())
}
