# 豆包零配置 ASR

豆包是 typeless-ibus 的默认供应商，`provider` 为 `doubao`。它不要求注册账号，也没有需要
用户获取的 API Key、endpoint 或 model。

## 配置

把以下对象合并到 `~/.config/typeless-ibus/config.json`：

```json
{
  "asr": {
    "provider": "doubao"
  }
}
```

配置文件完全没有 `asr` 字段时也会使用该默认值。

## 凭据如何获得

无需手动获得。引擎在第一次识别时通过服务发现注册虚拟设备并获取短期凭据，保存在：

```text
~/.local/share/typeless-ibus/credentials.json
```

服务发现拒绝旧身份时，引擎会重新获取凭据并继续当前录音。不要把
`credentials.json` 复制到其他机器或提交到 Git。

## 验证

```bash
/usr/libexec/typeless-ibus-engine --check-asr
/usr/libexec/typeless-ibus-engine \
  --check-asr-audio /path/to/16k-mono-s16le.pcm
```

第一条命令会实际执行服务发现；第二条会上传测试音频并检查是否返回非空文字。用户级安装
请使用 `~/.local/libexec/typeless-ibus-engine`。

该协议并非公开、稳定的商业 API，服务端行为可能变化。实现来源与风险说明见
[`yangmoling/doubaoime-asr`](https://github.com/yangmoling/doubaoime-asr) 和
[数据、隐私与风险](../privacy.md)。

[返回供应商索引](README.md)
