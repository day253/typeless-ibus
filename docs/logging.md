# 本地日志

[文档首页](index.md) · [数据与隐私](privacy.md) · [故障排查](troubleshooting.md)

typeless-ibus 默认把结构化日志写到当前用户的 XDG state 目录：

```text
~/.local/state/typeless-ibus/logs/
├── typeless-ibus.2026-07-18.jsonl
└── typeless-ibus.latest.jsonl -> typeless-ibus.2026-07-18.jsonl
```

如果设置了 `XDG_STATE_HOME`，则使用 `$XDG_STATE_HOME/typeless-ibus/logs/`。日志按 UTC
日期轮转，保留最近 7 个文件；目录权限为 `0700`，日志文件为 `0600`。可以用下面的命令
获取当前日志路径：

```bash
typeless-ibus-engine --log-path
```

## 格式

日志采用 JSON Lines：每行都是一个独立 JSON 对象。公共字段如下：

| 字段 | 含义 |
| --- | --- |
| `timestamp` | RFC 3339 UTC 时间 |
| `level` | `INFO`、`WARN` 或 `ERROR` |
| `target` | 产生事件的 Rust 模块 |
| `event` | 稳定的产品事件名；产品事件当前使用 `schema_version: 1` |
| `message` | 便于直接阅读的简短说明 |
| `span` | ASR 子事件所属的 `voice_session`，包含 `session_id` 和 Provider |

每次语音输入至少包含两个产品事件：

- `voice_session.started`：录音开始时的 IBus 上下文快照。
- `voice_session.finished`：会话状态、耗时和最终识别文本。
- `voice_session.audio_device_missing`：没有默认或指定的输入设备时记录的错误事件。

完成事件示例：

```json
{"timestamp":"2026-07-18T08:10:12.345Z","level":"INFO","message":"voice session finished","schema_version":1,"event":"voice_session.finished","session_id":"cd426142-7a3d-4f8c-b05e-6524aecc92e3","engine_path":"/org/freedesktop/IBus/Engine/Typeless/1","provider":"doubao","status":"committed","duration_ms":2384,"ibus_client":"gtk4:org.gnome.TextEditor","input_context":"/org/freedesktop/IBus/InputContext_7","cursor_x":742,"cursor_y":518,"cursor_width":2,"cursor_height":24,"input_purpose":"free-form","input_purpose_code":0,"input_hints":0,"client_capabilities":63,"transcript":"明天下午三点开会。","transcript_characters":10,"error":"","target":"typeless_ibus_engine::engine"}
```

`status` 当前可能是：

- `committed`：识别文本已成功提交到当前输入框。
- `commit_failed`：得到了识别文本，但 IBus 提交失败；日志仍保存 `transcript`。
- `no_speech`：服务没有返回非空文字。
- `recognition_failed`：识别失败，`error` 保存脱敏后的错误信息。
- `canceled`：用户按 Esc、焦点离开或输入法被停用。

## IBus 应用上下文

IBus 可以向引擎提供光标矩形、输入用途、输入提示和客户端能力。较新的客户端还可能通过
`FocusInId` 提供客户端名和输入上下文对象路径。typeless-ibus 只保存这些 IBus 字段，
不会读取窗口标题、进程命令行、剪贴板或输入框周边文本。

并非所有桌面和应用都会提供完整信息。缺失的 `ibus_client` 或 `input_context` 记录为
`unknown`；光标坐标也可能是零或由 Wayland/应用进行过坐标转换，因此只能用于排障，不能
当作稳定的应用身份或绝对屏幕位置。

`voice_session.started` 和 `voice_session.finished` 还会记录实际打开的 `audio_device` 名称，
用于确认当前会话使用的是默认设备还是配置中指定的设备。

## 保存和排除的数据

日志会保存：

- 最终识别文本；
- 会话时间、耗时、状态和 Provider；
- 实际打开的音频设备名称；
- IBus 可用的应用上下文；
- 上游返回的 `x-tt-logid`、`request_id` 等排障 ID；
- 脱敏后的错误信息。

日志不会保存：

- PCM、Opus、WAV 或其他录音内容；
- API Key、Token、豆包设备凭据；
- 剪贴板、输入框已有文字或周边文本；
- 窗口截图。

识别文本本身可能敏感。提交 issue 或把日志发给他人之前，请先删除或替换 `transcript`
和不希望公开的应用上下文字段。

## 查询和清理

使用 `jq` 查看成功输入的时间、应用和文字：

```bash
jq -r 'select(.event == "voice_session.finished" and .status == "committed") |
  [.timestamp, .ibus_client, .transcript] | @tsv' \
  ~/.local/state/typeless-ibus/logs/*.jsonl
```

按 `session_id` 关联一次输入的 Provider 请求和最终结果：

```bash
jq -c 'select(.session_id == "SESSION_UUID" or .span.session_id == "SESSION_UUID")' \
  ~/.local/state/typeless-ibus/logs/*.jsonl
```

删除日志不会影响配置或 ASR 凭据；引擎下次写入时会自动创建新的日志文件。

[返回文档索引](index.md)
