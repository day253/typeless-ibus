mod asr;
mod audio;
mod config;
mod output;

use audio::AudioCaptureHandle;
use config::AppConfig;
use serde::Serialize;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Mutex;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

struct AppState {
    config: Mutex<AppConfig>,
    capture: Mutex<Option<AudioCaptureHandle>>,
    status: Mutex<StatusEvent>,
    config_path: PathBuf,
    credentials_path: PathBuf,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum PipelineState {
    Idle,
    Recording,
    Processing,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusEvent {
    state: PipelineState,
    message: String,
}

impl Default for StatusEvent {
    fn default() -> Self {
        Self {
            state: PipelineState::Idle,
            message: "就绪".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FinalEvent {
    text: String,
    inserted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlatformInfo {
    os: &'static str,
    session_type: &'static str,
    supports_global_shortcut: bool,
    supports_auto_paste: bool,
}

fn platform_info() -> PlatformInfo {
    #[cfg(target_os = "macos")]
    {
        PlatformInfo {
            os: "macos",
            session_type: "macos",
            supports_global_shortcut: true,
            supports_auto_paste: true,
        }
    }
    #[cfg(target_os = "linux")]
    {
        let wayland = output::is_wayland();
        PlatformInfo {
            os: "linux",
            session_type: if wayland { "wayland" } else { "x11" },
            supports_global_shortcut: !wayland,
            supports_auto_paste: !wayland,
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    {
        PlatformInfo {
            os: "linux",
            session_type: "unknown",
            supports_global_shortcut: false,
            supports_auto_paste: false,
        }
    }
}

#[tauri::command]
fn get_config(state: State<'_, AppState>) -> AppConfig {
    state
        .config
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

#[tauri::command]
fn get_pipeline_status(state: State<'_, AppState>) -> StatusEvent {
    state
        .status
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone()
}

#[tauri::command]
fn get_platform_info() -> PlatformInfo {
    platform_info()
}

#[tauri::command]
fn list_input_devices() -> Result<Vec<audio::AudioDeviceInfo>, String> {
    audio::input_devices().map_err(|error| format!("{error:#}"))
}

#[tauri::command]
fn save_config(
    app: AppHandle,
    state: State<'_, AppState>,
    config: AppConfig,
) -> Result<AppConfig, String> {
    validate_hotkey(&config.hotkey)?;
    let previous = state
        .config
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();

    if let Err(error) = register_hotkey(&app, &config.hotkey) {
        let _ = register_hotkey(&app, &previous.hotkey);
        return Err(error);
    }
    if let Err(error) = config::save(&state.config_path, &config) {
        let _ = register_hotkey(&app, &previous.hotkey);
        return Err(format!("{error:#}"));
    }
    *state
        .config
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = config.clone();
    Ok(config)
}

#[tauri::command]
async fn toggle_recording(app: AppHandle) -> Result<(), String> {
    toggle_recording_internal(app).await
}

async fn toggle_recording_internal(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let current = state
        .status
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .state;
    match current {
        PipelineState::Idle | PipelineState::Error => start_recording(app),
        PipelineState::Recording => stop_recording(&app),
        PipelineState::Processing => Err("正在完成上一段识别，请稍候".to_string()),
    }
}

fn start_recording(app: AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let app_config = state
        .config
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .clone();
    let (capture, audio_rx) = AudioCaptureHandle::start(app_config.input_device.as_deref())
        .map_err(|error| format!("无法启动麦克风：{error:#}"))?;
    *state
        .capture
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = Some(capture);

    set_status(
        &app,
        PipelineState::Recording,
        "正在聆听，再按一次快捷键结束",
    );
    if let Some(capsule) = app.get_webview_window("capsule") {
        let _ = capsule.show();
    }

    let credentials_path = state.credentials_path.clone();
    tauri::async_runtime::spawn(run_dictation(
        app.clone(),
        audio_rx,
        app_config,
        credentials_path,
    ));
    Ok(())
}

fn stop_recording(app: &AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut capture = state
        .capture
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take()
        .ok_or_else(|| "当前没有正在进行的录音".to_string())?;
    set_status(app, PipelineState::Processing, "正在完成识别…");
    capture.stop();
    Ok(())
}

async fn run_dictation(
    app: AppHandle,
    audio_rx: tokio::sync::mpsc::Receiver<Vec<u8>>,
    app_config: AppConfig,
    credentials_path: PathBuf,
) {
    let event_app = app.clone();
    let result = asr::transcribe_realtime(audio_rx, &credentials_path, move |event| match event {
        asr::AsrEvent::SpeechStarted => {
            set_status(&event_app, PipelineState::Recording, "检测到语音");
        }
        asr::AsrEvent::Partial(text) | asr::AsrEvent::Final(text) => {
            let _ = event_app.emit("dictation://partial", text);
        }
    })
    .await;

    if let Some(mut capture) = app
        .state::<AppState>()
        .capture
        .lock()
        .unwrap_or_else(|error| error.into_inner())
        .take()
    {
        capture.stop();
    }

    match result {
        Ok(text) if !text.trim().is_empty() => {
            let output_text = text.trim().to_string();
            let output_config = app_config.clone();
            let delivery = tauri::async_runtime::spawn_blocking(move || {
                output::deliver(&output_text, &output_config).map(|outcome| (output_text, outcome))
            })
            .await;
            match delivery {
                Ok(Ok((text, outcome))) => {
                    let _ = app.emit(
                        "dictation://final",
                        FinalEvent {
                            text,
                            inserted: outcome.inserted,
                        },
                    );
                    set_status(
                        &app,
                        PipelineState::Idle,
                        if outcome.inserted {
                            "文字已输入"
                        } else {
                            "文字已复制到剪贴板"
                        },
                    );
                }
                Ok(Err(error)) => report_error(&app, format!("输出文字失败：{error:#}")),
                Err(error) => report_error(&app, format!("输出任务异常：{error}")),
            }
        }
        Ok(_) => report_error(&app, "没有识别到语音，请重试".to_string()),
        Err(error) => report_error(&app, format!("语音识别失败：{error:#}")),
    }

    if let Some(capsule) = app.get_webview_window("capsule") {
        let _ = capsule.hide();
    }
}

fn report_error(app: &AppHandle, message: String) {
    let _ = app.emit("dictation://error", message.clone());
    set_status(app, PipelineState::Error, &message);
}

fn set_status(app: &AppHandle, state_value: PipelineState, message: &str) {
    let status = StatusEvent {
        state: state_value,
        message: message.to_string(),
    };
    *app.state::<AppState>()
        .status
        .lock()
        .unwrap_or_else(|error| error.into_inner()) = status.clone();
    let _ = app.emit("dictation://state", status);
}

fn validate_hotkey(value: &str) -> Result<Shortcut, String> {
    Shortcut::from_str(value).map_err(|error| format!("快捷键格式无效：{error}"))
}

fn register_hotkey(app: &AppHandle, value: &str) -> Result<(), String> {
    if !platform_info().supports_global_shortcut {
        return Ok(());
    }
    let shortcut = validate_hotkey(value)?;
    app.global_shortcut()
        .unregister_all()
        .map_err(|error| format!("清理旧快捷键失败：{error}"))?;
    app.global_shortcut()
        .register(shortcut)
        .map_err(|error| format!("注册快捷键失败：{error}"))
}

fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "显示设置", true, None::<&str>)?;
    let record = MenuItem::with_id(app, "record", "开始 / 结束录音", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &record, &separator, &quit])?;
    TrayIconBuilder::new()
        .icon(
            app.default_window_icon()
                .expect("default window icon is required")
                .clone(),
        )
        .tooltip("Typeless ASR")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "show" => show_main_window(app),
            "record" => {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = toggle_recording_internal(handle.clone()).await {
                        report_error(&handle, error);
                    }
                });
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "typeless_asr=info".into()),
        )
        .init();

    tauri::Builder::default()
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state == ShortcutState::Pressed {
                        let handle = app.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(error) = toggle_recording_internal(handle.clone()).await {
                                report_error(&handle, error);
                            }
                        });
                    }
                })
                .build(),
        )
        .setup(|app| {
            let config_dir = app.path().app_config_dir()?;
            let data_dir = app.path().app_data_dir()?;
            let config_path = config_dir.join("config.json");
            let initial_config = config::load(&config_path);
            app.manage(AppState {
                config: Mutex::new(initial_config.clone()),
                capture: Mutex::new(None),
                status: Mutex::new(StatusEvent::default()),
                config_path,
                credentials_path: data_dir.join("credentials.json"),
            });

            if let Err(error) = register_hotkey(app.handle(), &initial_config.hotkey) {
                tracing::warn!("global shortcut unavailable: {error}");
            }
            build_tray(app.handle())?;

            if let Some(window) = app.get_webview_window("main") {
                let window_to_hide = window.clone();
                window.on_window_event(move |event| {
                    if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = window_to_hide.hide();
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_config,
            get_pipeline_status,
            get_platform_info,
            list_input_devices,
            save_config,
            toggle_recording
        ])
        .run(tauri::generate_context!())
        .expect("failed to run Typeless ASR");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_hotkey_is_parseable() {
        assert!(validate_hotkey(&AppConfig::default().hotkey).is_ok());
    }
}
