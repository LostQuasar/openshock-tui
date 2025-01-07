use std::cell::Cell;
use std::cmp::Ordering;
use std::fs::OpenOptions;
use std::io::{stdout, Write};
use std::str::FromStr;
use std::{ env, error, io };
use std::time::{ Duration, Instant, SystemTime };
use crossterm::event::{ DisableMouseCapture, EnableMouseCapture };
use crossterm::execute;
use crossterm::terminal::{ * };
use ratatui::backend::CrosstermBackend;
use ratatui::style::{ Color, Style, Stylize };
use ratatui::text::Span;
use ratatui::{ Frame, Terminal };
use ratatui::layout::{ Constraint, Direction, Flex, Layout, Rect };
use ratatui::widgets::{
    Block,
    BorderType,
    Borders,
    Gauge,
    List,
    ListDirection,
    ListState,
    Paragraph,
};
use rzap::api::{ ListShockerSource, OpenShockAPI };
use rzap::api_builder::OpenShockAPIBuilder;
use rzap::data_type::{ ControlType, ShockerResponse };
use tui_textarea::{ Input, Key, TextArea };
use std::panic::{set_hook, take_hook};

struct Screen {
    term: Terminal<CrosstermBackend<io::StdoutLock<'static>>>,
}
type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

#[derive(PartialEq)]
enum GaugeFormat {
    Percentage,
    Time,
    CountDown,
}
struct GaugeObject {
    title: Cell<String>,
    min: u16,
    fill: Cell<u16>,
    max: Cell<u16>,
    format: GaugeFormat,
    font_color: Color,
    bar_color: Color,
}

const CONTROL_TYPE_ARRAY:  [&'static str; 3] = ["Shock ⚡", "Vibrate 📳", "Sound 🔊"];
const ACTION_ARRAY:  [&'static str; 4] = ["Shocking...", "Vibratating...", "Beeping...", "Stopping..."];
const COLOR_OS_PINK: Color = Color::from_u32(0x00e14a6d);
const COLOR_OS_BLUE: Color = Color::from_u32(0x00afe3fe);   
const COLOR_OS_WHITE: Color = Color::from_u32(0x00ffffff);
const COLOR_OS_DARK_WHITE: Color = Color::from_u32(0x00b3b3b3);

fn render_gauge(gauge: &GaugeObject, selected: bool, area: Rect, f: &mut Frame<'_>) {
    let label;
    match gauge.format {
        GaugeFormat::Percentage => {
            label = Span::styled(
                format!("{:.1}%", gauge.fill.get()),
                Style::new().fg(gauge.font_color)
            );
        }
        GaugeFormat::Time | GaugeFormat::CountDown => {
            let string   = match gauge.fill.get().cmp(&10) {
                Ordering::Less => format!("{:.1}00 ms", gauge.fill.get()),
                Ordering::Greater | Ordering::Equal =>
                    format!("{:.1} s", (gauge.fill.get() as f32) / 10.0),
            };

            label = Span::styled(string, Style::new().fg(gauge.font_color));
        }
    }
    let layout = Layout::vertical([Constraint::Length(1), Constraint::Min(1)]).split(area);
    f.render_widget(ratatui::widgets::Clear, area);
    if (gauge.format != GaugeFormat::CountDown) | (gauge.fill.get() != 0) {
        let title = gauge.title.take();
        gauge.title.set(title.clone());
        let mut title_widget = Block::default().title(title).fg(gauge.bar_color);
        if selected {
            title_widget = title_widget.bold()
        }
        f.render_widget(title_widget, area);
        f.render_widget(
            Gauge::default()
                .label(label)
                .gauge_style(gauge.bar_color)
                .ratio(f64::from(gauge.fill.get() + 1) / f64::from(gauge.max.get() + 1)),
            layout[1]
        );
    }
}

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
            if display_time.elapsed() > Duration::from_millis(1500) {
                break;
            }
        }
        Ok(())
    }

    fn show_shocker_list(&mut self, items: &Vec<ShockerResponse>) -> Result<usize> {
        //let items = ["Leg Shocker", "Arm Shocker", "Thigh Shocker"];
        let mut state = ListState::default().with_selected(Some(0));
        let list_quantity = &items.len();
        let names: Vec<String> = items
            .iter()
            .map(|s| s.name.clone().unwrap())
            .collect();
        let list = List::new(names)
            .block(Block::bordered().title("Shockers").border_type(BorderType::Rounded))
            .style(Style::new().white())
            .highlight_style(Style::new().bold().fg(COLOR_OS_BLUE))
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
                    .constraints(
                        [Constraint::Max((list_quantity + 2).try_into().unwrap())].as_ref()
                    )
                    .flex(Flex::Center)
                    .split(outer_layout[0]);
                f.render_stateful_widget(&list, inner_layout[0], &mut state);
            })?;

            match crossterm::event::read()?.into() {
                Input { key, .. } => {
                    match key {
                        Key::Down => {
                            state.select_next();
                        }
                        Key::Up => {
                            state.select_previous();
                        }
                        Key::Enter => {
                            return Ok(state.selected().unwrap());
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    async fn show_shocker_controls(&mut self, api: OpenShockAPI, id: &String) -> Result<()> {
        let mut time = SystemTime::now();
        let mut control_type = 0;
        let gauges = [
            &(GaugeObject {
                title: Cell::new("Strength".to_string()),
                min: 0,
                fill: Cell::new(1),
                max: Cell::new(100),
                format: GaugeFormat::Percentage,
                font_color: COLOR_OS_WHITE,
                bar_color: COLOR_OS_PINK,
            }),
            &(GaugeObject {
                title: Cell::new("Duration".to_string()),
                min: 3,
                fill: Cell::new(3),
                max: Cell::new(300),
                format: GaugeFormat::Time,
                font_color: COLOR_OS_WHITE,
                bar_color: COLOR_OS_BLUE,
            }),
            &(GaugeObject {
                title: Cell::new(ACTION_ARRAY[control_type].to_string()),
                min: 0,
                fill: Cell::new(0),
                max: Cell::new(300),
                format: GaugeFormat::CountDown,
                font_color: COLOR_OS_WHITE,
                bar_color: COLOR_OS_DARK_WHITE,
            }),
        ];
        let mut selected = 0;
        loop {
            if &gauges[2].fill.get() > &0 {
                let since = (time.elapsed().unwrap().as_millis() / 100) as u16;
                if since < gauges[2].max.get() {
                    gauges[2].fill.set(gauges[2].max.get().checked_sub(since).unwrap());
                } else {
                    gauges[2].fill.set(0);
                }
            }
            self.term.draw(|f| {
                let outer_layout = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Max(38)])
                    .flex(Flex::Center)
                    .split(f.area());
                let inner_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Max(12)])
                    .flex(Flex::Center)
                    .split(outer_layout[0]);
                let gauge_layout = Layout::vertical([
                    Constraint::Length(2),
                    Constraint::Length(2),
                    Constraint::Length(2),
                    Constraint::Length(2),
                ]);
                let gauge_areas: [ratatui::prelude::Rect; 4] = gauge_layout.areas(inner_layout[0]);
                let mut i = 1;
                let mut control_type_widget = Block::default().title(format!("{}", CONTROL_TYPE_ARRAY[control_type]));
                if selected == 0 {
                    control_type_widget = control_type_widget.bold()
                }
                f.render_widget(
                    control_type_widget,
                    gauge_areas[0]
                );
                for gauge in gauges {
                    render_gauge(gauge, i == selected, gauge_areas[i], f);
                    i += 1;
                }
            })?;
            if crossterm::event::poll(Duration::from_millis(50))? {
                match crossterm::event::read()?.into() {
                    Input { key, .. } => {
                        match key {
                            Key::Right => {
                                if selected != 0 {
                                    let i = selected - 1;
                                    if gauges[i].fill.get() < gauges[i].max.get() {
                                        gauges[i].fill.set(gauges[i].fill.get() + 1);
                                    }
                                }
                                else {
                                    if control_type == 2 {
                                        control_type -= 2
                                    }
                                    else {
                                        control_type += 1
                                    }
                                }
                            }
                            Key::Left => {
                                if selected != 0 {
                                    let i = selected - 1;
                                    if gauges[i].fill.get() > gauges[i].min {
                                        gauges[i].fill.set(gauges[i].fill.get() - 1);
                                    }
                                }
                                else {
                                    if control_type == 0 {
                                        control_type += 2
                                    }
                                    else {
                                        control_type -= 1
                                    }
                                }
                            }
                            Key::Up => {
                                if selected > 0 {
                                    selected -= 1;
                                }
                            }
                            Key::Down => {
                                if selected < 2{
                                    selected += 1;
                                }
                            }
                            Key::Esc => {
                                return Ok(());
                            }
                            Key::Enter => {
                                if gauges[2].fill.get() == 0 {
                                    gauges[2].title.set(ACTION_ARRAY[control_type].to_string());
                                    let dur = gauges[1].fill.get();
                                    gauges[2].fill.set(dur);
                                    gauges[2].max.set(dur);
                                    time = SystemTime::now();
                                    api.post_control(
                                        id.clone(),
                                        ControlType::from_str(CONTROL_TYPE_ARRAY[control_type])?,
                                        gauges[0].fill.get() as u8,
                                        dur * 100,
                                        None
                                    ).await?;
                                }
                            }
                            Key::Backspace => {
                                gauges[2].title.set(ACTION_ARRAY[3].to_string());
                                let dur = 5;
                                gauges[2].fill.set(dur);
                                gauges[2].max.set(dur);
                                time = SystemTime::now();
                                api.post_control(
                                    id.clone(),
                                    ControlType::Stop,
                                    gauges[0].fill.get() as u8,
                                    dur * 100,
                                    None
                                ).await?;

                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    fn close(&mut self) -> Result<()> {
        restore_tui()?;
        self.term.show_cursor()?;
        Ok(())
    }
}

fn restore_tui() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

pub fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        let _ = restore_tui();
        original_hook(panic_info);
    }));
}

#[tokio::main]
async fn main() -> Result<()> {
    init_panic_hook();
    dotenvy::dotenv()?;
    let apikey = env::var("APIKEY");
    let mut screen = Screen::new()?;

    let key;
    match apikey {
        Ok(apikey) => {
            key = apikey;
        }
        Err(_) => {
            key = screen.api_key_prompt()?;
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .append(true)
                .open(".env")
                .unwrap();
            file.write(format!("APIKEY = {}", key).as_bytes())?;
        }
    }

    let openshock_api = OpenShockAPIBuilder::new()
        .with_default_api_token(key)
        .with_app("OpenShockTUI".to_string(), None)
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
    let mut available_shockers = vec![];
    let resp = openshock_api.get_shockers(ListShockerSource::Own, None).await?;
    match resp {
        Some(list_shockers_response) => {
            for mut shocker in list_shockers_response {
                available_shockers.append(&mut shocker.shockers);
            }
        }
        None => todo!(),
    }

    let index = screen.show_shocker_list(&available_shockers)?;

    screen.show_shocker_controls(openshock_api, &available_shockers[index].id).await?;
    screen.close()?;
    Ok(())
}
