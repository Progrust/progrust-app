// cargo・claude が見つからないときに案内する導入手順のページを、OS の既定ブラウザで開く
// （documents/spec/architecture/external-tools.md「見つからない場合」）。
// アプリが開く外部の URL は、ここの 2 つだけである。src-tauri/capabilities/default.json の権限も、
// この 2 つだけを開けるように絞っている（documents/spec/architecture/tauri-permissions.md、ADR 0240）。
// URL を変えるときは、仕様の表と権限の設定もあわせて直す。
import { openUrl } from "@tauri-apps/plugin-opener";

export type InstallGuideTool = "cargo" | "claude";

// 権限の照合は URL の文字列をそのまま比べるので、URL オブジェクトにせず（末尾に `/` が付く）、文字列で渡す。
export const INSTALL_GUIDE_URLS: Readonly<Record<InstallGuideTool, string>> = {
  cargo: "https://rustup.rs",
  claude: "https://code.claude.com/docs/ja/setup",
};

/** ツールの導入手順のページを開く。開けなかったときは reject する。 */
export function openInstallGuide(tool: InstallGuideTool): Promise<void> {
  return openUrl(INSTALL_GUIDE_URLS[tool]);
}
