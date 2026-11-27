pub mod attempts;
pub mod chat;
pub mod claude;
pub mod commands;
pub mod completion;
pub mod content;
pub mod dashboard;
pub mod drafts;
pub mod environment;
pub mod progress;
pub mod runner;
pub mod settings;
pub mod storage;

use tauri::{AppHandle, Builder, Context, Manager, Runtime};

use content::ContentDir;
use environment::ToolSearch;
use runner::RunnerDir;
use runner::runs::Runs;
use storage::AppDataDir;

/// コマンドのハンドラを載せたビルダーを返す。`run` と、モックの実行環境のテストで使う。
pub fn app_builder<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder
        // 導入手順の URL を OS の既定ブラウザで開く。開ける URL は capabilities/ の権限で絞る
        // （spec/architecture/tauri-permissions.md）。
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::content::list_chapters,
            commands::content::get_lesson,
            commands::environment::get_environment_status,
            commands::environment::redetect_tools,
            commands::environment::retry_crate_preparation,
            commands::learning_record::get_dashboard,
            commands::progress::end_session,
            commands::progress::get_passed_code,
            commands::progress::get_progress,
            commands::progress::open_lesson,
            commands::progress::reset_draft,
            commands::progress::save_draft,
            commands::progress::save_video_position,
            commands::run::close_stdin,
            commands::run::run_code,
            commands::run::send_stdin,
            commands::run::stop_run,
            commands::run::submit_code,
            commands::settings::get_app_version,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::settings::test_claude_connection,
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
            // 設定の `tool_paths`。設定を読めないときは、設定のパスを使わずに探す。
            let configured_cargo = environment::configured_cargo(&data_dir);
            let configured_claude = environment::configured_claude(&data_dir);
            // 正解した後に終わっていないセッション（前回、異常終了したときのもの）を終える。画面がレッスンを開く前に
            // 終えるように、ここで待って行う。終えられなくても、起動は止めない。
            report_session_errors("起動時", data_dir.end_all_passed_sessions());
            app.manage(data_dir);
            // 教材のディレクトリは、アプリでは `resource_dir()` の下の `content/` にする。同じく、テストでは差し替える。
            app.manage(ContentDir::from_app(app.handle())?);
            // 同梱の許可クレートの定義（`runner/`）も、`resource_dir()` の下にある。
            app.manage(RunnerDir::from_app(app.handle())?);
            // 実行・提出を同時に 1 つだけ行う状態。作業領域は `{app_data_dir}/workspace/`（実行のコマンドが使う）。
            app.manage(Runs::default());
            // cargo・rustc と claude を、画面の表示を待たせずに別のスレッドで探す。cargo が見つかったら、
            // 初回準備の「実行する条件」を判定し、当てはまれば初回準備を始める（上の AppDataDir と RunnerDir を使う）。
            environment::start_detection(
                app.handle(),
                ToolSearch::from_process(configured_cargo, configured_claude),
            );
            Ok(())
        })
        .build(app_context())
        .expect("error while running tauri application")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                // 動いている実行・テスト実行・提出を止める。run_finished を送らず、試行ログに追記しない。
                commands::run::stop_runs_on_exit(app);
                // 動いている初回準備の cargo を止める。prepared.json は書かず、次回の起動でやり直す（ADR 0236）。
                environment::preparation::stop_crate_preparation_on_exit(app);
                end_sessions_on_exit(app);
            }
        });
}

/// アプリを終了するときに、正解済みで終わっていないセッションを終える。
fn end_sessions_on_exit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(data_dir) = app.try_state::<AppDataDir>() {
        report_session_errors("終了時", data_dir.end_all_passed_sessions());
    }
}

/// セッションを終えられなかったことを、標準エラー出力に書く。
fn report_session_errors(when: &str, ended: completion::EndedSessions) {
    for error in ended.errors {
        eprintln!("{when}に試行セッションを終えられなかった: {error}");
    }
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
