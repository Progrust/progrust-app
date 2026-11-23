//! 設定のコマンド（spec/architecture/commands/settings.md）。

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, PackageInfo, Runtime, State};

use super::error::CommandError;
use crate::settings::Settings;
use crate::storage::AppDataDir;

/// `get_app_version` の戻り値。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct AppVersion {
    pub version: String,
}

impl AppVersion {
    /// アプリのパッケージの情報から作る。Tauri の実行時に頼らずに呼べる。
    pub fn from_package_info(package_info: &PackageInfo) -> Self {
        Self {
            version: package_info.version.to_string(),
        }
    }
}

/// アプリのバージョンを返す。フロントエンドには Tauri のアプリ情報の API を許可しないので、
/// 設定画面はこのコマンドでバージョンを受け取る。
#[tauri::command(rename_all = "snake_case")]
pub fn get_app_version<R: Runtime>(app: AppHandle<R>) -> Result<AppVersion, CommandError> {
    Ok(AppVersion::from_package_info(app.package_info()))
}

/// 設定を返す。`settings.json` が無いときは、初期値の設定を返す。
#[tauri::command(rename_all = "snake_case")]
pub fn get_settings(data_dir: State<'_, AppDataDir>) -> Result<Settings, CommandError> {
    Ok(data_dir.read_settings()?)
}

/// 設定の全体を検査してから保存し、保存した設定を返す。`schema_version` はバックエンドが付ける。
/// 値が条件を満たさないときは `invalid_argument` を返し、ファイルは変えない。
#[tauri::command(rename_all = "snake_case")]
pub fn update_settings(
    data_dir: State<'_, AppDataDir>,
    settings: Value,
) -> Result<Settings, CommandError> {
    let settings = Settings::from_argument(settings)?;
    data_dir.write_settings(&settings)?;
    Ok(settings)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package_info(version: &str) -> PackageInfo {
        PackageInfo {
            name: "Progrust".into(),
            version: version.parse().unwrap(),
            authors: "",
            description: "",
            crate_name: "progrust",
        }
    }

    #[test]
    fn app_version_is_the_package_version() {
        assert_eq!(
            AppVersion::from_package_info(&package_info("9.8.7")),
            AppVersion {
                version: "9.8.7".into()
            }
        );
    }

    #[test]
    fn app_version_serializes_as_an_object() {
        assert_eq!(
            serde_json::to_value(AppVersion::from_package_info(&package_info("1.2.3-beta.1")))
                .unwrap(),
            serde_json::json!({ "version": "1.2.3-beta.1" })
        );
    }
}
