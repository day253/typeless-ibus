# 安装与卸载

## 系统要求

- Ubuntu 或 Debian
- IBus 1.5.22+
- Rust stable
- ALSA 和 Opus 开发库

CI 会分别在 Ubuntu 20.04、22.04、24.04 和 26.04 的官方用户空间中构建 `.deb`，
并实际调用当前发行版的 IBus Factory、`CreateEngine` 和 `ProcessKeyEvent` 接口。
请选择与目标 Ubuntu 版本一致的 `.deb`，避免 glibc 和 ALSA ABI 不匹配。所有版本
都只包含 Rust IBus 引擎，不再区分 GTK 设置版和无界面版。

## 构建 Debian 包

```bash
sudo apt update
sudo apt install -y build-essential pkg-config libasound2-dev libopus-dev ibus

git clone https://github.com/day253/typeless-ibus.git
cd typeless-ibus
cargo build --release --locked
cargo install cargo-deb --version 3.7.0 --locked
cargo deb --no-build
sudo apt install ./target/debian/typeless-ibus_*.deb
```

安装完成后注销并重新登录，或重新启动 IBus。随后在 Ubuntu“设置 → 键盘 → 输入源”
中添加 `typeless-ibus`。

## Nix Flake

Nix 包原生支持 `x86_64-linux` 和 `aarch64-linux`：

```bash
nix profile install github:day253/typeless-ibus
```

安装后重新登录，让图形会话刷新 `$XDG_DATA_DIRS` 和 IBus 组件缓存，然后添加
`typeless-ibus` 输入源。Nix 会封装 Rust 引擎所需的用户空间库，但桌面会话仍需要
发行版提供 IBus 1.5.22 或更高版本。

## 用户级安装

没有 sudo 权限时，可以安装到当前用户：

```bash
cargo build --release --locked
./packaging/install-user.sh
```

安装脚本会把引擎和组件放到 `~/.local`，并为 GNOME 的用户级 IBus 服务增加组件搜索
路径。它不会改动现有输入源列表，也不会在应用列表中创建独立启动器。

用户级卸载：

```bash
./packaging/uninstall-user.sh
```

卸载程序不会删除用户的配置文件和 ASR 凭据。

## 程序路径

- 系统包：`/usr/libexec/typeless-ibus-engine`
- 用户级安装：`~/.local/libexec/typeless-ibus-engine`

[返回文档索引](README.md)
