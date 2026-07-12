# Typeless ASR

一个面向 macOS 和 Linux 的轻量语音输入工具。按一次快捷键开始说话，再按一次结束；
识别结果会直接粘贴到当前应用，或在受限制的平台上复制到剪贴板。

首版刻意只做一件事：**语音 → 文字输入**。不接入 LLM，不润色、不改写，也没有账户、
历史记录或云额度系统。

## 当前能力

- 基于 [`yangmoling/doubaoime-asr`](https://github.com/yangmoling/doubaoime-asr)
  的实时中文语音识别
- macOS 与 Linux 共用同一套录音和识别核心
- 系统托盘常驻、录音状态浮层和实时中间结果
- 可配置全局快捷键，默认 `Ctrl+Shift+Space`
- 自动粘贴，并在安全时恢复原剪贴板文字
- 可选择麦克风设备
- 本地 Unix Socket 控制命令，兼容 Wayland 的桌面快捷键方案
- 配置与 ASR 凭据只保存在本机用户配置目录

## 平台支持

| 平台 | 开始/停止录音 | 结果输出 | 备注 |
| --- | --- | --- | --- |
| macOS | 原生全局快捷键 / 托盘 | 自动粘贴 | 首次运行需要麦克风和辅助功能权限 |
| Linux X11 | 全局快捷键 / 托盘 | 自动粘贴 | 需要可用的系统托盘 |
| Linux Wayland | 桌面快捷键调用控制命令 / 托盘 | 剪贴板 | Wayland 默认禁止应用监听全局按键和模拟粘贴 |
| Windows | 暂不支持 | 暂不支持 | 当前不在项目范围内 |

## 安装

需要 Python 3.11 或更高版本、Git、PortAudio 和 Opus。

### macOS

```bash
brew install python@3.12 portaudio opus
git clone https://github.com/day253/typeless.git
cd typeless
python3.12 -m venv .venv
source .venv/bin/activate
pip install -e .
typeless-asr
```

第一次录音时，在“系统设置 → 隐私与安全性”中为终端或打包后的应用开启：

1. 麦克风
2. 辅助功能（全局快捷键和自动粘贴需要）

### Ubuntu / Debian

```bash
sudo apt install python3-venv git libopus0 libportaudio2
git clone https://github.com/day253/typeless.git
cd typeless
python3 -m venv .venv
source .venv/bin/activate
pip install -e .
typeless-asr
```

### Arch Linux

```bash
sudo pacman -S python git opus portaudio
git clone https://github.com/day253/typeless.git
cd typeless
python -m venv .venv
source .venv/bin/activate
pip install -e .
typeless-asr
```

## 使用

1. 运行 `typeless-asr`，应用会进入系统托盘。
2. 把光标放到任意输入框。
3. 按 `Ctrl+Shift+Space` 开始录音。
4. 说完后再次按相同快捷键。
5. 识别结果会自动输入；失败时会保留在剪贴板。

关闭设置窗口不会退出应用。要完全退出，请使用托盘菜单中的“退出”，或运行：

```bash
typeless-asr --quit
```

### Wayland

Wayland 下应用不会绕过桌面安全模型。请先保持 `typeless-asr` 运行，然后在 GNOME、KDE
或其他桌面环境的“自定义快捷键”中，将你喜欢的按键绑定到：

```bash
/absolute/path/to/.venv/bin/typeless-asr --toggle
```

按一次开始、再按一次结束。结果会进入剪贴板，随后手动按 `Ctrl+V`。可用控制命令：

```text
typeless-asr --start
typeless-asr --stop
typeless-asr --toggle
typeless-asr --show
typeless-asr --quit
```

## 开发

```bash
python3 -m venv .venv
source .venv/bin/activate
pip install -e '.[dev]'
ruff check .
pytest
typeless-asr
```

CI 会在 macOS 14 和 Ubuntu 24.04、Python 3.11/3.12 上运行静态检查与单元测试。

本地实验性打包：

```bash
pip install -e '.[build]'
python scripts/build.py
```

请先阅读 [第三方说明](THIRD_PARTY.md)：ASR 上游在当前固定版本没有提供许可证，因此项目
不会自动发布包含该依赖的二进制包。

## 架构

```text
全局快捷键 / 托盘 / Wayland 控制命令
                  │
                  ▼
       16 kHz 单声道 PCM 录音
                  │
                  ▼
        doubaoime-asr 实时识别
                  │
                  ▼
   macOS/X11 自动粘贴 · Wayland 剪贴板
```

核心模块：

- `worker.py`：音频采集、异步队列和实时 ASR 会话
- `hotkey.py`：快捷键校验与 macOS/X11 全局监听
- `ipc.py`：Wayland 和命令行控制使用的本地 Socket
- `output.py`：剪贴板、安全恢复和平台粘贴策略
- `app.py` / `ui.py`：托盘、设置窗口和录音状态

## 数据与风险说明

- 音频会直接发送到豆包输入法相关服务，不经过本项目自己的服务器。
- `doubaoime-asr` 是非官方客户端，协议可能变化，服务也可能随时不可用。
- 首次运行会自动注册虚拟设备，并把凭据写入本机配置目录。
- 使用前请自行确认适用的服务条款、隐私要求和所在地区法规。

本项目自身代码采用 [MIT License](LICENSE)。第三方组件适用各自条款。
