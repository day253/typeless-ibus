# 架构与设计

[文档首页](index.md) · [ASR 供应商设计](asr-providers.md) · [开发与验证](development.md)

typeless-ibus 是 Linux 原生 IBus 组件。按键事件、预编辑文本和最终提交全部通过 IBus
接口完成，不依赖剪贴板、模拟粘贴或 X11。

项目仅实现 IBus 引擎。GTK、Qt、XIM 和 Wayland 应用的输入由发行版提供的 IBus
集成模块覆盖，不再增加并行前端。设置通过 IBus 属性菜单、Rust CLI 和 JSON 完成，
无 GTK 设置程序，当前范围也不包含 Fcitx5。

## 数据流

```text
物理按键 → IBus ProcessKeyEvent
                  │
          按下触发键 / 松开触发键
                  │
        ALSA + cpal → 16 kHz PCM
                  │
             ASR Provider
       ┌──────────┼───────────┐
    豆包/百炼/火山        HTTP 批量接口
    实时 WebSocket       OpenAI/Groq/OpenRouter/
    PCM 或 Opus         ElevenLabs/MiMo/Fun-ASR
                  │
        IBus preedit / CommitText → 当前输入框
```

## 模块

```text
src/ibus.rs       IBus D-Bus 组件、Factory 与引擎注册
src/engine.rs     长按/切换触发、录音会话和 IBus 文本提交
src/audio.rs      麦克风采集、单声道混音与重采样
src/asr.rs        ASR 供应商选择、豆包设备注册、Token、WebSocket 与 Opus
src/asr/provider.rs              与 IBus 会话隔离的 ASR trait
src/asr/openai_compatible.rs     multipart/WAV 音频转写适配器
src/asr/cloud_batch.rs           ElevenLabs、MiMo 与 Fun-ASR-Flash HTTP 协议
src/asr/bailian_realtime.rs      百炼经典与 Qwen3 Realtime WebSocket
src/asr/volcengine.rs            火山引擎流式客户端
src/asr/volcengine_frame.rs      火山引擎二进制帧编解码
src/asr/shared.rs                PCM、WAV、切片、请求 ID 与脱敏工具
src/config.rs     JSON 配置、共享状态、按键解析与本地路径
src/logging.rs    JSONL 本地日志、按天轮转、保留策略与文件权限
src/properties.rs IBus 原生配置菜单与菜单操作解析
```

## 输入会话

触发键按下后，引擎创建带唯一编号的录音会话。音频线程持续产生 16 kHz、单声道、
16-bit PCM 帧，再交给配置选中的 ASR provider。实时 provider 边录边发送并用部分结果
更新 IBus preedit；批量 provider 缓存当前录音、转换为 WAV，在松开触发键后一次提交。
最终结果使用 `CommitText` 写入当前应用。

会话编号用于隔离过期异步结果：用户取消或开始新录音后，旧任务不能再提交文字。

## ASR 供应商边界

IBus 引擎只依赖统一的 `AsrProvider` trait：输入是 16 kHz 单声道 s16le PCM channel，
输出是 `SpeechStarted`、`Partial`、`Final` 事件。供应商独有的协议、鉴权、音频封装和
请求 ID 日志留在各自适配器中；新增服务不需要修改 IBus D-Bus 或麦克风模块。

配置文件是供应商选择的唯一来源。没有 `asr` 配置时构造默认 `doubao` provider；不会
因为环境变量、OpenAI Key 或本地凭据文件而隐式切换服务。

## 豆包 ASR 凭据恢复

正式识别遇到 `50700000` 或 `service discovery failure` 时，引擎会：

1. 保留当前会话已经接收的 PCM 帧。
2. 注册新的虚拟设备并获取 Token。
3. 在新连接上从第一帧重放当前语音。
4. 等待 2 秒后重试，并仅在重试成功后原子替换本地凭据。

每次识别最多自动恢复一次；并发额度错误只重放一次，不触发凭据轮换，其他错误不会自动重试。

## 可观测性

每次语音输入生成 UUID `session_id`。识别任务在同名 tracing span 中运行，使 Provider
建联、请求 ID、错误和最终提交可以关联。`voice_session.started` 快照记录当时可用的 IBus
客户端、输入上下文、光标矩形、内容类型与能力；`voice_session.finished` 记录状态、耗时和
最终识别文本。旧客户端若只调用 `FocusIn` 而不调用 `FocusInId`，客户端和输入上下文会
明确记录为 `unknown`。

豆包 WebSocket 建联后读取 `x-tt-logid`。其他云端接口按响应读取 `x-request-id`、
`request-id`、`x-trace-id`、`x-tt-logid` 或 `x-api-request-id`；DashScope JSON 的
`request_id` 也会被识别。握手、传输或识别失败时，请求 ID 随错误写入日志，同时不会
记录任何密钥或音频。

[返回文档索引](index.md)
