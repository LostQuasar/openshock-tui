use std::{ cell, io::StdoutLock, str::FromStr, time::* };

use crossterm::{ event::EnableMouseCapture, terminal::{ enable_raw_mode, EnterAlternateScreen } };
use gauges::*;
use rzap::{ api::OpenShockAPI, data_type::{ ControlType, ShockerResponse } };
use tui_textarea::{ Input, Key, TextArea };
use crate::*;
use ratatui::{ layout::*, prelude::CrosstermBackend, widgets::*, Terminal };

const CONTROL_TYPE_ARRAY: [&'static str; 3] = ["Shock ⚡", "Vibrate 📳", "Sound 🔊"];
const ACTION_ARRAY: [&'static str; 4] = [
    "Shocking...",
    "Vibratating...",
    "Beeping...",
    "Stopping...",
];
const COLOR_OS_PINK: Color = Color::from_u32(0x00e14a6d);
const COLOR_OS_BLUE: Color = Color::from_u32(0x00afe3fe);
const COLOR_OS_WHITE: Color = Color::from_u32(0x00ffffff);
const COLOR_OS_DARK_WHITE: Color = Color::from_u32(0x00b3b3b3);
pub mod gauges;

pub(crate) struct Screen {
    pub(crate) term: Terminal<CrosstermBackend<StdoutLock<'static>>>,
}

impl Screen {
    pub(crate) fn new() -> Result<Self> {
        let stdout = io::stdout();
        let mut stdout = stdout.lock();
        enable_raw_mode()?;
        crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        Ok(Screen { term: Terminal::new(backend)? })
    }

    pub(crate) fn api_key_prompt(&mut self) -> Result<String> {
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

    pub(crate) fn show_hello(&mut self, username: String) -> Result<()> {
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

    pub(crate) fn show_shocker_list(&mut self, items: &Vec<ShockerResponse>) -> Result<usize> {
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

    pub(crate) async fn show_shocker_controls(
        &mut self,
        api: OpenShockAPI,
        id: &String
    ) -> Result<()> {
        let mut time = SystemTime::now();
        let mut control_type = 0;
        let gauges = [
            &(GaugeObject {
                title: cell::Cell::new("Strength".to_string()),
                min: 0,
                fill: cell::Cell::new(1),
                max: cell::Cell::new(100),
                format: GaugeFormat::Percentage,
                font_color: COLOR_OS_WHITE,
                bar_color: COLOR_OS_PINK,
            }),
            &(GaugeObject {
                title: cell::Cell::new("Duration".to_string()),
                min: 3,
                fill: cell::Cell::new(3),
                max: cell::Cell::new(300),
                format: GaugeFormat::Time,
                font_color: COLOR_OS_WHITE,
                bar_color: COLOR_OS_BLUE,
            }),
            &(GaugeObject {
                title: cell::Cell::new(ACTION_ARRAY[control_type].to_string()),
                min: 0,
                fill: cell::Cell::new(0),
                max: cell::Cell::new(300),
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
                let mut control_type_widget = Block::default().title(
                    format!("{}", CONTROL_TYPE_ARRAY[control_type])
                );
                if selected == 0 {
                    control_type_widget = control_type_widget.bold();
                }
                f.render_widget(control_type_widget, gauge_areas[0]);
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
                                } else {
                                    if control_type == 2 {
                                        control_type -= 2;
                                    } else {
                                        control_type += 1;
                                    }
                                }
                            }
                            Key::Left => {
                                if selected != 0 {
                                    let i = selected - 1;
                                    if gauges[i].fill.get() > gauges[i].min {
                                        gauges[i].fill.set(gauges[i].fill.get() - 1);
                                    }
                                } else {
                                    if control_type == 0 {
                                        control_type += 2;
                                    } else {
                                        control_type -= 1;
                                    }
                                }
                            }
                            Key::Up => {
                                if selected > 0 {
                                    selected -= 1;
                                }
                            }
                            Key::Down => {
                                if selected < 2 {
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

    pub(crate) fn close(&mut self) -> Result<()> {
        restore_tui()?;
        self.term.show_cursor()?;
        Ok(())
    }
}
