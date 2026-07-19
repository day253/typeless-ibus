# 使用与配置

[文档首页](index.md) · [安装与卸载](installation.md) · [ASR 供应商](asr/README.md) ·
[语种选择](languages.md)

## 基本使用

1. 在桌面环境的输入源设置中添加 `typeless-ibus`；GNOME 用户可打开“设置 → 键盘 → 输入源”。
2. 切换到 `typeless-ibus`，将光标放入任意可输入文本的位置。
3. 长按触发键开始录音，松开后结束录音。
4. 识别中的文字显示为预编辑文本，最终文字由 IBus 提交。

如果配置了可选 LLM，松开触发键后 ASR 最终稿会继续留在预编辑区，润色并通过本地保护
校验后再一次提交。LLM 失败、超时或改动关键 token 时自动提交 ASR 原文，不会丢失本次输入。

录音开始后，输入框的独立辅助提示会显示实际使用的麦克风，例如 `麦克风：内置麦克风`；
这个设备提示只在刚触发时短暂显示，随后会自动隐藏。预编辑区会先显示“请说话…”/
“Please speak…”，收到识别文本后始终在当前未提交文本后显示灰色的 `…`，表示仍在
持续聆听。如果系统没有默认或指定的输入设备，辅助提示会显示“没有检测到音频输入设备”。

按 `Esc` 可以取消当前录音或识别。单次录音默认最多 600 秒（10 分钟）。

部分笔记本的 `Fn` 键由固件处理，不会向 Linux 上报 `XF86_Fn`。这种情况可以把
触发键改成 `Control_R` 或 `F8`；这不是 Wayland 限制。

## IBus 配置菜单

切换到 `typeless-ibus` 后，打开桌面环境提供的 IBus 输入源菜单，可以直接修改：

- 触发方式：长按或按键切换。
- 触发键：`Fn`、左右 Ctrl、F8、F9、F10 或空格。

菜单会按 `LC_ALL`、`LC_MESSAGES`、`LANG` 的优先级自动选择中文或英文；中文 locale
显示中文，其他 locale 默认显示英文。

菜单由 IBus 和桌面环境绘制，不需要独立设置程序。选择后会立即生效并写入配置文件，
不需要重新启动 IBus。

麦克风默认跟随系统输入设备，最长录音时间默认是 600 秒。这两个低频选项保留在配置
文件和命令行中。

## 配置文件

首次运行会创建：

```text
~/.config/typeless-ibus/config.json
```

默认配置：

```json
{
  "triggerKey": "XF86_Fn",
  "triggerMode": "hold",
  "inputDevice": null,
  "maxRecordingSeconds": 600,
  "asr": {
    "provider": "doubao"
  }
}
```

- `triggerKey`：支持 `XF86_Fn`、`Control_R`、`Control_L`、`F8`、`F9`、`F10`、
  `Space` 或以 `0x` 开头的十六进制 XKB keysym。
- `triggerMode`：`hold` 表示按下开始、松开结束；`toggle` 表示按一次开始、再按一次结束。
- `inputDevice`：`null` 使用默认麦克风，也可以填写设备名称。
- `maxRecordingSeconds`：允许 1 到 600 秒。
- `asr.provider`：默认 `doubao`；只有显式配置其他值时才切换云端接口。

`doubao` 是零配置默认项，不需要用户填写账号、API Key、endpoint 或 model。引擎首次
识别时会自动获取凭据，之后在服务发现拒绝旧身份时自动刷新。已有配置文件即使完全没有
`asr` 字段，也仍按 `doubao` 运行。

### 云端 ASR 配置

每个供应商的控制台入口、凭据获取步骤、字段对应关系和可复制配置已经拆分为
[独立配置文档](asr/README.md)。下面只说明通用字段和内置默认值。

所有需要鉴权的 provider 都可以从 `provider + apiKey` 开始。例如：

```json
{
  "asr": {
    "provider": "elevenlabs",
    "apiKey": "your-api-key"
  }
}
```

`endpoint`、`model`、`language`、`prompt` 和 `resourceId` 都由各 provider 的 Rust 实现
提供默认值或省略行为，只在需要覆盖时填写。每个 provider 真正支持的字段、内置默认值、
最小配置和最大配置统一维护在 [ASR 供应商配置索引](asr/README.md)，本页不重复维护清单。
其中 `language` 默认结合系统 locale 与时区推断，再按 provider 能力发送或回退；详见
[语种选择与回退](languages.md)。

ASR 供应商只由配置文件决定，不从环境变量或残留凭据推断。配置文件权限为 `0600`，但
`apiKey` 仍是明文保存；使用自建 endpoint 时也应确认网络可信。
`openai-compatible`、五个品牌化 Audio Transcriptions provider、`elevenlabs`、
`xiaomi-mimo-asr` 和 `bailian-fun-asr-flash` 在录音结束后上传；豆包、百炼两种实时协议
与火山引擎会持续返回中间文本。

手动修改 JSON 后切换一次输入源即可重新读取。[`data/config.example.json`](https://github.com/day253/typeless-ibus/blob/main/data/config.example.json)
是可以直接复制的最小配置；切换供应商时只需修改 `provider`，再添加该供应商必需的
`apiKey`。有内置默认值或可省略的字段不预先写入。复制命令以及供应商 Key、endpoint、
model 与特殊字段的获取方法请查阅 [ASR 供应商配置索引](asr/README.md)，不要根据字段名
猜测控制台中的对应项。

### 可选 LLM 润色

没有 `llm` 字段时润色关闭。最小配置是 `provider + apiKey`：

```json
{
  "llm": {
    "provider": "deepseek",
    "apiKey": "replace-me"
  }
}
```

可直接复制 [`data/config.llm.example.json`](https://github.com/day253/typeless-ibus/blob/main/data/config.llm.example.json)。
支持 OpenAI、Anthropic、DeepSeek、智谱、硅基流动、Gemini、Moonshot、豆包大模型、通义
千问、Groq、OpenRouter 和自定义 OpenAI-compatible 云端接口。每个 provider 的 Key 获取
方法、默认 endpoint/model、最小和完整示例见 [LLM 润色供应商](llm/README.md)。

默认 `smart` 模式只处理安全的普通文本最终稿。数字、电话号码、URL、邮箱地址、姓名、密码、
PIN、终端和未知用途直接提交原文；密码和 PIN 不发送给 LLM。LLM 不读取选中文本、输入框
已有内容、窗口标题、剪贴板或历史输入。

## 命令行配置

```bash
typeless-ibus-engine config show
typeless-ibus-engine config set trigger-key Control_R
typeless-ibus-engine config set trigger-mode hold
typeless-ibus-engine config set input-device default
typeless-ibus-engine config set max-recording-seconds 600
typeless-ibus-engine config set asr-provider doubao
typeless-ibus-engine --check-llm "嗯我们周三不对周四上线"
typeless-ibus-engine config reset
```

使用命令行或手动修改 JSON 后，切换一次输入源即可让新引擎实例读取配置。

[返回文档索引](index.md)
