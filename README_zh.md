[English](README.md) | 中文

# typeless-ibus

typeless-ibus 是一个面向 Ubuntu / Linux 的原生语音输入法。它把语音识别结果通过 IBus
直接写入当前输入框，在 GNOME Wayland 下不依赖剪贴板、模拟粘贴或 X11。

项目专注于一件事：按住一个键说话，松开后在正在使用的应用里得到文字。

## 产品特点

- **原生输入法体验**：识别中的内容显示为预编辑文本，最终结果由 IBus 提交。
- **长按即说**：默认长按 `Fn` 录音、松开结束，也支持切换模式和其他触发键。
- **适用于 Wayland**：通过 IBus D-Bus 接口输入，不模拟键盘和粘贴操作。
- **系统原生配置**：直接从 IBus 输入法菜单切换触发键和录音方式，并根据系统语言显示中文或英文。
- **轻量实现**：引擎使用 Rust，不依赖 GUI 工具包或 Python，不引入 LLM。
- **多发行版支持**：原生 `.deb` 覆盖 Ubuntu 20.04–26.04 和 Debian 11–13；Fedora、openSUSE、Arch Linux 经过协议测试，Nix 包覆盖 x86_64 与 aarch64。
- **自动恢复**：ASR 身份被服务发现拒绝时，会重新获取凭据并重放当前语音。

## 使用体验

1. 在 Ubuntu“设置 → 键盘 → 输入源”中添加 `typeless-ibus`。
2. 切换到 `typeless-ibus`，把光标放进任意输入框。
3. 长按 `Fn` 开始说话。
4. 松开 `Fn`，识别结果直接进入当前输入框。

按 `Esc` 可以取消当前录音或识别。

## 产品范围

当前版本面向带有 IBus 1.5.22 及以上版本的 Linux 发行版。IBus 是唯一输入法后端，
项目不单独实现 GTK、Qt、XIM、Wayland 或 Fcitx5 前端；也不包含 Windows、macOS
客户端、LLM 润色、账号、云额度、历史记录或词典系统。

语音识别协议参考
[`yangmoling/doubaoime-asr`](https://github.com/yangmoling/doubaoime-asr)，产品交互思路参考
[`tover0314-w/opentypeless`](https://github.com/tover0314-w/opentypeless)。

## 文档

- [安装与卸载](docs/installation.md)
- [使用与配置](docs/usage.md)
- [架构与设计](docs/architecture.md)
- [开发与验证](docs/development.md)
- [故障排查](docs/troubleshooting.md)
- [数据、隐私与风险](docs/privacy.md)
- [完整文档索引](docs/README.md)
- [更新日志](CHANGELOG.md)

本项目代码采用 [MIT License](LICENSE)。
