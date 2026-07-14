# 开发与验证

## 本地检查

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
```

产品运行时代码和 IBus 配置菜单均使用 Rust，不依赖 GUI 工具包或 Python。

## Debian 打包

```bash
cargo install cargo-deb --version 3.7.0 --locked
cargo build --release --locked
cargo deb --no-build
```

打包资源和安装路径定义在 [`Cargo.toml`](../Cargo.toml) 的
`package.metadata.deb` 中。

`.deb` 的 IBus 依赖下限是 1.5.22。CI 在每个 Ubuntu 和 Debian 版本内原生构建包，不使用
高版本 glibc 构建的单一二进制冒充老版本兼容包。

## Nix

```bash
nix flake check
nix build .#packages.x86_64-linux.default
```

Flake 同时暴露 `packages.x86_64-linux.default` 和 `packages.aarch64-linux.default`，CI 会在
x86_64 和 aarch64 GitHub runner 上分别原生构建。

## GitHub Actions

CI 的基础 Rust 检查在 Ubuntu 24.04 上执行，另有发行版兼容矩阵：

- Rust 格式检查
- Clippy（警告视为错误）
- 单元测试
- release 构建
- Ubuntu 20.04、22.04、24.04、26.04 和 Debian 11、12、13 原生 `.deb` 打包
- 上述七个版本的 IBus Factory、`CreateEngine` 和 `ProcessKeyEvent` 协议测试
- Fedora 43/44、openSUSE Tumbleweed 和 Arch Linux 的相同 IBus 协议测试
- x86_64-linux 和 aarch64-linux Nix 构建

真实 ASR 可用性测试是独立的非阻塞 job。上游拒绝 GitHub runner 网络时，它会产生
告警并保留 Log ID，但不会阻断 Rust 检查或 `.deb` 打包。

## ASR 固定音频

[`tests/fixtures/asr-availability.pcm`](../tests/fixtures/asr-availability.pcm) 是 16 kHz、
单声道、16-bit little-endian PCM 普通话样本，由 macOS `say` 合成后转换并提交到仓库。
详细格式见 [`tests/fixtures/README.md`](../tests/fixtures/README.md)。

[返回文档索引](README.md)
