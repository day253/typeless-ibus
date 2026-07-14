# 使用与配置

## 基本使用

1. 在 Ubuntu“设置 → 键盘 → 输入源”中添加 `Typeless Voice`。
2. 切换到 `Typeless Voice`，将光标放入任意可输入文本的位置。
3. 长按触发键开始录音，松开后结束录音。
4. 识别中的文字显示为预编辑文本，最终文字由 IBus 提交。

按 `Esc` 可以取消当前录音或识别。单次录音默认最多 120 秒。

部分笔记本的 `Fn` 键由固件处理，不会向 Linux 上报 `XF86_Fn`。这种情况可以把
触发键改成 `Control_R` 或 `F8`；这不是 Wayland 限制。

## IBus 配置菜单

切换到 `Typeless Voice` 后，打开 Ubuntu 顶栏的输入法菜单，可以直接修改：

- 触发方式：长按或按键切换。
- 触发键：`Fn`、左右 Ctrl、F8、F9、F10 或空格。

菜单由 IBus 和桌面环境绘制，不需要独立设置程序。选择后会立即生效并写入配置文件，
不需要重新启动 IBus。

麦克风默认跟随系统输入设备，最长录音时间默认是 120 秒。这两个低频选项保留在配置
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
  "maxRecordingSeconds": 120
}
```

- `triggerKey`：支持 `XF86_Fn`、`Control_R`、`Control_L`、`F8`、`F9`、`F10`、
  `Space` 或以 `0x` 开头的十六进制 XKB keysym。
- `triggerMode`：`hold` 表示按下开始、松开结束；`toggle` 表示按一次开始、再按一次结束。
- `inputDevice`：`null` 使用默认麦克风，也可以填写设备名称。
- `maxRecordingSeconds`：允许 1 到 600 秒。

手动修改 JSON 后切换一次输入源即可重新读取。配置示例也可查看
[`data/config.example.json`](../data/config.example.json)。

## 命令行配置

```bash
typeless-ibus-engine config show
typeless-ibus-engine config set trigger-key Control_R
typeless-ibus-engine config set trigger-mode hold
typeless-ibus-engine config set input-device default
typeless-ibus-engine config set max-recording-seconds 120
typeless-ibus-engine config reset
```

使用命令行或手动修改 JSON 后，切换一次输入源即可让新引擎实例读取配置。

[返回文档索引](README.md)
