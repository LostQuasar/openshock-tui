use std::fs::{ self, OpenOptions };
use std::io::{ stdout, Read, Write };
use std::{ error, io };
use crossterm::event::DisableMouseCapture;
use crossterm::execute;
use crossterm::terminal::{ disable_raw_mode, LeaveAlternateScreen };
use platform_dirs::AppDirs;
use rzap::api::ListShockerSource;
use rzap::api_builder::OpenShockAPIBuilder;
use screen::Screen;
use std::panic::{ set_hook, take_hook };
pub mod screen;

type Result<T> = std::result::Result<T, Box<dyn error::Error>>;

fn restore_tui() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(
        Box::new(move |panic_info| {
            let _ = restore_tui();
            original_hook(panic_info);
        })
    );
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
    let key;
    match file_result {
        Ok(mut file) => {
            let mut buffer = [0; 64];
            file.read(&mut buffer[..])?;
            key = String::from_utf8(buffer.to_vec())?;
        }
        Err(_) => {
            key = screen.api_key_prompt()?;
            //TODO: VERIFY BEFORE WRITING
            let mut file = OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(format!("{}/data.bin", app_dirs.data_dir.display()))?;
            let _ = file.write(key.as_bytes());
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
