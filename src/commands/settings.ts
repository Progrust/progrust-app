// 設定のコマンド（documents/spec/architecture/commands/settings.md）。
import { invoke } from "@tauri-apps/api/core";

export type AppVersion = {
  version: string;
};

// アプリのバージョンを、バックエンドから受け取る。
export function getAppVersion(): Promise<AppVersion> {
  return invoke<AppVersion>("get_app_version");
}
