# 安装与卸载

## 系统要求

- Ubuntu 或 Debian
- IBus 1.5.29+
- Rust stable
- ALSA 和 Opus 开发库

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
