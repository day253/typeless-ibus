# 开发与验证

## 本地检查

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
```

产品运行时代码使用 Rust，Linux 设置界面使用 GTK4，不依赖 Python。

## Debian 打包

```bash
cargo install cargo-deb --version 3.7.0 --locked
cargo build --release --locked
cargo deb --no-build
```

打包资源和安装路径定义在 [`Cargo.toml`](../Cargo.toml) 的
`package.metadata.deb` 中。

## GitHub Actions

CI 在 Ubuntu 24.04 上执行：

- Rust 格式检查
- Clippy（警告视为错误）
- 单元测试
- release 构建
- `.deb` 打包

真实 ASR 可用性测试是独立的非阻塞 job。上游拒绝 GitHub runner 网络时，它会产生
告警并保留 Log ID，但不会阻断 Rust 检查或 `.deb` 打包。

## ASR 固定音频

[`tests/fixtures/asr-availability.pcm`](../tests/fixtures/asr-availability.pcm) 是 16 kHz、
单声道、16-bit little-endian PCM 普通话样本，由 macOS `say` 合成后转换并提交到仓库。
详细格式见 [`tests/fixtures/README.md`](../tests/fixtures/README.md)。

[返回文档索引](README.md)
