import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";
import type {
  AppConfig,
  AudioDevice,
  FinalEvent,
  PipelineState,
  PlatformInfo,
  StatusEvent,
} from "./types";

const fallbackConfig: AppConfig = {
  hotkey: "Ctrl+Shift+Space",
  autoPaste: true,
  restoreClipboard: true,
  inputDevice: null,
};

export default function App() {
  const [config, setConfig] = useState<AppConfig>(fallbackConfig);
  const [devices, setDevices] = useState<AudioDevice[]>([]);
  const [platform, setPlatform] = useState<PlatformInfo | null>(null);
  const [state, setState] = useState<PipelineState>("idle");
  const [message, setMessage] = useState("正在初始化…");
  const [transcript, setTranscript] = useState("");
  const [error, setError] = useState("");
  const [saving, setSaving] = useState(false);

  useEffect(() => {
    if (!("__TAURI_INTERNALS__" in window)) {
      setPlatform({
        os: "macos",
        sessionType: "macos",
        supportsGlobalShortcut: true,
        supportsAutoPaste: true,
      });
      setState("idle");
      setMessage("界面预览");
      return;
    }

    void Promise.all([
      invoke<AppConfig>("get_config"),
      invoke<AudioDevice[]>("list_input_devices"),
      invoke<PlatformInfo>("get_platform_info"),
      invoke<StatusEvent>("get_pipeline_status"),
    ])
      .then(([loadedConfig, loadedDevices, loadedPlatform, loadedStatus]) => {
        setConfig(loadedConfig);
        setDevices(loadedDevices);
        setPlatform(loadedPlatform);
        setState(loadedStatus.state);
        setMessage(loadedStatus.message);
      })
      .catch((reason: unknown) => setError(String(reason)));

    const subscriptions = Promise.all([
      listen<StatusEvent>("dictation://state", ({ payload }) => {
        setState(payload.state);
        setMessage(payload.message);
        if (payload.state !== "error") setError("");
      }),
      listen<string>("dictation://partial", ({ payload }) => setTranscript(payload)),
      listen<FinalEvent>("dictation://final", ({ payload }) => {
        setTranscript(payload.text);
        setMessage(payload.inserted ? "文字已输入" : "文字已复制到剪贴板");
      }),
      listen<string>("dictation://error", ({ payload }) => setError(payload)),
    ]);

    return () => {
      void subscriptions.then((unlisten) => unlisten.forEach((dispose) => dispose()));
    };
  }, []);

  const isBusy = state === "processing";
  const buttonLabel = useMemo(() => {
    if (state === "recording") return "结束并识别";
    if (state === "processing") return "正在完成识别…";
    return "开始录音";
  }, [state]);

  async function toggleRecording() {
    setError("");
    if (state === "idle" || state === "error") setTranscript("");
    try {
      await invoke("toggle_recording");
    } catch (reason) {
      setError(String(reason));
    }
  }

  async function saveSettings() {
    setSaving(true);
    setError("");
    try {
      const saved = await invoke<AppConfig>("save_config", { config });
      setConfig(saved);
      setMessage("设置已保存");
    } catch (reason) {
      setError(String(reason));
    } finally {
      setSaving(false);
    }
  }

  return (
    <main className="app-shell">
      <header className="hero">
        <div className="brand-mark" aria-hidden="true">
          T
        </div>
        <div>
          <p className="eyebrow">VOICE INPUT · RUST NATIVE</p>
          <h1>Typeless ASR</h1>
          <p className="hero__copy">说完即输入。没有 LLM，没有改写，只有实时语音转文字。</p>
        </div>
        <span className={`state-badge state-badge--${state}`}>{message}</span>
      </header>

      {platform?.sessionType === "wayland" && (
        <aside className="notice">
          Wayland 会限制全局快捷键和模拟粘贴。可从托盘开始录音，识别结果会复制到剪贴板。
        </aside>
      )}
      {error && <aside className="notice notice--error">{error}</aside>}

      <section className="workspace-card">
        <div className="section-heading">
          <div>
            <span className="step-number">01</span>
            <h2>语音输入</h2>
          </div>
          <kbd>{config.hotkey}</kbd>
        </div>
        <div className={`transcript ${transcript ? "transcript--filled" : ""}`}>
          {transcript || "识别中的文字会出现在这里…"}
        </div>
        <button className={`record-button record-button--${state}`} disabled={isBusy} onClick={toggleRecording}>
          <span className="record-button__dot" />
          {buttonLabel}
        </button>
      </section>

      <section className="settings-card">
        <div className="section-heading">
          <div>
            <span className="step-number">02</span>
            <h2>输入设置</h2>
          </div>
          <span className="platform-chip">{platform?.os === "macos" ? "macOS" : "Linux"}</span>
        </div>

        <label className="field">
          <span>全局快捷键</span>
          <input
            value={config.hotkey}
            onChange={(event) => setConfig({ ...config, hotkey: event.target.value })}
            placeholder="Ctrl+Shift+Space"
          />
        </label>

        <label className="field">
          <span>麦克风</span>
          <select
            value={config.inputDevice ?? ""}
            onChange={(event) =>
              setConfig({ ...config, inputDevice: event.target.value || null })
            }
          >
            <option value="">系统默认麦克风</option>
            {devices.map((device) => (
              <option key={device.id} value={device.id}>
                {device.name}{device.isDefault ? "（默认）" : ""}
              </option>
            ))}
          </select>
        </label>

        <label className="check-row">
          <input
            type="checkbox"
            checked={config.autoPaste && (platform?.supportsAutoPaste ?? true)}
            disabled={platform ? !platform.supportsAutoPaste : false}
            onChange={(event) => setConfig({ ...config, autoPaste: event.target.checked })}
          />
          <span>
            <strong>自动粘贴到当前应用</strong>
            <small>macOS 需要辅助功能权限；Wayland 自动使用剪贴板。</small>
          </span>
        </label>

        <label className="check-row">
          <input
            type="checkbox"
            checked={config.restoreClipboard}
            onChange={(event) =>
              setConfig({ ...config, restoreClipboard: event.target.checked })
            }
          />
          <span>
            <strong>粘贴后恢复原剪贴板</strong>
            <small>仅当剪贴板仍是本次识别结果时恢复。</small>
          </span>
        </label>

        <button className="save-button" disabled={saving} onClick={saveSettings}>
          {saving ? "保存中…" : "保存设置"}
        </button>
      </section>

      <footer>
        <span>Doubao IME ASR · 非官方协议</span>
        <span>Rust + Tauri + React</span>
      </footer>
    </main>
  );
}
