use crate::config::ConfigStore;
use crate::engine::VoiceEngine;
use anyhow::{Context, Result, bail};
use std::env;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use zbus::connection::Builder;
use zbus::names::WellKnownName;
use zbus::zvariant::OwnedObjectPath;
use zbus::{Connection, fdo, interface};

pub const COMPONENT_NAME: &str = "org.freedesktop.IBus.Typeless";
pub const ENGINE_NAME: &str = "typeless";
const FACTORY_PATH: &str = "/org/freedesktop/IBus/Factory";

pub async fn serve(config: ConfigStore, credentials_path: PathBuf) -> Result<Connection> {
    let address = ibus_address()?;
    let connection = Builder::address(address.as_str())
        .context("解析 IBus D-Bus 地址失败")?
        .build()
        .await
        .context("连接 IBus D-Bus 失败")?;

    connection
        .object_server()
        .at(FACTORY_PATH, IBusService)
        .await
        .context("注册 IBus Service 接口失败")?;
    connection
        .object_server()
        .at(
            FACTORY_PATH,
            EngineFactory::new(connection.clone(), config, credentials_path),
        )
        .await
        .context("注册 IBus Factory 接口失败")?;

    let name = WellKnownName::try_from(COMPONENT_NAME).context("IBus 组件名称无效")?;
    connection
        .request_name(name)
        .await
        .context("请求 IBus 组件名称失败")?;
    tracing::info!(component = COMPONENT_NAME, "IBus engine ready");
    Ok(connection)
}

pub fn ibus_address() -> Result<String> {
    if let Ok(address) = env::var("IBUS_ADDRESS")
        && !address.trim().is_empty()
        && address.trim() != "(null)"
    {
        return Ok(address);
    }

    let output = Command::new("ibus")
        .arg("address")
        .output()
        .context("执行 `ibus address` 失败；请确认已安装并启动 IBus")?;
    if !output.status.success() {
        bail!(
            "`ibus address` 返回错误：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    let address = String::from_utf8(output.stdout)
        .context("IBus 地址不是 UTF-8")?
        .trim()
        .to_string();
    if address.is_empty() || address == "(null)" {
        bail!("没有找到当前图形会话的 IBus 地址");
    }
    Ok(address)
}

struct EngineFactory {
    connection: Connection,
    config: ConfigStore,
    credentials_path: PathBuf,
    next_engine_id: AtomicU64,
}

impl EngineFactory {
    fn new(connection: Connection, config: ConfigStore, credentials_path: PathBuf) -> Self {
        Self {
            connection,
            config,
            credentials_path,
            next_engine_id: AtomicU64::new(1),
        }
    }
}

#[interface(name = "org.freedesktop.IBus.Factory")]
impl EngineFactory {
    async fn create_engine(&self, name: String) -> fdo::Result<OwnedObjectPath> {
        if name != ENGINE_NAME {
            return Err(fdo::Error::Failed(format!("未知输入法引擎：{name}")));
        }

        let id = self.next_engine_id.fetch_add(1, Ordering::Relaxed);
        let path_string = format!("/org/freedesktop/IBus/Engine/Typeless/{id}");
        let path = OwnedObjectPath::try_from(path_string.clone())
            .map_err(|error| fdo::Error::Failed(error.to_string()))?;

        if let Err(error) = self.config.reload() {
            tracing::warn!(%error, "failed to reload configuration; keeping last valid values");
        }

        self.connection
            .object_server()
            .at(
                path_string.as_str(),
                VoiceEngine::new(self.config.clone(), self.credentials_path.clone()),
            )
            .await
            .map_err(|error| fdo::Error::Failed(error.to_string()))?;
        self.connection
            .object_server()
            .at(path_string.as_str(), IBusService)
            .await
            .map_err(|error| fdo::Error::Failed(error.to_string()))?;

        tracing::info!(engine_path = %path_string, "created IBus engine instance");
        Ok(path)
    }
}

struct IBusService;

#[interface(name = "org.freedesktop.IBus.Service")]
impl IBusService {
    fn destroy(&self) {
        tracing::debug!("IBus requested object destruction");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_names_match_registration_xml() {
        let xml = include_str!("../data/typeless.xml");
        assert!(xml.contains(&format!("<name>{COMPONENT_NAME}</name>")));
        assert!(xml.contains(&format!("<name>{ENGINE_NAME}</name>")));
        assert!(xml.contains(&format!("<version>{}</version>", env!("CARGO_PKG_VERSION"))));
        assert!(xml.contains("<symbol>语</symbol>"));
        assert!(xml.contains("/usr/libexec/typeless-ibus-engine"));
        assert!(!xml.contains("<setup>"));
        assert!(!xml.contains("<icon>"));
    }
}
