# 语种选择与回退

[文档首页](README.md) · [使用与配置](usage.md) · [ASR 供应商](asr/README.md)

typeless-ibus 不要求用户为了日常使用在 JSON 中固定 `language`。引擎把“用户偏好”和
“provider 协议能力”分开处理：先从系统推断语音语言，再只向明确支持语言提示的协议发送。

## 默认语种如何确定

1. 当前 provider 支持 `asr.language` 且用户显式填写时，配置值优先。
2. 未填写时，Linux 按 `LC_ALL`、`LC_MESSAGES`、`LANGUAGE`、`LANG` 的优先级读取首选
   locale，并归一化为 `zh`、`en`、`ja` 等语言代码。
3. 很多中国区 Linux 安装默认仍是英文。如果 locale 是英语、`C` 或 `POSIX`，但系统使用
   `Asia/Shanghai` 等中国时区，引擎改用 `zh` 作为语音语言提示。
4. 非英语的明确 locale 始终优先于时区。例如日语系统即使临时使用上海时区，仍选择
   `ja`。
5. provider 不接受语言字段，或推断出的代码不在其支持范围内时，不发送该字段，回退到
   provider 的自动语言识别，不会为了补默认值构造无效请求。

这个分层方式参考 Apple Foundation 把用户首选语言、当前 locale 和当前时区作为独立
系统偏好读取的做法：[`Locale.preferredLanguages`](https://developer.apple.com/documentation/foundation/locale/preferredlanguages)、
[`Locale`](https://developer.apple.com/documentation/foundation/locale) 和
[`TimeZone.autoupdatingCurrent`](https://developer.apple.com/documentation/foundation/timezone/autoupdatingcurrent)。
Linux 实现只使用 Rust 和系统 locale/时区文件，不依赖 Apple API，也不新增 GUI 设置项。

## Provider 支持矩阵

| `provider` | 未配置 `language` 时 | 可显式配置 | 语言字段、上游范围与回退 |
| --- | --- | --- | --- |
| `doubao` | 系统推断值用于虚拟设备注册 | 否 | ASR 协议没有稳定公开的语言字段，识别由上游处理 |
| `openai-compatible` | 不自动发送 | 是 | 目标服务声明支持时原样发送；默认避免破坏精简兼容接口 |
| `whisper` | 发送系统推断值 | 是 | OpenAI 多语种模型；只自动发送 ISO-639-1，否则由模型识别 |
| `groq` | 发送系统推断值 | 是 | Groq Whisper 多语种模型；只自动发送 ISO-639-1，否则由模型识别 |
| `openrouter` | 服务端自动识别 | 否 | 范围由所选模型决定；当前 JSON 音频协议不发送语言字段 |
| `siliconflow` | 不自动发送 | 是 | 范围与字段支持由所选模型决定，明确支持时才手动填写 |
| `zhipu` | 不自动发送 | 是 | 范围与字段支持由所选模型决定，明确支持时才手动填写 |
| `elevenlabs` | 发送系统推断值 | 是 | Scribe v2 支持 90+ 语种；接受 ISO-639-1/639-3，省略时自动识别 |
| `xiaomi-mimo-asr` | 模型自动识别 | 否 | 范围由 MiMo ASR 模型决定，当前适配器不发送语言字段 |
| `bailian` | 模型自动识别 | 否 | 范围由经典 Fun-ASR 模型和地域决定，不发送语言字段 |
| `bailian-qwen3-realtime` | 支持列表内才发送 | 是 | 支持下列 27 个代码；列表外省略并由模型自动识别 |
| `bailian-fun-asr-flash` | 模型自动识别 | 否 | 范围由 Fun-ASR-Flash 模型决定，不发送语言字段 |
| `volcengine` | 服务端资源自动处理 | 否 | 语种范围由已开通 SAUC 资源决定，不发送通用语言字段 |

OpenAI 和 Groq 的转写接口都把 `language` 定义为可选 ISO-639-1 提示；ElevenLabs 接受
ISO-639-1 或 ISO-639-3，省略时会自动预测。官方字段说明分别见
[OpenAI Audio Transcriptions](https://platform.openai.com/docs/api-reference/audio)、
[Groq Speech to Text](https://console.groq.com/docs/speech-to-text) 和
[ElevenLabs Create transcript](https://elevenlabs.io/docs/api-reference/speech-to-text/convert)。

Qwen3 ASR Realtime 自动提示仅允许以下当前官方语言代码：`zh`、`yue`、`en`、`ja`、`de`、
`ko`、`ru`、`fr`、`pt`、`ar`、`it`、`es`、`hi`、`id`、`th`、`tr`、`uk`、`vi`、`cs`、
`da`、`fil`、`fi`、`is`、`ms`、`no`、`pl`、`sv`。完整模型范围见
[阿里云 Qwen-ASR 语音识别说明](https://help.aliyun.com/zh/model-studio/asr-model/)。

## 何时手动设置

只有用户长期使用的口语与系统偏好不同，或 provider 官方建议明确指定语言以降低延迟时，
才需要在该 provider 的最大配置中加入：

```json
{
  "asr": {
    "provider": "whisper",
    "apiKey": "replace-with-openai-api-key",
    "language": "ja"
  }
}
```

不支持语言字段的 provider 会在配置校验阶段给出错误，避免“配置看似生效、请求实际忽略”。
运行 `typeless-ibus-engine --check-asr` 可以看到最终使用的是配置值、系统推断值，还是
provider 自动识别。

[返回文档索引](README.md)
