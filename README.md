# Typeless ASR

一个面向 macOS 和 Linux 的轻量语音输入桌面应用。按一次快捷键开始说话，再按一次结束；
识别结果会直接粘贴到当前应用，或在受限制的平台上复制到剪贴板。

当前版本使用 **Tauri 2 + Rust + React/TypeScript**。运行时不需要 Python，也不包含 LLM、
文字润色、账户或云额度系统。

## 当前能力

- Rust `cpal` 实时采集麦克风，统一转换为 16 kHz 单声道 PCM
- Rust 实现豆包输入法 ASR WebSocket 客户端、Protobuf 消息和 Opus 编码
- Tauri 系统托盘、全局快捷键和无焦点录音胶囊
- React/TypeScript 设置界面与实时中间识别结果
- macOS / Linux X11 自动粘贴，并在安全时恢复原剪贴板文字
- Linux Wayland 自动降级到托盘控制和剪贴板输出
- 可配置快捷键和麦克风设备
- 凭据和设置只保存在本机 Tauri 应用数据目录

Windows 暂不支持。

## 技术架构

项目参考了 [OpenTypeless](https://github.com/tover0314-w/opentypeless) 的 Tauri 桌面分层，
但只保留语音输入所需的最小链路：

```text
React + TypeScript
设置 · 状态 · 中间识别结果
          │ Tauri commands / events
          ▼
Rust desktop core
全局快捷键 · 托盘 · cpal 录音 · Opus · 豆包 ASR · 剪贴板/粘贴
```

核心目录：

```text
src/                         # React / TypeScript 前端
src-tauri/src/audio.rs       # 麦克风采集、混音和重采样
src-tauri/src/asr.rs         # 豆包设备注册、Token、Protobuf、WebSocket、Opus
src-tauri/src/output.rs      # 剪贴板、自动粘贴和 Wayland 降级
src-tauri/src/config.rs      # 本地设置
src-tauri/src/lib.rs         # Tauri 命令、事件、快捷键、托盘和流水线
```

## 安装依赖

需要：

- Node.js 22+
- Rust stable
- Opus
- macOS 或 Linux 桌面环境

### macOS

```bash
brew install node rust opus pkg-config
git clone https://github.com/day253/typeless.git
cd typeless
npm install
npm run tauri dev
```

第一次录音时，在“系统设置 → 隐私与安全性”中为 Typeless ASR 开启：

1. 麦克风
2. 辅助功能（全局快捷键和自动粘贴需要）

构建 `.app` 和 `.dmg`：

```bash
npm run tauri build
```

### Ubuntu / Debian

```bash
sudo apt update
sudo apt install -y \
  build-essential curl wget file pkg-config libopus-dev libasound2-dev \
  libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libxdo-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
git clone https://github.com/day253/typeless.git
cd typeless
npm install
npm run tauri dev
```

构建 AppImage 和 `.deb`：

```bash
npm run tauri build
```

## 使用

1. 启动 Typeless ASR。
2. 把光标放到任意应用的输入框。
3. 按 `Ctrl+Shift+Space` 开始录音。
4. 说完后再次按相同快捷键。
5. 识别结果会自动输入；不能模拟粘贴时会保留在剪贴板。

关闭主窗口只会隐藏到系统托盘，选择托盘菜单中的“退出”才会完全结束。

### Wayland

Wayland 默认禁止普通应用监听系统级按键和模拟键盘输入。本项目不会绕过该安全模型：

- 使用托盘菜单“开始 / 结束录音”控制录音
- 识别结果复制到剪贴板
- 手动按 `Ctrl+V` 完成输入

X11 下会正常注册全局快捷键并自动粘贴。

## 开发与验证

```bash
npm install
npm run build

cargo fmt --manifest-path src-tauri/Cargo.toml -- --check
cargo clippy --manifest-path src-tauri/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path src-tauri/Cargo.toml
```

GitHub CI 会在 macOS 14 和 Ubuntu 24.04 上验证 TypeScript 构建、Rust 格式、Clippy 和测试。

## 数据与风险说明

- 音频直接发送到豆包输入法相关服务，不经过本项目自己的服务器。
- ASR 是根据非官方客户端所展示协议实现的互操作客户端，协议可能变化或随时不可用。
- 首次使用会注册虚拟设备，并将设备标识与 Token 保存到本机应用数据目录。
- 本项目没有复制或打包 `doubaoime-asr` 的 Python 源码，也不依赖 Python 运行时。
- 使用前请自行确认适用的服务条款、隐私要求和所在地区法规。

请阅读 [第三方说明](THIRD_PARTY.md)。本项目自身代码采用 [MIT License](LICENSE)。
