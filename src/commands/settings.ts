// 設定のコマンド（documents/spec/architecture/commands/settings.md）。
import { invoke } from "@tauri-apps/api/core";
import type { AiError } from "../claude/aiErrorMessage";

export type AppVersion = {
  version: string;
};

// アプリのバージョンを、バックエンドから受け取る。
export function getAppVersion(): Promise<AppVersion> {
  return invoke<AppVersion>("get_app_version");
}

export type Theme = "light" | "dark" | "system";

// 動画の再生速度として選べる値（documents/spec/screens/video-screen.md）。
export type PlaybackRate = 0.75 | 1 | 1.25 | 1.5 | 2;

// settings.json の形（documents/spec/screens/settings-screen.md「settings.json」）。
export type Settings = {
  schema_version: number;
  theme: Theme;
  // 12〜24（px）。
  editor_font_size: number;
  video: {
    // 0〜1。
    volume: number;
    muted: boolean;
    playback_rate: PlaybackRate;
    captions: boolean;
  };
  // opus / sonnet / haiku のエイリアスか、モデル ID。
  models: { chat: string; background: string };
  // null は自動検出。
  tool_paths: { cargo: string | null; claude: string | null };
  first_launch_completed: boolean;
};

// 設定を、バックエンドから受け取る。
export function getSettings(): Promise<Settings> {
  return invoke<Settings>("get_settings");
}

// 変えた後の設定の全体を保存し、保存した設定を受け取る。schema_version はバックエンドが付ける。
export function updateSettings(settings: Settings): Promise<Settings> {
  return invoke<Settings>("update_settings", { settings });
}

// 接続テストの結果（documents/spec/architecture/commands/settings.md「test_claude_connection」）。
export type ConnectionTestResult = {
  ok: boolean;
  // 使ったモデル（質疑応答のモデル）。
  model: string;
  error: AiError | null;
};

// 質疑応答のモデルで短い指示文を実行し、応答が返るかを確かめる。終わるまで最大 120 秒かかる。
// `claude` が見つからないときは、`claude_unavailable` で reject する。
export function testClaudeConnection(): Promise<ConnectionTestResult> {
  return invoke<ConnectionTestResult>("test_claude_connection");
}
