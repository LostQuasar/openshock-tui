use crossterm::event::DisableMouseCapture;
use crossterm::execute;
use crossterm::terminal::{disable_raw_mode, LeaveAlternateScreen};
use platform_dirs::AppDirs;
use reqwest::StatusCode;
use rzap::api::ListShockerSource;
use rzap::api_builder::OpenShockAPIBuilder;
use screen::Screen;
use std::fs::{self, OpenOptions};
use std::io::{stdout, ErrorKind, Read, Write};
use std::panic::{set_hook, take_hook};
use std::{error, io};
pub mod screen;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

fn restore_tui() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        let _ = restore_tui();
        original_hook(panic_info);
    }));
}

fn update_key_file(app_dirs: &AppDirs, screen: &mut Screen) -> Result<String> {
    let mut key_invalid = true;
    let mut key = "".to_string();
    while key_invalid {
        key = screen.api_key_prompt()?;
        if screen.should_exit.get() {
            screen.close()?;
            return Ok("".to_string());
        }
        let buffer = key.as_bytes();
        if buffer.len() == 64 {
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(format!("{}/data.bin", app_dirs.data_dir.display()))?;
            let _ = file.write(buffer);
            key_invalid = false;
        }
    }
    Ok(key)
}

#[tokio::main]
async fn main() -> Result<()> {
    init_panic_hook();

    let mut screen = Screen::new()?;
    let app_dirs = AppDirs::new(env!("CARGO_CRATE_NAME").into(), false).unwrap();

    fs::create_dir_all(&app_dirs.data_dir)?;
    let file_result = OpenOptions::new()
        .read(true)
        .open(format!("{}/data.bin", app_dirs.data_dir.display()));
    let mut key;
    match file_result {
        Ok(mut file) => {
            let mut buffer = [0; 64];
            file.read(&mut buffer[..])?;
            key = String::from_utf8(buffer.to_vec())?;
        }
        Err(err) => match err.kind() {
            ErrorKind::NotFound => {
                key = update_key_file(&app_dirs, &mut screen)?;
            }
            _ => todo!(),
        },
    }
    let mut openshock_api = OpenShockAPIBuilder::new()
        .with_default_api_token(key)
        .with_app("OpenShockTUI".to_string(), None)
        .build()?;

    let resp = openshock_api.get_user_info(None).await;
    match resp {
        Ok(Some(self_response)) => match self_response.name {
            Some(username) => screen.show_hello(username)?,
            None => todo!(),
        },

        Err(err) => {
            match err {
                rzap::error::Error::Reqwest(error) => {
                    if error.status() == Some(StatusCode::UNAUTHORIZED) {
                        fs::remove_file(format!("{}/data.bin", app_dirs.data_dir.display()))?;
                        key = update_key_file(&app_dirs, &mut screen)?;
                        openshock_api = OpenShockAPIBuilder::new()
                            .with_default_api_token(key)
                            .with_app("OpenShockTUI".to_string(), None)
                            .build()?;
                    }
                }
                rzap::error::Error::Serde(_) => todo!(),
                rzap::error::Error::MissingApiToken => todo!(),
                rzap::error::Error::InvalidHeaderValue(_) => todo!(),
            }
            //update_key_file(&app_dirs, &mut screen)?;
        }
        _ => todo!(),
    }

    let mut available_shockers = vec![];
    let resp = openshock_api
        .get_shockers(ListShockerSource::Own, None)
        .await?;
    match resp {
        Some(list_shockers_response) => {
            for mut shocker in list_shockers_response {
                available_shockers.append(&mut shocker.shockers);
            }
        }
        None => todo!(),
    }
    let index = screen.show_shocker_list(&available_shockers)?;
    if screen.should_exit.get() {
        screen.close()?;
        return Ok(());
    }
    screen
        .show_shocker_controls(openshock_api, &available_shockers[index].id)
        .await?;
    screen.close()?;
    Ok(())
}
