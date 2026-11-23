//! 設定（`settings.json`。spec/screens/settings-screen.md「settings.json」）の型、初期値、値の検査と、
//! 読み書き。

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::commands::error::{CommandError, ErrorCode};
use crate::storage::{AppDataDir, StorageError};

/// `settings.json` の、`{app_data_dir}` からの相対パス。
const SETTINGS_FILE: &str = "settings.json";

/// `settings.json` の `schema_version`。
pub const SCHEMA_VERSION: u64 = 1;

/// エディタの文字サイズ（px）の範囲。
const EDITOR_FONT_SIZE_RANGE: std::ops::RangeInclusive<u32> = 12..=24;

/// 動画の再生速度として選べる値（spec/screens/video-screen.md）。
pub const PLAYBACK_RATES: [f64; 5] = [0.75, 1.0, 1.25, 1.5, 2.0];

/// テーマ。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
    System,
}

/// 動画の設定。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoSettings {
    /// 0.0〜1.0。
    pub volume: f64,
    pub muted: bool,
    pub playback_rate: f64,
    pub captions: bool,
}

/// 用途ごとのモデル（Claude Code のモデルのエイリアスか、モデル ID）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelSettings {
    /// 質疑応答、接続テスト。
    pub chat: String,
    /// 記録・分析・問題生成のすべてのジョブ。
    pub background: String,
}

/// ツールのパスの手動指定。`None`（JSON では `null`）は自動検出。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ToolPaths {
    pub cargo: Option<String>,
    pub claude: Option<String>,
}

/// `settings.json` の全体。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Settings {
    pub schema_version: u64,
    pub theme: Theme,
    pub editor_font_size: u32,
    pub video: VideoSettings,
    pub models: ModelSettings,
    pub tool_paths: ToolPaths,
    pub first_launch_completed: bool,
}

impl Default for Settings {
    /// `settings.json` が無いときの初期値（ADR 0179）。
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            theme: Theme::System,
            editor_font_size: 14,
            video: VideoSettings {
                volume: 1.0,
                muted: false,
                playback_rate: 1.0,
                captions: false,
            },
            models: ModelSettings {
                chat: "sonnet".into(),
                background: "opus".into(),
            },
            tool_paths: ToolPaths {
                cargo: None,
                claude: None,
            },
            first_launch_completed: false,
        }
    }
}

impl Settings {
    /// コマンドの引数の JSON から、設定を作って検査する。`schema_version` は渡された値を使わず、
    /// バックエンドが付ける。形が合わないとき、値が条件を満たさないときは、`invalid_argument` にする。
    pub fn from_argument(argument: Value) -> Result<Self, CommandError> {
        let Value::Object(mut fields) = argument else {
            return Err(invalid_argument("設定がオブジェクトではありません。"));
        };
        fields.insert("schema_version".into(), SCHEMA_VERSION.into());
        let settings: Self = serde_json::from_value(Value::Object(fields))
            .map_err(|error| invalid_argument(error.to_string()))?;
        settings.validate()?;
        Ok(settings)
    }

    /// 値が、設定画面と `settings.json` の仕様の条件を満たすかを確かめる。
    pub fn validate(&self) -> Result<(), CommandError> {
        if !EDITOR_FONT_SIZE_RANGE.contains(&self.editor_font_size) {
            return Err(invalid_argument(format!(
                "editor_font_size は {}〜{} にしてください: {}",
                EDITOR_FONT_SIZE_RANGE.start(),
                EDITOR_FONT_SIZE_RANGE.end(),
                self.editor_font_size
            )));
        }
        if !(0.0..=1.0).contains(&self.video.volume) {
            return Err(invalid_argument(format!(
                "video.volume は 0.0〜1.0 にしてください: {}",
                self.video.volume
            )));
        }
        if !PLAYBACK_RATES.contains(&self.video.playback_rate) {
            return Err(invalid_argument(format!(
                "video.playback_rate は {PLAYBACK_RATES:?} のどれかにしてください: {}",
                self.video.playback_rate
            )));
        }
        for (name, model) in [
            ("models.chat", &self.models.chat),
            ("models.background", &self.models.background),
        ] {
            if model.is_empty() || model.chars().any(|c| c.is_whitespace() || c.is_control()) {
                return Err(invalid_argument(format!(
                    "{name} は、空でなく、空白を含まないモデルのエイリアスかモデル ID にしてください: {model:?}"
                )));
            }
        }
        Ok(())
    }
}

fn invalid_argument(detail: impl Into<String>) -> CommandError {
    CommandError {
        code: ErrorCode::InvalidArgument,
        message: "設定の値が正しくありません。".into(),
        detail: Some(detail.into()),
    }
}

impl AppDataDir {
    /// 設定を読む。ファイルが無いときは初期値を返す（ファイルは作らない）。
    /// 読み込めないときは、退避して初期値で作り直す。
    pub fn read_settings(&self) -> Result<Settings, StorageError> {
        Ok(self
            .read_versioned_json(SETTINGS_FILE, SCHEMA_VERSION, Settings::default)?
            .value)
    }

    /// 検査済みの設定を保存する。
    pub fn write_settings(&self, settings: &Settings) -> Result<(), StorageError> {
        self.write_json(SETTINGS_FILE, settings)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    fn argument() -> Value {
        json!({
            "theme": "dark",
            "editor_font_size": 16,
            "video": { "volume": 0.5, "muted": true, "playback_rate": 1.5, "captions": true },
            "models": { "chat": "haiku", "background": "claude-opus-4-1" },
            "tool_paths": { "cargo": "/opt/cargo", "claude": null },
            "first_launch_completed": true
        })
    }

    fn with(path: &[&str], value: Value) -> Value {
        let mut argument = argument();
        let mut target = &mut argument;
        for key in path {
            target = target.get_mut(*key).unwrap();
        }
        *target = value;
        argument
    }

    #[test]
    fn default_matches_the_spec_json() {
        assert_eq!(
            serde_json::to_value(Settings::default()).unwrap(),
            json!({
                "schema_version": 1,
                "theme": "system",
                "editor_font_size": 14,
                "video": { "volume": 1.0, "muted": false, "playback_rate": 1.0, "captions": false },
                "models": { "chat": "sonnet", "background": "opus" },
                "tool_paths": { "cargo": null, "claude": null },
                "first_launch_completed": false
            })
        );
        Settings::default().validate().unwrap();
    }

    #[test]
    fn a_valid_argument_gets_the_backend_schema_version() {
        let settings = Settings::from_argument(argument()).unwrap();
        assert_eq!(settings.schema_version, 1);
        assert_eq!(settings.theme, Theme::Dark);
        // 渡された schema_version は使わない。
        let mut with_version = argument();
        with_version["schema_version"] = json!(99);
        assert_eq!(Settings::from_argument(with_version).unwrap(), settings);
    }

    #[test]
    fn boundary_values_are_accepted() {
        for (path, value) in [
            (vec!["editor_font_size"], json!(12)),
            (vec!["editor_font_size"], json!(24)),
            (vec!["video", "volume"], json!(0.0)),
            (vec!["video", "volume"], json!(1)),
            (vec!["video", "playback_rate"], json!(0.75)),
            (vec!["video", "playback_rate"], json!(2)),
            (vec!["theme"], json!("light")),
            (vec!["theme"], json!("system")),
        ] {
            Settings::from_argument(with(&path, value.clone()))
                .unwrap_or_else(|e| panic!("{path:?} = {value} を受け付けない: {e:?}"));
        }
    }

    #[test]
    fn invalid_values_are_invalid_argument() {
        for (path, value) in [
            (vec!["theme"], json!("pink")),
            (vec!["theme"], json!(null)),
            (vec!["editor_font_size"], json!(11)),
            (vec!["editor_font_size"], json!(25)),
            (vec!["editor_font_size"], json!(14.5)),
            (vec!["editor_font_size"], json!(-1)),
            (vec!["video", "volume"], json!(-0.1)),
            (vec!["video", "volume"], json!(1.1)),
            (vec!["video", "playback_rate"], json!(1.1)),
            (vec!["video", "playback_rate"], json!(0)),
            (vec!["video", "muted"], json!("yes")),
            (vec!["models", "chat"], json!("")),
            (vec!["models", "background"], json!("two words")),
            (vec!["models", "chat"], json!(3)),
            (vec!["tool_paths", "cargo"], json!(1)),
            (vec!["first_launch_completed"], json!(null)),
        ] {
            let error = Settings::from_argument(with(&path, value.clone())).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidArgument, "{path:?} = {value}");
        }
    }

    #[test]
    fn missing_fields_and_non_objects_are_invalid_argument() {
        let mut missing = argument();
        missing.as_object_mut().unwrap().remove("video");
        for value in [missing, json!(null), json!([]), json!({})] {
            let error = Settings::from_argument(value).unwrap_err();
            assert_eq!(error.code, ErrorCode::InvalidArgument);
        }
    }
}
