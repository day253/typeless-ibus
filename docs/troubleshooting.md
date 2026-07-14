# 故障排查

以下命令中的系统安装路径是 `/usr/libexec/typeless-ibus-engine`。用户级安装时，请替换为
`~/.local/libexec/typeless-ibus-engine`。

## 基础诊断

```bash
/usr/libexec/typeless-ibus-engine --config-path
/usr/libexec/typeless-ibus-engine --print-config
/usr/libexec/typeless-ibus-engine --list-devices
/usr/libexec/typeless-ibus-engine --check
```

## Fn 没有反应

部分设备的 `Fn` 由键盘固件处理，Linux 收不到独立按键事件。请从 IBus 输入法菜单改用
右 Ctrl、F8 或其他受支持的触发键。

## 找不到麦克风

默认使用系统输入设备。先运行 `--list-devices` 查看 cpal 能发现的输入设备，再通过
`config set input-device DEVICE` 选择设备；`default` 或配置文件中的 `null` 表示系统默认。

## IBus 中找不到输入法

安装后注销并重新登录，或重新启动 IBus。用户级安装还可以重新执行：

```bash
./packaging/install-user.sh
```

然后在 Ubuntu“设置 → 键盘 → 输入源”中添加 `typeless-ibus`。

## ASR 握手诊断

```bash
/usr/libexec/typeless-ibus-engine --check-asr
```

该命令不读取麦克风，也不依赖 IBus。它会检查 settings Token 接口以及
WebSocket 的 `StartTask`、`StartSession` 握手，并隐藏所有凭据。首次握手失败时会使用
同一设备身份重试；仍然失败才注册一个仅用于诊断的临时身份，且不会覆盖本地凭据。

## 真实音频诊断

```bash
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio tests/fixtures/asr-availability.pcm
```

该命令发送指定的 16 kHz、单声道、16-bit little-endian PCM 文件，并要求服务返回非空
文字。失败日志中的 `x_tt_logid` 可用于排查对应的上游请求。

正式输入遇到 `service discovery failure` 时会自动获取新凭据并重放当前音频；第二次
失败不会无限重试。

[返回文档索引](README.md)
