export type PipelineState = "idle" | "recording" | "processing" | "error";

export interface AppConfig {
  hotkey: string;
  autoPaste: boolean;
  restoreClipboard: boolean;
  inputDevice: string | null;
}

export interface AudioDevice {
  id: string;
  name: string;
  isDefault: boolean;
}

export interface PlatformInfo {
  os: "macos" | "linux";
  sessionType: "macos" | "x11" | "wayland" | "unknown";
  supportsGlobalShortcut: boolean;
  supportsAutoPaste: boolean;
}

export interface StatusEvent {
  state: PipelineState;
  message: string;
}

export interface FinalEvent {
  text: string;
  inserted: boolean;
}
