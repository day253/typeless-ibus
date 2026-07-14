# 安装与卸载

## 系统要求

- 支持 IBus 的 Linux 发行版
- IBus 1.5.22+
- Rust stable
- ALSA 和 Opus 开发库

完整的已验证版本、安装方式和兼容边界见[发行版支持范围](distributions.md)。简要来说：
Ubuntu 20.04–26.04 与 Debian 11–13 提供在对应官方用户空间内构建的原生 `.deb`；Fedora
43/44、openSUSE Tumbleweed 和 Arch Linux 通过相同的 IBus 协议测试，可使用 Nix Flake
或从源码安装。

请选择与目标发行版和版本一致的 `.deb`，避免 glibc 和 ALSA ABI 不匹配。所有安装方式
都只包含 Rust IBus 引擎，不再区分 GTK 设置版和无界面版。

## 构建 `.deb` 包

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

安装完成后注销并重新登录，或重新启动 IBus。随后在桌面环境的输入源设置中添加
`typeless-ibus`；GNOME 用户可打开“设置 → 键盘 → 输入源”。

## 其他发行版从源码安装

Fedora：

```bash
sudo dnf install gcc make pkgconf-pkg-config alsa-lib-devel opus-devel ibus
```

openSUSE：

```bash
sudo zypper install gcc make pkg-config alsa-devel libopus-devel ibus
```

Arch Linux：

```bash
sudo pacman -S --needed base-devel pkgconf alsa-lib opus ibus
```

安装 Rust stable 后克隆项目并执行：

```bash
cargo build --release --locked
./packaging/install-user.sh
```

该安装方式只写入当前用户的 `~/.local`，不需要维护 RPM 或 PKGBUILD。

## Nix Flake

Nix 包原生支持 `x86_64-linux` 和 `aarch64-linux`：

```bash
nix profile install github:day253/typeless-ibus
```

安装后重新登录，让图形会话刷新 `$XDG_DATA_DIRS` 和 IBus 组件缓存，然后添加
`typeless-ibus` 输入源。Nix 会封装 Rust 引擎所需的用户空间库，但桌面会话仍需要
发行版提供 IBus 1.5.22 或更高版本。

该方式适用于 Fedora、openSUSE、Arch Linux、NixOS 及其他能够运行 Nix 且由桌面会话
提供 IBus 的发行版。

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
