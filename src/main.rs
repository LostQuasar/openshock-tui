use std::error;
use std::io::{ self, Error };
use crossterm::event::{ DisableMouseCapture, EnableMouseCapture };
use crossterm::terminal::{
    disable_raw_mode,
    enable_raw_mode,
    EnterAlternateScreen,
    LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::style::{ Style, Stylize };
use ratatui::Terminal;
use ratatui::layout::{ Constraint, Direction, Flex, Layout };
use ratatui::widgets::{ Block, BorderType, Borders, List, ListDirection, ListState, Paragraph };
use rzap::api_builder::OpenShockAPIBuilder;
use tui_textarea::{ Input, Key, TextArea };

struct Screen {
    term: Terminal<CrosstermBackend<io::StdoutLock<'static>>>,
}

impl Screen {
    fn new() -> Result<Self, Error> {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        enable_raw_mode()?;
        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        Ok(Screen { term: Terminal::new(backend)? })
    }

    fn api_key_prompt(&mut self) -> Result<String, Error> {
        let mut text_area = TextArea::default();
        text_area.set_placeholder_text("Enter your Openshock API KEY");
        text_area.set_block(
            Block::default()
                .borders(Borders::all())
                .border_type(BorderType::Rounded)
                .title("API Key")
        );
        text_area.set_mask_char('*');

        loop {
            self.term.draw(|f| {
                let outer_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Max(56)])
                    .flex(Flex::Center)
                    .split(f.area());

                let inner_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3)].as_ref())
                    .flex(Flex::Center)
                    .split(outer_layout[0]);
                f.render_widget(&text_area, inner_layout[0]);
            })?;

            match crossterm::event::read()?.into() {
                Input { key: Key::Enter, .. } => {
                    break;
                }
                input => {
                    text_area.input(input);
                }
            }
        }
        Ok(text_area.lines()[0].clone())
    }

    fn show_hello(&mut self, username: String) -> Result<(), Error> {
        let paragraph = Paragraph::new(String::from(format!("Hello {}!", username)))
            .centered()
            .style(Style::new().bold());

        loop {
            self.term.draw(|f| {
                let outer_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Max(56)])
                    .flex(Flex::Center)
                    .split(f.area());
                let inner_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3)].as_ref())
                    .flex(Flex::Center)
                    .split(outer_layout[0]);
                f.render_widget(&paragraph, inner_layout[0]);
            })?;

            match crossterm::event::read()?.into() {
                Input { key: Key::Enter, .. } => {
                    break;
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn show_shocker_list(&mut self) -> Result<(), Error> {
        let items = ["Leg Shocker", "Arm Shocker", "Thigh Shocker"];
        let mut state = ListState::default().with_selected(Some(0));
        let list = List::new(items)
            .block(Block::bordered().title("Shockers").border_type(BorderType::Rounded))
            .style(Style::new().white())
            .highlight_style(Style::new().bold())
            .repeat_highlight_symbol(true)
            .direction(ListDirection::TopToBottom);
        loop {
            self.term.draw(|f| {
                let outer_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Max(24)])
                    .flex(Flex::Center)
                    .split(f.area());
                let inner_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Max((items.len()+2).try_into().unwrap())].as_ref())
                    .flex(Flex::Center)
                    .split(outer_layout[0]);
                f.render_stateful_widget(&list, inner_layout[0], &mut state);
            })?;

            match crossterm::event::read()?.into() {
                Input { key,.. } => {
                    match key {
                        Key::Down => {
                            state.select_next();
                        }
                        Key::Up => {
                            state.select_previous();
                        }
                        Key::Enter => {
                            break;
                        }
                        _ => {},
                    }
                }
            }
        }
        Ok(())
    }

    fn close(&mut self) -> Result<(), Error> {
        disable_raw_mode()?;
        crossterm::execute!(self.term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        self.term.show_cursor()
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        self.close().unwrap();
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn error::Error>> {
    let mut screen = Screen::new().unwrap();
    let openshock_api = OpenShockAPIBuilder::new()
        .with_default_api_token(screen.api_key_prompt()?)
        .build()?;
    let resp = openshock_api.get_user_info(None).await?;
    match resp {
        Some(self_response) =>
            match self_response.name {
                Some(username) => screen.show_hello(username)?,
                None => todo!(),
            }
        None => todo!(),
    }
    let _ = screen.show_shocker_list();
    screen.close()?;
    Ok(())
}
