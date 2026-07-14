# ASR availability fixture

`asr-availability.pcm` is macOS `say -v Tingting` speaking:

> 你好，欢迎使用语音输入。

It is stored as headerless 16 kHz, mono, signed 16-bit little-endian PCM. Regenerate it with:

```bash
say -v Tingting -r 165 -o /tmp/typeless-asr-availability.aiff \
  '你好，欢迎使用语音输入。'
ffmpeg -i /tmp/typeless-asr-availability.aiff \
  -ar 16000 -ac 1 -c:a pcm_s16le -f s16le asr-availability.pcm
```

The CI availability check only requires a non-empty recognition result so minor transcription
differences do not make the external-service probe flaky.
