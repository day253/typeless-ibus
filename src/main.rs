mod asr;
mod engine;
mod ibus;

use anyhow::{Context, Result, bail};
use std::path::Path;
use typeless_ibus::config::{Config, ConfigStore, TriggerMode};
use typeless_ibus::{audio, config, properties};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "typeless_ibus=info,typeless_ibus_engine=info".into()),
        )
        .init();

    if let Err(error) = run().await {
        eprintln!("typeless-ibus: {error:#}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let mut arguments = std::env::args().skip(1);
    let argument = arguments.next();
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
        Some("config") => {
            run_config_command(arguments)?;
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
        Some("--check-asr") => {
            let credentials_path = config::credentials_path()?;
            asr::diagnose_service(&credentials_path).await?;
            return Ok(());
        }
        Some("--check-asr-audio") => {
            let audio_path = arguments
                .next()
                .context("--check-asr-audio 需要一个 PCM 音频文件路径")?;
            if let Some(extra) = arguments.next() {
                bail!("--check-asr-audio 收到了多余参数：{extra}");
            }
            let credentials_path = config::credentials_path()?;
            let text = asr::check_audio_fixture(Path::new(&audio_path), &credentials_path).await?;
            println!("asr.audio: recognized {text:?}");
            return Ok(());
        }
        Some("--ibus") | None => {}
        Some(argument) => bail!("未知参数：{argument}；使用 --help 查看帮助"),
    }

    let config_path = config::config_path()?;
    let config = ConfigStore::load(config_path.clone())?;
    let effective = config.snapshot();
    let credentials_path = config::credentials_path()?;
    tracing::info!(
        config = %config_path.display(),
        trigger = %effective.trigger_key,
        mode = ?effective.trigger_mode,
        "starting Typeless IBus engine"
    );
    let _connection = ibus::serve(config, credentials_path).await?;
    tokio::signal::ctrl_c().await?;
    Ok(())
}

fn run_config_command(mut arguments: impl Iterator<Item = String>) -> Result<()> {
    let path = config::config_path()?;
    let command = arguments.next().unwrap_or_else(|| "show".to_string());
    match command.as_str() {
        "show" => {
            reject_extra_argument(&mut arguments)?;
            let config = Config::load_or_create(&path)?;
            println!("{}", serde_json::to_string_pretty(&config)?);
        }
        "reset" => {
            reject_extra_argument(&mut arguments)?;
            Config::default().save(&path)?;
            println!("已恢复默认配置：{}", path.display());
        }
        "set" => {
            let key = arguments.next().context("config set 需要配置名和配置值")?;
            let value = arguments.next().context("config set 需要配置名和配置值")?;
            reject_extra_argument(&mut arguments)?;
            validate_config_assignment(&key, &value)?;
            let store = ConfigStore::load(path.clone())?;
            store.update(|config| match key.as_str() {
                "trigger-key" => config.trigger_key = value.clone(),
                "trigger-mode" => {
                    config.trigger_mode = match value.as_str() {
                        "hold" => TriggerMode::Hold,
                        "toggle" => TriggerMode::Toggle,
                        _ => unreachable!("trigger mode was validated"),
                    }
                }
                "input-device" => {
                    config.input_device = match value.as_str() {
                        "default" | "null" => None,
                        _ => Some(value.clone()),
                    }
                }
                "max-recording-seconds" => {
                    config.max_recording_seconds =
                        value.parse().expect("recording limit was validated");
                }
                _ => unreachable!("configuration key was validated"),
            })?;
            println!("已更新 {key}：{value}");
            println!("切换一次输入源后，高级配置会在新引擎实例中生效。");
        }
        _ => bail!("未知 config 命令：{command}；可用 show、set、reset"),
    }
    Ok(())
}

fn validate_config_assignment(key: &str, value: &str) -> Result<()> {
    match key {
        "trigger-key" => {
            let config = Config {
                trigger_key: value.to_string(),
                ..Config::default()
            };
            config.validate()
        }
        "trigger-mode" if matches!(value, "hold" | "toggle") => Ok(()),
        "trigger-mode" => bail!("trigger-mode 只支持 hold 或 toggle"),
        "input-device" => Ok(()),
        "max-recording-seconds" => {
            let seconds = value
                .parse::<u64>()
                .context("max-recording-seconds 必须是整数")?;
            if (1..=600).contains(&seconds) {
                Ok(())
            } else {
                bail!("max-recording-seconds 必须在 1 到 600 之间")
            }
        }
        _ => bail!(
            "未知配置项：{key}；可用 trigger-key、trigger-mode、input-device、max-recording-seconds"
        ),
    }
}

fn reject_extra_argument(arguments: &mut impl Iterator<Item = String>) -> Result<()> {
    if let Some(extra) = arguments.next() {
        bail!("收到多余参数：{extra}");
    }
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
         Usage: typeless-ibus-engine [OPTION] [PATH]\n\
         \n\
         Options:\n\
           --ibus                  Run as an IBus engine (default)\n\
           config show             Print the effective configuration\n\
           config set KEY VALUE    Update one configuration value\n\
           config reset            Restore the default configuration\n\
           --check                 Check configuration, IBus and microphones\n\
           --check-asr             Diagnose ASR APIs without IBus or audio\n\
           --check-asr-audio PATH  Recognize a 16 kHz mono s16le PCM fixture\n\
           --list-devices          List microphone devices\n\
           --config-path           Print configuration path\n\
           --print-config          Print the effective configuration\n\
           --write-default-config  Replace configuration with defaults\n\
           -V, --version           Print version\n\
           -h, --help              Print help"
    );
}
