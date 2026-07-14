# 发行版支持范围

typeless-ibus 的运行条件是 Linux、IBus 1.5.22 或更高版本，以及可用的 ALSA 输入设备。
项目只实现一个 Rust IBus 引擎；GTK、Qt、XIM 和 Wayland 应用由发行版提供的 IBus
集成模块覆盖。GNOME Wayland 是主要桌面验证环境，但产品不限定 Ubuntu 或 GNOME。

## 已验证范围

| 发行版 | 已验证版本 | 推荐安装方式 | CI 验证 |
| --- | --- | --- | --- |
| Ubuntu | 20.04、22.04、24.04、26.04 | 对应版本的原生 `.deb` | 原生构建、打包、IBus 协议 |
| Debian | 11、12、13 | 对应版本的原生 `.deb` | 原生构建、打包、IBus 协议 |
| Fedora | 43、44 | Nix Flake 或源码安装 | 原生构建、IBus 协议 |
| openSUSE | Tumbleweed | Nix Flake 或源码安装 | 原生构建、IBus 协议 |
| Arch Linux | rolling | Nix Flake 或源码安装 | 原生构建、IBus 协议 |
| 支持 Nix 的 Linux | x86_64-linux、aarch64-linux | Nix Flake | 两个架构原生构建与包布局 |

这里的 IBus 协议验证会实际启动隔离的 D-Bus 会话，调用 Factory、`CreateEngine` 和
`ProcessKeyEvent`，并检查输入法属性注册。它验证引擎与该发行版 IBus 的接口兼容性，
不等于测试了每一种桌面环境、应用和硬件组合。

## 衍生发行版和其他 IBus 系统

Linux Mint、Pop!_OS 等 Ubuntu 衍生版可优先选择与其 Ubuntu 基础版本一致的 `.deb`。
这些组合属于继承兼容，尚未作为独立 CI 条目逐个验证。

其他提供 IBus 1.5.22+ 的 Linux 发行版通常可以使用 Nix Flake，或安装 Rust、ALSA、
Opus 和 IBus 开发依赖后执行用户级源码安装。Nix 包支持 x86_64-linux 和
aarch64-linux，但桌面会话仍需由宿主发行版提供 IBus 及其应用集成模块。

项目当前不维护 RPM、PKGBUILD 或其他发行版专用包，也不实现 Fcitx5 以及独立的 GTK、
Qt、XIM 或 Wayland 前端。这样可以让运行时代码和配置保持为轻量的 Rust + IBus 实现。

具体命令见[安装与卸载](installation.md)，测试矩阵见[开发与验证](development.md)。

[返回文档索引](README.md)
