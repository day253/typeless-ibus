mod asr;
mod audio;
mod config;
mod engine;
mod ibus;

use anyhow::{Result, bail};
use config::Config;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "typeless_ibus=info".into()),
        )
        .init();

    if let Err(error) = run().await {
        eprintln!("typeless-ibus: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let argument = std::env::args().nth(1);
    match argument.as_deref() {
        Some("--help" | "-h") => {
            print_help();
            return Ok(());
        }
        Some("--version" | "-V") => {
            println!("typeless-ibus-engine {VERSION}");
            return Ok(());
        }
        Some("--config-path") => {
            println!("{}", config::config_path()?.display());
            return Ok(());
        }
        Some("--print-config") => {
            let path = config::config_path()?;
            let config = Config::load_or_create(&path)?;
            println!("{}", serde_json::to_string_pretty(&config)?);
            return Ok(());
        }
        Some("--write-default-config") => {
            let path = config::config_path()?;
            Config::default().save(&path)?;
            println!("已写入默认配置：{}", path.display());
            return Ok(());
        }
        Some("--list-devices") => {
            print_audio_devices()?;
            return Ok(());
        }
        Some("--check") => {
            let path = config::config_path()?;
            let config = Config::load_or_create(&path)?;
            println!("config: {}", path.display());
            println!(
                "trigger: {} ({:?})",
                config.trigger_key, config.trigger_mode
            );
            println!("ibus: {}", ibus::ibus_address()?);
            print_audio_devices()?;
            println!("check: ok");
            return Ok(());
        }
        Some("--ibus") | None => {}
        Some(argument) => bail!("未知参数：{argument}；使用 --help 查看帮助"),
    }

    let config_path = config::config_path()?;
    let config = Config::load_or_create(&config_path)?;
    let credentials_path = config::credentials_path()?;
    tracing::info!(
        config = %config_path.display(),
        trigger = %config.trigger_key,
        mode = ?config.trigger_mode,
        "starting Typeless IBus engine"
    );
    let _connection = ibus::serve(config, credentials_path).await?;
    tokio::signal::ctrl_c().await?;
    Ok(())
}

fn print_audio_devices() -> Result<()> {
    let devices = audio::input_devices()?;
    if devices.is_empty() {
        println!("audio: no input devices");
    } else {
        for device in devices {
            let marker = if device.is_default {
                "default"
            } else {
                "available"
            };
            println!("audio: {marker}: {}", device.name);
        }
    }
    Ok(())
}

fn print_help() {
    println!(
        "Typeless IBus voice input engine {VERSION}\n\
         \n\
         Usage: typeless-ibus-engine [OPTION]\n\
         \n\
         Options:\n\
           --ibus                  Run as an IBus engine (default)\n\
           --check                 Check configuration, IBus and microphones\n\
           --list-devices          List microphone devices\n\
           --config-path           Print configuration path\n\
           --print-config          Print the effective configuration\n\
           --write-default-config  Replace configuration with defaults\n\
           -V, --version           Print version\n\
           -h, --help              Print help"
    );
}
