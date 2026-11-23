pub mod commands;
pub mod content;
pub mod dashboard;
pub mod progress;
pub mod runner;
pub mod settings;
pub mod storage;

use tauri::{Builder, Context, Manager, Runtime};

use content::ContentDir;
use runner::RunnerDir;
use storage::AppDataDir;

/// コマンドのハンドラを載せたビルダーを返す。`run` と、モックの実行環境のテストで使う。
pub fn app_builder<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        commands::content::list_chapters,
        commands::content::get_lesson,
        commands::learning_record::get_dashboard,
        commands::progress::get_progress,
        commands::progress::open_lesson,
        commands::progress::save_video_position,
        commands::settings::get_app_version,
        commands::settings::get_settings,
        commands::settings::update_settings,
    ])
}

/// `tauri.conf.json` と `capabilities/` から作ったアプリのコンテキスト。
pub fn app_context<R: Runtime>() -> Context<R> {
    tauri::generate_context!()
}

pub fn run() {
    app_builder(Builder::default())
        // 学習者データのディレクトリは、アプリでは Tauri の `app_data_dir()` にする。
        // `app_builder` には入れず、テストでは一時ディレクトリの `AppDataDir` を `manage` で置けるようにする。
        .setup(|app| {
            let data_dir = AppDataDir::from_app(app.handle())?;
            app.manage(data_dir);
            // 教材のディレクトリは、アプリでは `resource_dir()` の下の `content/` にする。同じく、テストでは差し替える。
            app.manage(ContentDir::from_app(app.handle())?);
            // 同梱の許可クレートの定義（`runner/`）も、`resource_dir()` の下にある。
            app.manage(RunnerDir::from_app(app.handle())?);
            Ok(())
        })
        .run(app_context())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    fn config() -> tauri::Config {
        serde_json::from_str(include_str!("../tauri.conf.json"))
            .expect("tauri.conf.json を読めない")
    }

    #[test]
    fn identifier_and_product_name() {
        let config = config();
        assert_eq!(config.identifier, "com.progrust.desktop");
        assert_eq!(config.product_name.as_deref(), Some("Progrust"));
    }

    #[test]
    fn dev_config_identifier() {
        let dev: serde_json::Value = serde_json::from_str(include_str!("../tauri.dev.conf.json"))
            .expect("tauri.dev.conf.json を読めない");
        assert_eq!(dev["identifier"], "com.progrust.desktop.dev");
    }

    /// CSP（spec/non-functional/security-privacy.md「アプリ自体」）。
    /// `tauri dev` では CSP が掛からないので、設定の値をここで確かめる。
    #[test]
    fn csp() {
        let config = config();
        let csp = config.app.security.csp.expect("CSP を設定する").to_string();
        let mut directives: Vec<(String, Vec<String>)> = csp
            .split(';')
            .map(str::trim)
            .filter(|directive| !directive.is_empty())
            .map(|directive| {
                let mut parts = directive.split_whitespace().map(String::from);
                let name = parts.next().unwrap();
                (name, parts.collect())
            })
            .collect();
        directives.sort();
        let expected = [
            ("connect-src", "'self' ipc: http://ipc.localhost"),
            ("default-src", "'self'"),
            ("font-src", "'self' data:"),
            ("img-src", "'self' data:"),
            ("media-src", "'self' asset: data:"),
            ("script-src", "'self'"),
            ("style-src", "'self' 'unsafe-inline'"),
            ("worker-src", "'self'"),
        ]
        .map(|(name, sources)| {
            (
                name.to_string(),
                sources.split(' ').map(String::from).collect::<Vec<_>>(),
            )
        });
        assert_eq!(directives, expected);
    }

    /// アセットプロトコルで読める範囲は、同梱データの音声ファイルだけ（spec/architecture/tauri-permissions.md）。
    /// Tauri のスコープの照合では、`*` は `/` をまたがない。
    #[test]
    fn asset_protocol_scope() {
        let config = config();
        assert!(config.app.security.asset_protocol.enable);
        let raw: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json"))
            .expect("tauri.conf.json を読めない");
        assert_eq!(
            raw["app"]["security"]["assetProtocol"],
            serde_json::json!({
                "enable": true,
                "scope": { "allow": ["$RESOURCE/content/lessons/*/video/voice/*.m4a"] }
            })
        );
    }

    #[test]
    fn window_size() {
        let config = config();
        let [window] = config.app.windows.as_slice() else {
            panic!("ウィンドウは 1 つ");
        };
        assert_eq!((window.width, window.height), (1280.0, 800.0));
        assert_eq!(
            (window.min_width, window.min_height),
            (Some(1024.0), Some(700.0))
        );
    }
}
