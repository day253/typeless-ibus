# 发行版支持范围

typeless-ibus 的运行条件是 Linux、IBus 1.5.22 或更高版本，以及可用的 ALSA 输入设备。
项目只实现一个 Rust IBus 引擎；GTK、Qt、XIM 和 Wayland 应用由发行版提供的 IBus
集成模块覆盖。GNOME Wayland 是主要桌面验证环境，但产品不限定 Ubuntu 或 GNOME。

## 已验证范围

| 发行版 | 已验证版本 | 原生包架构 | 推荐安装方式 | CI 验证 |
| --- | --- | --- | --- | --- |
| Ubuntu | 20.04、22.04、24.04、26.04 | amd64、arm64 | 对应版本的原生 `.deb` | 原生构建、打包、IBus 协议 |
| Debian | 11 Bullseye、12 Bookworm、13 Trixie | amd64、arm64 | 对应代号的原生 `.deb` | 原生构建、打包、IBus 协议 |
| Fedora | 43、44 | x86_64、aarch64 | 对应版本的原生 `.rpm` | 原生构建、打包、安装、IBus 协议、卸载 |
| openSUSE | Tumbleweed | x86_64、aarch64 | Tumbleweed 原生 `.rpm` | 原生构建、打包、安装、IBus 协议、卸载 |
| Arch Linux | rolling | — | Nix Flake 或源码安装 | x86_64 原生构建、IBus 协议 |
| 支持 Nix 的 Linux | x86_64-linux、aarch64-linux | x86_64、aarch64 | Nix Flake | 两个架构原生构建与包布局 |

这里的 IBus 协议验证会实际启动隔离的 D-Bus 会话，调用 Factory、`CreateEngine` 和
`ProcessKeyEvent`，并检查输入法属性注册。它验证引擎与该发行版 IBus 的接口兼容性，
不等于测试了每一种桌面环境、应用和硬件组合。

## 衍生发行版和其他 IBus 系统

Linux Mint、Pop!_OS 等 Ubuntu 衍生版可优先选择与其 Ubuntu 基础版本一致的 `.deb`。
这些组合属于继承兼容，尚未作为独立 CI 条目逐个验证。

Fedora 和 openSUSE 用户应选择与发行版、版本及 CPU 架构对应的 RPM，不能把某个 Fedora
版本构建的 RPM 当作通用 Linux 包安装。二进制包原生覆盖 x86_64 和 aarch64；每个发行版
另提供一份带有完整 vendored Rust 依赖的 SRPM，便于其他 RPM 系发行版重建和适配。

Debian 的官方代号分别是 11 Bullseye、12 Bookworm 和 13 Trixie；Buster 对应已归档的
Debian 10，不在当前 IBus 1.5.22+ 支持范围内。Release 文件名同时保留版本号和代号，
例如 `debian-12-bookworm`，以免只看代号时选错包。

其他提供 IBus 1.5.22+ 的 Linux 发行版通常可以使用 Nix Flake，或安装 Rust、ALSA、
Opus 和 IBus 开发依赖后执行用户级源码安装。Nix 包支持 x86_64-linux 和
aarch64-linux，但桌面会话仍需由宿主发行版提供 IBus 及其应用集成模块。

项目当前不维护 PKGBUILD 或 Fcitx5，也不实现独立的 GTK、Qt、XIM 或 Wayland 前端。
`.deb`、`.rpm`、Nix 和用户级安装都使用同一个 Rust IBus 引擎，没有新增运行时框架。

具体命令见[安装与卸载](installation.md)，测试矩阵见[开发与验证](development.md)。

[返回文档索引](README.md)
