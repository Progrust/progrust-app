pub mod commands;
pub mod storage;

use tauri::{Builder, Context, Manager, Runtime};

use storage::AppDataDir;

/// コマンドのハンドラを載せたビルダーを返す。`run` と、モックの実行環境のテストで使う。
pub fn app_builder<R: Runtime>(builder: Builder<R>) -> Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        commands::settings::get_app_version,
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
