# LLM 润色供应商

[文档首页](../index.md) · [使用与配置](../usage.md) · [数据与隐私](../privacy.md)

LLM 是可选的最终文本润色步骤。没有 `llm` 配置时功能关闭，ASR 行为与之前完全一致；
配置后，引擎只把本次语音的最终文字发送给所选供应商，不发送音频、输入框已有内容、
窗口标题、剪贴板或历史记录。

## 选择供应商

运行时只维护 OpenAI Chat Completions compatible 与 Anthropic Messages 两套 Rust 协议，
品牌化 provider 只提供经过整理的 endpoint、model 和认证方式默认值。

| Provider | 协议 | 默认 model | 配置文档 |
| --- | --- | --- | --- |
| `openai` | OpenAI compatible | `gpt-5-mini` | [OpenAI](openai.md) |
| `anthropic` | Anthropic Messages | `claude-haiku-4-5` | [Anthropic](anthropic.md) |
| `deepseek` | OpenAI compatible | `deepseek-v4-flash` | [DeepSeek](deepseek.md) |
| `zhipu` | OpenAI compatible | `glm-4-flash` | [智谱](zhipu.md) |
| `siliconflow` | OpenAI compatible | `Qwen/Qwen2.5-7B-Instruct` | [硅基流动](siliconflow.md) |
| `gemini` | OpenAI compatible | `gemini-3.5-flash` | [Gemini](gemini.md) |
| `moonshot` | OpenAI compatible | `moonshot-v1-8k` | [Moonshot](moonshot.md) |
| `doubao` | OpenAI compatible | `doubao-seed-1-6-flash-250615` | [豆包大模型](doubao.md) |
| `qwen` | OpenAI compatible | `qwen3.6-flash` | [通义千问](qwen.md) |
| `groq` | OpenAI compatible | `qwen/qwen3.6-27b` | [Groq](groq.md) |
| `openrouter` | OpenAI compatible | `openai/gpt-5-mini` | [OpenRouter](openrouter.md) |
| `openai-compatible` | OpenAI compatible | `gpt-5-mini` | [自定义兼容接口](openai-compatible.md) |

模型会持续更新。表格是当前内置默认值，不代表供应商永久保证；如果账户区域、权限或模型
生命周期不同，请在配置中显式覆盖 `model`，不需要修改代码。

## 最快开始

复制带 LLM 的示例：

```bash
install -Dm600 /usr/share/doc/typeless-ibus/config.llm.example.json \
  ~/.config/typeless-ibus/config.json
```

源码安装可把路径替换为 `data/config.llm.example.json`。然后把 `replace-me` 换成真实 Key，
切换一次输入源使新引擎实例读取配置。也可以直接在现有配置末尾增加：

```json
{
  "llm": {
    "provider": "deepseek",
    "apiKey": "replace-me"
  }
}
```

最小配置只需要 `provider + apiKey`。每个品牌化 provider 都会从 Rust 实现取得 endpoint、
model、10 秒超时、`smart` 模式和 `clean` 风格默认值。

## 润色边界

`smart` 模式只润色 IBus 标记为普通文本或字母文本的最终 ASR 稿。数字、号码、URL、
邮箱地址、姓名、密码、PIN、终端和未知用途会直接提交原文；短确认词也直接提交。
密码和 PIN 的文本不会发送给 LLM，也不会写入本地日志。

文本进入 LLM 润色流程后，本地完成日志会同时保存润色前的 `llm_original_transcript`
和最终实际提交的 `transcript`，方便比较改写效果并迭代提示词。日志仅保留在当前用户目录；
共享前应先脱敏，字段和查询方式见[本地日志](../logging.md)。

允许的变换只有标点、大小写、无意义口头词、明确误重复、明确自我纠正、明确列表和必要
分段。提示词要求保留语言、事实、否定、数字、日期、金额、版本、URL、邮箱、路径、命令、
参数、标识符、专有名词和中英混输文本。

LLM 输出还会经过本地保护：空结果、截断结果、语言改变、异常长度、包装说明或关键 token
变化都会被拒绝并回退到 ASR 原文。429、5xx、连接失败和超时会等待后重试一次；整个润色
阶段仍受 `timeoutMs` 总超时限制。

## 可选字段

| 字段 | 默认值 | 含义 |
| --- | --- | --- |
| `enabled` | `true` | 保留配置但临时关闭时设为 `false` |
| `endpoint` | provider 内置 | 完整的请求 URL，不是只有域名的 base URL |
| `model` | provider 内置 | 供应商模型 ID |
| `mode` | `smart` | `smart` 跳过短确认词；`always` 处理所有安全的普通文本 |
| `style` | `clean` | `clean`、`concise` 或 `formal` |
| `timeoutMs` | `10000` | 整个调用总超时，允许 `3000` 到 `30000` |
| `customPrompt` | 无 | 仅补充表达偏好，不能覆盖保真和安全规则 |

使用下面的命令单独测试 LLM，不需要录音或进入 IBus 集成流程：

```bash
typeless-ibus-engine --check-llm "嗯我们周三不对周四上线"
```

[返回文档索引](../index.md)
