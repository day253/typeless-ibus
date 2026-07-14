# Typeless IBus

一个面向 Ubuntu / Linux 的原生 IBus 语音输入法。它直接通过 IBus 的预编辑与提交接口向
当前输入框写入文字，因此可以在 GNOME Wayland 下工作，不依赖剪贴板、模拟粘贴或 X11。

当前版本只包含 Rust 引擎和 GTK4 原生设置程序，不使用 Python，也没有引入 LLM、文字润色、账号或云额度系统。
语音识别协议参考
[`yangmoling/doubaoime-asr`](https://github.com/yangmoling/doubaoime-asr)，桌面产品思路参考
[`tover0314-w/opentypeless`](https://github.com/tover0314-w/opentypeless)。

## 使用方式

1. 在 Ubuntu“设置 → 键盘 → 输入源”中添加 `Typeless Voice`。
2. 切换到 `Typeless Voice` 输入源并把光标放进任意输入框。
3. 长按 `Fn` 开始录音，说完后松开 `Fn`。
4. 识别中的文字显示为预编辑文本，最终结果由 IBus 直接提交。

`Esc` 可以取消当前录音或识别。单次录音默认最多 120 秒。

> 部分笔记本的 Fn 键由固件处理，不会向 Linux 上报 `XF86_Fn`。此时把触发键改成
> `Control_R` 或 `F8` 即可；这不是 Wayland 限制。

## 安装

系统要求：Ubuntu/Debian、IBus 1.5.29+、Rust stable、GTK4、ALSA 和 Opus 开发库。

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libasound2-dev libgtk-4-dev libopus-dev ibus

git clone https://github.com/day253/typeless.git
cd typeless
cargo build --release --locked
cargo install cargo-deb --version 3.7.0 --locked
cargo deb --no-build
sudo apt install ./target/debian/typeless-ibus_0.4.0-1_amd64.deb
```

安装完成后注销并重新登录，或重启 IBus，再从 GNOME 设置添加输入源。

没有 sudo 权限时可安装到当前用户：

```bash
cargo build --release --locked
./packaging/install-user.sh
```

用户级安装脚本会把引擎和组件放到 `~/.local`，并为 GNOME 的用户级 IBus 服务加入组件
搜索路径。它不会改动现有输入源列表；安装后仍需在 GNOME 设置中添加
`Typeless Voice`。卸载使用 `./packaging/uninstall-user.sh`。

## 配置

在 Ubuntu“设置 → 键盘 → 输入源”中选择 `Typeless Voice`，点击设置按钮即可打开原生
GTK4 设置窗口。也可以从应用列表打开“Typeless Voice 设置”。界面支持配置触发键、
长按/切换模式、麦克风和最长录音时间；点击“保存并应用”会自动重新加载 IBus。

用户级安装也可直接运行：

```bash
~/.local/libexec/typeless-ibus-settings
```

设置仍以可读的 JSON 格式保存在：

首次运行会创建：

```text
~/.config/typeless-ibus/config.json
```

默认配置：

```json
{
  "triggerKey": "XF86_Fn",
  "triggerMode": "hold",
  "inputDevice": null,
  "maxRecordingSeconds": 120
}
```

- `triggerKey`：支持 `XF86_Fn`、`Control_R`、`Control_L`、`F8`、`F9`、`F10`、
  `Space` 或 `0x` 开头的十六进制 XKB keysym。
- `triggerMode`：`hold` 表示按下开始、松开结束；`toggle` 表示按一次开始、再按一次结束。
- `inputDevice`：`null` 使用默认麦克风，也可填写设备名称。
- `maxRecordingSeconds`：1 到 600 秒。

手动修改配置后需要重启 IBus。可用以下命令检查：

```bash
/usr/libexec/typeless-ibus-engine --config-path
/usr/libexec/typeless-ibus-engine --print-config
/usr/libexec/typeless-ibus-engine --list-devices
/usr/libexec/typeless-ibus-engine --check
```

系统包中的程序位于 `/usr/libexec/typeless-ibus-*`；用户安装版本位于
`~/.local/libexec/typeless-ibus-*`。

## 架构

```text
物理按键 → IBus ProcessKeyEvent
                  │
          按下触发键 / 松开触发键
                  │
        ALSA + cpal → 16 kHz PCM → Opus
                  │
              豆包 IME ASR
                  │
        IBus preedit / CommitText → 当前输入框
```

主要文件：

```text
src/ibus.rs       IBus D-Bus 组件、Factory 与引擎注册
src/engine.rs     长按/切换触发、录音会话和 IBus 文本提交
src/audio.rs      麦克风采集、单声道混音与重采样
src/asr.rs        设备注册、Token、Protobuf、WebSocket 与 Opus
src/config.rs     JSON 配置、按键解析与本地路径
src/settings.rs   GTK4 原生设置窗口和 IBus 配置重载
```

## 开发与验证

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
cargo build --release --locked
```

GitHub Actions 会在 Ubuntu 24.04 上运行格式、Clippy、测试、发布构建和 `.deb` 打包。

## 数据与风险

- 音频直接发送到豆包输入法相关服务，不经过本项目自己的服务器。
- 首次使用会注册虚拟设备，并把设备标识与 Token 保存到
  `~/.local/share/typeless-ibus/credentials.json`，权限为 `0600`。
- 这是基于非官方协议信息实现的互操作客户端，协议可能变化或停止可用。
- 本项目不复制、不打包也不执行 `doubaoime-asr` 的 Python 源码。
- 使用前请自行确认适用的服务条款、隐私要求和所在地区法规。

请阅读 [第三方说明](THIRD_PARTY.md)。本项目自身代码采用 [MIT License](LICENSE)。
