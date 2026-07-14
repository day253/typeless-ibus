# 架构与设计

Typeless IBus 是 Linux 原生 IBus 组件。按键事件、预编辑文本和最终提交全部通过 IBus
接口完成，不依赖剪贴板、模拟粘贴或 X11。

## 数据流

```text
物理按键 → IBus ProcessKeyEvent
                  │
          按下触发键 / 松开触发键
                  │
        ALSA + cpal → 16 kHz PCM → Opus
                  │
              豆包 IME ASR
                  │
        IBus preedit / CommitText → 当前输入框
```

## 模块

```text
src/ibus.rs       IBus D-Bus 组件、Factory 与引擎注册
src/engine.rs     长按/切换触发、录音会话和 IBus 文本提交
src/audio.rs      麦克风采集、单声道混音与重采样
src/asr.rs        设备注册、Token、Protobuf、WebSocket 与 Opus
src/config.rs     JSON 配置、共享状态、按键解析与本地路径
src/properties.rs IBus 原生配置菜单与菜单操作解析
```

## 输入会话

触发键按下后，引擎创建带唯一编号的录音会话。音频线程持续产生 16 kHz、单声道、
16-bit PCM 帧，ASR 任务将其编码为 Opus 并发送。部分识别结果更新 IBus preedit，最终
结果使用 `CommitText` 写入当前应用。

会话编号用于隔离过期异步结果：用户取消或开始新录音后，旧任务不能再提交文字。

## ASR 凭据恢复

正式识别遇到 `50700000` 或 `service discovery failure` 时，引擎会：

1. 保留当前会话已经接收的 PCM 帧。
2. 注册新的虚拟设备并获取 Token。
3. 在新连接上从第一帧重放当前语音。
4. 仅在重试成功后原子替换本地凭据。

每次识别最多自动恢复一次，其他错误不会触发凭据轮换。

## 可观测性

WebSocket 建联后会读取响应头中的 `x-tt-logid`。握手、传输或识别失败时，同一个
Log ID 会随错误写入日志，便于关联上游请求。

[返回文档索引](README.md)
