use anyhow::{Context, Result, anyhow};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, SampleFormat, Stream, StreamConfig};
use serde::Serialize;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

const TARGET_SAMPLE_RATE: u32 = 16_000;
const FRAME_DURATION_MS: u32 = 20;
const SAMPLES_PER_FRAME: usize = (TARGET_SAMPLE_RATE * FRAME_DURATION_MS / 1000) as usize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub fn input_devices() -> Result<Vec<AudioDeviceInfo>> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.name().ok());
    let devices = host
        .input_devices()
        .context("读取麦克风列表失败")?
        .filter_map(|device| device.name().ok())
        .map(|name| AudioDeviceInfo {
            id: name.clone(),
            is_default: default_name.as_deref() == Some(name.as_str()),
            name,
        })
        .collect();
    Ok(devices)
}

pub struct AudioCaptureHandle {
    stop_tx: Option<std::sync::mpsc::Sender<()>>,
    device_name: String,
}

impl AudioCaptureHandle {
    pub fn device_name(&self) -> &str {
        &self.device_name
    }
}

impl AudioCaptureHandle {
    pub fn start(device_name: Option<&str>) -> Result<(Self, mpsc::Receiver<Vec<u8>>)> {
        let requested_device = device_name.map(str::to_owned);
        let (audio_tx, audio_rx) = mpsc::channel(200);
        let (stop_tx, stop_rx) = std::sync::mpsc::channel();
        let (ready_tx, ready_rx) = std::sync::mpsc::sync_channel(1);

        std::thread::spawn(move || {
            let result = run_capture(requested_device.as_deref(), audio_tx, stop_rx, ready_tx);
            if let Err(error) = &result {
                tracing::error!("audio capture failed: {error:#}");
            }
        });

        let device_name = ready_rx
            .recv_timeout(Duration::from_secs(5))
            .context("等待麦克风启动超时")?;
        let device_name = device_name.map_err(anyhow::Error::msg)?;
        Ok((
            Self {
                stop_tx: Some(stop_tx),
                device_name,
            },
            audio_rx,
        ))
    }

    pub fn stop(&mut self) {
        if let Some(sender) = self.stop_tx.take() {
            let _ = sender.send(());
        }
    }
}

impl Drop for AudioCaptureHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

fn find_device(name: Option<&str>) -> Result<Device> {
    let host = cpal::default_host();
    if let Some(name) = name {
        return host
            .input_devices()
            .context("读取麦克风列表失败")?
            .find(|device| device.name().ok().as_deref() == Some(name))
            .ok_or_else(|| anyhow!("找不到麦克风：{name}"));
    }
    host.default_input_device()
        .ok_or_else(|| anyhow!("没有可用的默认麦克风"))
}

fn run_capture(
    device_name: Option<&str>,
    sender: mpsc::Sender<Vec<u8>>,
    stop_rx: std::sync::mpsc::Receiver<()>,
    ready_tx: std::sync::mpsc::SyncSender<Result<String, String>>,
) -> Result<()> {
    let device = match find_device(device_name) {
        Ok(device) => device,
        Err(error) => {
            let _ = ready_tx.send(Err(format!("{error:#}")));
            return Err(error);
        }
    };
    let actual_device_name = device.name().unwrap_or_else(|_| "默认输入设备".to_string());
    let supported = match device.default_input_config() {
        Ok(config) => config,
        Err(error) => {
            let _ = ready_tx.send(Err(format!("{error:#}")));
            return Err(error).context("读取麦克风格式失败");
        }
    };
    let sample_format = supported.sample_format();
    let source_rate = supported.sample_rate().0;
    let source_channels = supported.channels();
    let config: StreamConfig = supported.into();
    let buffer = Arc::new(Mutex::new(Vec::<i16>::with_capacity(SAMPLES_PER_FRAME * 2)));

    let stream = match build_stream(
        &device,
        &config,
        sample_format,
        source_rate,
        source_channels,
        sender,
        buffer,
    ) {
        Ok(stream) => stream,
        Err(error) => {
            let _ = ready_tx.send(Err(format!("{error:#}")));
            return Err(error);
        }
    };

    if let Err(error) = stream.play().context("启动麦克风失败") {
        let _ = ready_tx.send(Err(format!("{error:#}")));
        return Err(error);
    }
    let _ = ready_tx.send(Ok(actual_device_name));
    let _ = stop_rx.recv();
    drop(stream);
    Ok(())
}

pub fn is_no_input_device_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        let message = cause.to_string().to_ascii_lowercase();
        message.contains("没有可用的默认麦克风")
            || message.contains("找不到麦克风")
            || message.contains("no input device")
            || message.contains("no default input device")
            || message.contains("input device not found")
    })
}

fn build_stream(
    device: &Device,
    config: &StreamConfig,
    sample_format: SampleFormat,
    source_rate: u32,
    source_channels: u16,
    sender: mpsc::Sender<Vec<u8>>,
    buffer: Arc<Mutex<Vec<i16>>>,
) -> Result<Stream> {
    let error_callback = |error| tracing::error!("audio stream error: {error}");
    let stream = match sample_format {
        SampleFormat::F32 => {
            let buffer = buffer.clone();
            let sender = sender.clone();
            device.build_input_stream(
                config,
                move |data: &[f32], _| {
                    process_samples(data, source_rate, source_channels, &sender, &buffer)
                },
                error_callback,
                None,
            )?
        }
        SampleFormat::I16 => {
            let buffer = buffer.clone();
            let sender = sender.clone();
            device.build_input_stream(
                config,
                move |data: &[i16], _| {
                    let converted: Vec<f32> =
                        data.iter().map(|sample| *sample as f32 / 32768.0).collect();
                    process_samples(&converted, source_rate, source_channels, &sender, &buffer);
                },
                error_callback,
                None,
            )?
        }
        SampleFormat::U16 => device.build_input_stream(
            config,
            move |data: &[u16], _| {
                let converted: Vec<f32> = data
                    .iter()
                    .map(|sample| (*sample as f32 - 32768.0) / 32768.0)
                    .collect();
                process_samples(&converted, source_rate, source_channels, &sender, &buffer);
            },
            error_callback,
            None,
        )?,
        other => return Err(anyhow!("不支持的麦克风采样格式：{other:?}")),
    };
    Ok(stream)
}

fn process_samples(
    data: &[f32],
    source_rate: u32,
    source_channels: u16,
    sender: &mpsc::Sender<Vec<u8>>,
    buffer: &Arc<Mutex<Vec<i16>>>,
) {
    if data.is_empty() {
        return;
    }
    let channels = source_channels.max(1) as usize;
    let mono: Vec<f32> = data
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect();
    let ratio = source_rate as f64 / TARGET_SAMPLE_RATE as f64;
    let output_len = (mono.len() as f64 / ratio) as usize;

    let mut pcm = buffer.lock().unwrap_or_else(|error| error.into_inner());
    for output_index in 0..output_len {
        let source_index = output_index as f64 * ratio;
        let index = source_index.floor() as usize;
        let fraction = source_index - index as f64;
        let current = mono[index.min(mono.len() - 1)] as f64;
        let next = mono[(index + 1).min(mono.len() - 1)] as f64;
        let sample = current * (1.0 - fraction) + next * fraction;
        pcm.push((sample * i16::MAX as f64).clamp(i16::MIN as f64, i16::MAX as f64) as i16);
    }

    while pcm.len() >= SAMPLES_PER_FRAME {
        let frame: Vec<i16> = pcm.drain(..SAMPLES_PER_FRAME).collect();
        let bytes = frame
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect();
        let _ = sender.try_send(bytes);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_missing_input_devices() {
        assert!(is_no_input_device_error(&anyhow!("没有可用的默认麦克风")));
        assert!(is_no_input_device_error(&anyhow!(
            "No default input device"
        )));
        assert!(!is_no_input_device_error(&anyhow!("启动麦克风失败")));
    }

    #[test]
    fn reports_an_unavailable_selected_device_without_waiting_for_audio() {
        let result = AudioCaptureHandle::start(Some("__typeless_missing_input_device__"));
        let error = match result {
            Ok(_) => panic!("an intentionally missing input device unexpectedly opened"),
            Err(error) => error,
        };
        assert!(is_no_input_device_error(&error));
    }
}
