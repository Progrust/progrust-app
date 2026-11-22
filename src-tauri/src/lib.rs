pub fn run() {
    tauri::Builder::default()
        .run(tauri::generate_context!())
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
