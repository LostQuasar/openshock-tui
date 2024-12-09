use std::error::Error;
use std::fs::OpenOptions;
use std::io::Write;
use std::{env, io};
use std::time::{Duration, Instant};
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
use rzap::api::ListShockerSource;
use rzap::api_builder::OpenShockAPIBuilder;
use tui_textarea::{ Input, Key, TextArea };
struct Screen {
    term: Terminal<CrosstermBackend<io::StdoutLock<'static>>>,
}
type Result<T> = std::result::Result<T, Box<dyn Error>>;

impl Screen {
    fn new() -> Result<Self> {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        enable_raw_mode()?;
        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        Ok(Screen { term: Terminal::new(backend)? })
    }

    fn api_key_prompt(&mut self) -> Result<String> {
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

    fn show_hello(&mut self, username: String) -> Result<()> {
        let paragraph = Paragraph::new(String::from(format!("Hello {}!", username)))
            .centered()
            .style(Style::new().bold());
        let display_time = Instant::now();
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
            if display_time.elapsed() > Duration::from_secs(2){
                break;
            }
        }
        Ok(())
    }

    fn show_shocker_list(&mut self, items: Vec<&str>) -> Result<String> {
        //let items = ["Leg Shocker", "Arm Shocker", "Thigh Shocker"];
        let mut state = ListState::default().with_selected(Some(0));
        let list_quantity = &items.len();
        let list = List::new(items.to_owned())
            .block(Block::bordered().title("Shockers").border_type(BorderType::Rounded))
            .style(Style::new().white())
            .highlight_style(Style::new().bold().light_blue())
            .highlight_symbol("  ")
            .repeat_highlight_symbol(false)
            .direction(ListDirection::TopToBottom);
            
        loop {
            self.term.draw(|f| {
                let outer_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Max(28)]) 
                    .flex(Flex::Center)
                    .split(f.area());
                let inner_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Max((list_quantity+2).try_into().unwrap())].as_ref())
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
                            return Ok(items.get(state.selected().unwrap()).unwrap().to_string());
                        }
                        _ => {},
                    }
                }
            }
        }
    }

    fn close(&mut self) -> Result<()> {
        disable_raw_mode()?;
        crossterm::execute!(self.term.backend_mut(), LeaveAlternateScreen, DisableMouseCapture)?;
        Ok(self.term.show_cursor()?)
    }
}

impl Drop for Screen {
    fn drop(&mut self) {
        self.close().unwrap();
    }
}

#[tokio::main]
async fn main() -> Result<()>{
    dotenvy::dotenv()?;
    let apikey =env::var("APIKEY");
    let mut screen = Screen::new().unwrap();

    let key; 
    match apikey {
        Ok(apikey) => key = apikey,
        Err(_) => {
            key = screen.api_key_prompt()?;
            let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .append(true)
            .open(".env")
            .unwrap();
            file.write(format!("APIKEY = {}",key).as_bytes())?;
        },
    }

    let openshock_api = OpenShockAPIBuilder::new()
        .with_default_api_token(key)
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
    let mut available_shockers_names: Vec<String> = vec![]; 
    let resp = openshock_api.get_shockers(ListShockerSource::Own, None).await?;
    match resp {
        Some(list_shockers_response) => {
            for shocker in list_shockers_response {
                available_shockers_names.push(shocker.shockers.iter().map(|s| s.name.as_deref().unwrap()).collect());
            }
        }
        None => todo!()
    }
    let resp = openshock_api.get_shockers(ListShockerSource::Shared, None).await?;
    match resp {
        Some(list_shockers_response) => {
            for shocker in list_shockers_response {
                available_shockers_names.push(shocker.shockers.iter().map(|s|s.name.as_deref().unwrap()).collect());
            }
        }
        None => todo!()
    }

    let selected = screen.show_shocker_list(available_shockers_names.iter().map(|s| &**s).collect()).unwrap();
    screen.close()?;
    print!("{}",selected);
    Ok(())
}
