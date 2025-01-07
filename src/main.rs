use std::cmp::Ordering;
use std::fs::OpenOptions;
use std::io::{stdout, Write};
use std::{ env, error, io };
use crossterm::event::DisableMouseCapture;
use crossterm::execute;
use crossterm::terminal::{ * };
use ratatui::style::{ Color, Style, Stylize };
use ratatui::text::Span;
use ratatui::Frame;
use ratatui::layout::{ Constraint, Layout, Rect };
use ratatui::widgets::{
    Block,
    Gauge,
    
};
use rzap::api::ListShockerSource;
use rzap::api_builder::OpenShockAPIBuilder;
use screen::Screen;
use std::panic::{set_hook, take_hook};
pub mod screen;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;


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
