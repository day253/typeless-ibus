# 数据、隐私与风险

[文档首页](README.md) · [默认 provider](asr/doubao.md)

## 数据流

- 默认配置把麦克风音频直接发送到豆包输入法相关服务；显式选择其他 provider 时，音频
  发送到对应供应商或用户覆盖的 endpoint。所有模式都不经过本项目自己的服务器。
- 项目不提供账号、云存储、历史记录或 LLM 处理。
- 识别结果只通过本地 IBus 接口提交到当前输入框。

## 本地凭据

默认豆包 provider 首次使用会注册虚拟设备，并把设备标识与 Token 保存到：

```text
~/.local/share/typeless-ibus/credentials.json
```

文件权限设置为 `0600`。日志不会输出完整凭据；请求失败时记录的是用于排查的
`x-tt-logid`。

其他云端 provider 的 `apiKey` 保存在 `~/.config/typeless-ibus/config.json`，该文件同样
使用 `0600` 权限，但内容是明文。
日志不会打印密钥。请避免把真实配置提交到 Git，并优先使用 HTTPS/WSS endpoint。

## 协议风险

本项目依据公开的非官方协议信息实现互操作客户端。上游协议、接口或访问策略可能随时
变化，也可能停止可用。使用前请自行确认适用的服务条款、隐私要求和所在地区法规。

默认协议的具体实现来源和代码边界见[豆包零配置 ASR](asr/doubao.md)，完整许可归属见
[第三方说明](../THIRD_PARTY.md)。

[返回文档索引](README.md)
