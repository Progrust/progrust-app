//! `cargo`・`rustc` と `claude` の検出（spec/architecture/external-tools.md「検出」）と、環境のコマンドと
//! イベント（spec/architecture/commands/environment.md）を、偽の `cargo`・`rustc`・`claude` とモックの
//! 実行環境で確かめる。

#[path = "fake_tools/claude.rs"]
mod claude;
#[path = "fake_tools/claude_version.rs"]
mod claude_version;
mod common;
#[path = "fake_tools/rust.rs"]
mod fake_rust;

use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::PermissionsExt,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::Duration,
};

use claude::FakeClaude;
use common::{LOCAL_URL, app, invoke};
use fake_rust::FakeRust;
use progrust_lib::claude::{ClaudeEnv, ENV_VAR_NAMES};
use progrust_lib::commands::error::ErrorCode;
use progrust_lib::environment::{
    CargoSearch, ClangSearch, ClaudeSearch, ClaudeTool, DetectedTools, ENVIRONMENT_UPDATED,
    Environment, REQUIRED_RUSTC_VERSION, RustTools, ToolSearch, VERSION_TIMEOUT, configured_cargo,
    configured_claude, start_detection,
};
use progrust_lib::runner::RunnerDir;
use progrust_lib::runner::prepared::{
    PreparedRecord, crates_hash, rustc_verbose_version, write_prepared,
};
use progrust_lib::runner::workspace::WorkspaceDir;
use progrust_lib::settings::Settings;
use progrust_lib::storage::AppDataDir;
use serde_json::{Value, json};
use tauri::{Listener, Manager};
use tempfile::TempDir;

fn dir_of(fake: &FakeRust) -> &Path {
    fake.cargo().parent().unwrap()
}

/// 偽のツールのほかに、何も無いディレクトリ（`PATH` やホームディレクトリに使う）。
fn empty_dir() -> TempDir {
    TempDir::new().unwrap()
}

fn search(configured: Option<&Path>, path: &[&Path], home: &Path) -> CargoSearch {
    CargoSearch {
        configured_cargo: configured.map(Path::to_path_buf),
        path_var: Some(std::env::join_paths(path).unwrap()),
        home: Some(home.to_path_buf()),
    }
}

/// 検出の結果を、`get_environment_status` の `cargo` の形にする。
fn cargo_status(search: &CargoSearch) -> Value {
    let status = Environment::detected(DetectedTools {
        cargo: search.detect(),
        claude: None,
        clang: None,
    })
    .status();
    serde_json::to_value(status.cargo).unwrap()
}

fn found_json(status: &str, fake: &FakeRust, version: &str) -> Value {
    json!({
        "status": status,
        "cargo_path": fake.cargo().to_str().unwrap(),
        "rustc_path": fake.rustc().to_str().unwrap(),
        "rustc_version": version,
        "required_rustc_version": "1.100.0",
    })
}

fn not_found_json() -> Value {
    json!({
        "status": "not_found",
        "cargo_path": null,
        "rustc_path": null,
        "rustc_version": null,
        "required_rustc_version": "1.100.0",
    })
}

// ---- found・version_too_old・not_found ----

#[test]
fn rustc_at_the_required_version_is_found() {
    let fake = FakeRust::new("1.100.0");
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&fake)], home.path())),
        found_json("found", &fake, "1.100.0")
    );
}

#[test]
fn newer_rustc_is_found() {
    for version in ["1.100.1", "1.101.0", "1.102.0-nightly", "2.0.0"] {
        let fake = FakeRust::new(version);
        let home = empty_dir();
        assert_eq!(
            cargo_status(&search(None, &[dir_of(&fake)], home.path())),
            found_json("found", &fake, version)
        );
    }
}

#[test]
fn older_rustc_is_version_too_old() {
    for version in ["1.99.1", "1.85.0", "1.9.0"] {
        let fake = FakeRust::new(version);
        let home = empty_dir();
        assert_eq!(
            cargo_status(&search(None, &[dir_of(&fake)], home.path())),
            found_json("version_too_old", &fake, version)
        );
    }
}

#[test]
fn nothing_found_is_not_found() {
    let path = empty_dir();
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[path.path()], home.path())),
        not_found_json()
    );
    // PATH もホームディレクトリも分からないとき。
    let nothing = CargoSearch {
        configured_cargo: None,
        path_var: None,
        home: None,
    };
    assert_eq!(cargo_status(&nothing), not_found_json());
}

// ---- 探す順 ----

#[test]
fn the_configured_cargo_comes_before_path() {
    let configured = FakeRust::new("1.100.0");
    let in_path = FakeRust::new("1.101.0");
    let home = empty_dir();
    let search = search(Some(configured.cargo()), &[dir_of(&in_path)], home.path());
    assert_eq!(
        cargo_status(&search),
        found_json("found", &configured, "1.100.0")
    );
}

/// 設定ファイル（settings.json）の `tool_paths.cargo` を、探す場所の最初に使う。
#[test]
fn tool_paths_cargo_in_settings_json_is_used() {
    let data = empty_dir();
    let data_dir = AppDataDir::new(data.path());
    let configured = FakeRust::new("1.100.0");
    let mut settings = Settings::default();
    settings.tool_paths.cargo = Some(configured.cargo().to_str().unwrap().to_string());
    data_dir.write_settings(&settings).unwrap();

    let in_path = FakeRust::new("1.101.0");
    let home = empty_dir();
    let mut search = search(None, &[dir_of(&in_path)], home.path());
    search.configured_cargo = configured_cargo(&data_dir);
    assert_eq!(search.configured_cargo.as_deref(), Some(configured.cargo()));
    assert_eq!(
        cargo_status(&search),
        found_json("found", &configured, "1.100.0")
    );
}

/// `tool_paths.cargo` が `null` のときと、settings.json が無いときは、設定のパスを使わない。
#[test]
fn without_tool_paths_cargo_the_settings_give_no_path() {
    let data = empty_dir();
    let data_dir = AppDataDir::new(data.path());
    assert_eq!(configured_cargo(&data_dir), None);
    data_dir.write_settings(&Settings::default()).unwrap();
    assert_eq!(configured_cargo(&data_dir), None);
}

#[test]
fn the_configured_cargo_is_used_even_if_its_rustc_is_too_old() {
    let configured = FakeRust::new("1.99.0");
    let in_path = FakeRust::new("1.100.0");
    let home = empty_dir();
    let search = search(Some(configured.cargo()), &[dir_of(&in_path)], home.path());
    assert_eq!(
        cargo_status(&search),
        found_json("version_too_old", &configured, "1.99.0")
    );
}

#[test]
fn a_missing_configured_cargo_falls_back_to_path() {
    let in_path = FakeRust::new("1.100.0");
    let home = empty_dir();
    let missing = home.path().join("no-such-dir/cargo");
    let search = search(Some(&missing), &[dir_of(&in_path)], home.path());
    assert_eq!(
        cargo_status(&search),
        found_json("found", &in_path, "1.100.0")
    );
}

#[test]
fn the_configured_cargo_uses_rustc_in_the_same_directory() {
    // 設定のパスの `cargo` の隣に `rustc` が無いときは、PATH の `rustc` と組み合わせずに、次の場所を探す。
    let configured = FakeRust::new("1.100.0");
    fs::remove_file(configured.rustc()).unwrap();
    let in_path = FakeRust::new("1.101.0");
    let home = empty_dir();
    let search = search(Some(configured.cargo()), &[dir_of(&in_path)], home.path());
    assert_eq!(
        cargo_status(&search),
        found_json("found", &in_path, "1.101.0")
    );
}

#[test]
fn cargo_bin_in_home_is_searched_when_path_has_no_cargo() {
    let path = empty_dir();
    let home = empty_dir();
    let in_home = FakeRust::place(&home.path().join(".cargo/bin"), "1.100.0");
    assert_eq!(
        cargo_status(&search(None, &[path.path()], home.path())),
        found_json("found", &in_home, "1.100.0")
    );
}

#[test]
fn cargo_bin_in_home_is_searched_after_a_configured_cargo_is_missing() {
    let path = empty_dir();
    let home = empty_dir();
    let in_home = FakeRust::place(&home.path().join(".cargo/bin"), "1.100.0");
    let missing = path.path().join("cargo");
    assert_eq!(
        cargo_status(&search(Some(&missing), &[path.path()], home.path())),
        found_json("found", &in_home, "1.100.0")
    );
}

#[test]
fn path_comes_before_cargo_bin_in_home() {
    let in_path = FakeRust::new("1.99.0");
    let home = empty_dir();
    FakeRust::place(&home.path().join(".cargo/bin"), "1.100.0");
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&in_path)], home.path())),
        found_json("version_too_old", &in_path, "1.99.0")
    );
}

#[test]
fn path_directories_are_searched_in_order() {
    let empty = empty_dir();
    let first = FakeRust::new("1.100.0");
    let second = FakeRust::new("1.101.0");
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(
            None,
            &[empty.path(), dir_of(&first), dir_of(&second)],
            home.path()
        )),
        found_json("found", &first, "1.100.0")
    );
}

// ---- 使えない場所は飛ばす ----

#[test]
fn a_place_without_rustc_is_skipped() {
    let without_rustc = FakeRust::new("1.100.0");
    fs::remove_file(without_rustc.rustc()).unwrap();
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&without_rustc)], home.path())),
        not_found_json()
    );
    let in_home = FakeRust::place(&home.path().join(".cargo/bin"), "1.100.0");
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&without_rustc)], home.path())),
        found_json("found", &in_home, "1.100.0")
    );
}

#[test]
fn a_place_without_cargo_is_skipped() {
    let without_cargo = FakeRust::new("1.100.0");
    fs::remove_file(without_cargo.cargo()).unwrap();
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&without_cargo)], home.path())),
        not_found_json()
    );
}

#[test]
fn a_rustc_whose_version_fails_is_skipped() {
    // 偽の rustc は、rust-version が無いと `--version` に失敗する（終了コード 1）。
    let failing = FakeRust::new("1.100.0");
    fs::remove_file(dir_of(&failing).join("rust-version")).unwrap();
    let next = FakeRust::new("1.101.0");
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&failing)], home.path())),
        not_found_json()
    );
    assert_eq!(
        cargo_status(&search(
            None,
            &[dir_of(&failing), dir_of(&next)],
            home.path()
        )),
        found_json("found", &next, "1.101.0")
    );
}

#[test]
fn a_rustc_with_unreadable_version_output_is_skipped() {
    let odd = FakeRust::new("unknown");
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[dir_of(&odd)], home.path())),
        not_found_json()
    );
}

#[test]
fn files_that_are_not_executable_are_skipped() {
    let dir = empty_dir();
    for name in ["cargo", "rustc"] {
        let path = dir.path().join(name);
        fs::write(&path, "#!/bin/sh\necho rustc 1.100.0\n").unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    }
    let home = empty_dir();
    assert_eq!(
        cargo_status(&search(None, &[dir.path()], home.path())),
        not_found_json()
    );
}

// ---- 実行・提出のコマンドが使う取り出し口 ----

/// `cargo` だけが検出の結果にある（`claude` は見つからない）状態。
fn cargo_only(cargo: Option<RustTools>) -> DetectedTools {
    DetectedTools {
        cargo,
        claude: None,
        clang: None,
    }
}

fn tools(fake: &FakeRust, version: &str) -> RustTools {
    RustTools {
        cargo: fake.cargo().to_path_buf(),
        rustc: fake.rustc().to_path_buf(),
        rustc_version: version.parse().unwrap(),
    }
}

#[test]
fn rust_tools_returns_paths_when_found_or_too_old() {
    for version in ["1.100.0", "1.99.0"] {
        let fake = FakeRust::new(version);
        let environment = Environment::detected(cargo_only(Some(tools(&fake, version))));
        let found = environment.rust_tools().unwrap();
        assert_eq!(found.cargo, fake.cargo());
        assert_eq!(found.rustc, fake.rustc());
        assert_eq!(found.rustc_version.as_str(), version);
    }
}

#[test]
fn rust_tools_is_cargo_unavailable_when_not_found() {
    let error = Environment::detected(DetectedTools::default())
        .rust_tools()
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::CargoUnavailable);
    assert!(!error.message.is_empty());
}

#[test]
fn rust_tools_waits_for_detection() {
    let fake = FakeRust::new("1.100.0");
    let environment = Environment::pending();
    let (sender, receiver) = mpsc::channel();
    let reader = environment.clone();
    let handle = thread::spawn(move || sender.send(reader.rust_tools()).unwrap());
    // 検出が終わるまでは返らない。
    assert!(receiver.recv_timeout(Duration::from_millis(300)).is_err());
    environment.finish_detection(cargo_only(Some(tools(&fake, "1.100.0"))));
    let found = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(found.unwrap().cargo, fake.cargo());
    handle.join().unwrap();
}

// ---- get_environment_status と environment_updated ----

/// `cargo` だけを探し、`claude` はどこも探さない。
fn cargo_search_only(cargo: CargoSearch) -> ToolSearch {
    ToolSearch {
        cargo,
        claude: ClaudeSearch::nowhere(),
        clang: ClangSearch::nowhere(),
    }
}

fn status_json(cargo: Value, crates: Value) -> Value {
    json!({
        "cargo": cargo,
        // claude と clang はどこも探さないので not_found。
        "claude": claude_not_found_json(),
        "clang": { "status": "not_found", "path": null, "version": null },
        "crates": crates,
    })
}

/// 初回準備を始めていないときの `crates`。
fn not_started_json() -> Value {
    json!({ "status": "not_started", "prepared_at": null, "error": null })
}

/// 前の初回準備が終わった日時（テストで書く `prepared.json` の `prepared_at`）。
const PREPARED_AT: &str = "2026-11-24T19:02:00+09:00";

fn ready_json() -> Value {
    json!({ "status": "ready", "prepared_at": PREPARED_AT, "error": null })
}

/// 学習者データのディレクトリと同梱の許可クレートの定義を置き、`fake` の `rustc` と同梱の一覧で初回準備が済んで
/// いる（`prepared.json` が「実行する条件」のどれにも当てはまらない）アプリ。起動時の判定は `ready` になり、
/// 初回準備は動かない。初回準備を動かすテストは tests/crate_preparation.rs にある。
fn app_prepared_for(
    fake: &FakeRust,
) -> (
    tauri::App<tauri::test::MockRuntime>,
    tauri::WebviewWindow<tauri::test::MockRuntime>,
    TempDir,
) {
    let (app, window) = app();
    let data = empty_dir();
    let data_dir = AppDataDir::new(data.path());
    let runner = RunnerDir::new(Path::new(env!("CARGO_MANIFEST_DIR")).join("../runner"));
    write_prepared(
        &WorkspaceDir::in_app_data(&data_dir),
        &PreparedRecord::ready(
            crates_hash(&runner).unwrap(),
            rustc_verbose_version(fake.rustc()).unwrap(),
            PREPARED_AT,
        ),
    )
    .unwrap();
    app.manage(data_dir);
    app.manage(runner);
    (app, window, data)
}

#[test]
fn get_environment_status_without_cargo_returns_not_found() {
    let (app, window) = app();
    let path = empty_dir();
    let home = empty_dir();
    start_detection(
        app.handle(),
        cargo_search_only(search(None, &[path.path()], home.path())),
    );
    let response = invoke(&window, LOCAL_URL, "get_environment_status", json!({}));
    // cargo が見つからないときは、初回準備の条件を判定しない。
    assert_eq!(
        response,
        Ok(status_json(not_found_json(), not_started_json()))
    );
}

#[test]
fn get_environment_status_returns_the_detected_tools() {
    let fake = FakeRust::new("1.100.0");
    let (app, window, _data) = app_prepared_for(&fake);
    let home = empty_dir();
    start_detection(
        app.handle(),
        cargo_search_only(search(None, &[dir_of(&fake)], home.path())),
    );
    let response = invoke(&window, LOCAL_URL, "get_environment_status", json!({}));
    assert_eq!(
        response,
        Ok(status_json(
            found_json("found", &fake, "1.100.0"),
            ready_json()
        ))
    );
}

#[test]
fn get_environment_status_waits_for_detection() {
    let (app, window) = app();
    let environment = Environment::pending();
    app.manage(environment.clone());
    let fake = FakeRust::new("1.99.0");
    let finished = tools(&fake, "1.99.0");
    let finisher = thread::spawn(move || {
        thread::sleep(Duration::from_millis(300));
        environment.finish_detection(cargo_only(Some(finished)));
    });
    let response = invoke(&window, LOCAL_URL, "get_environment_status", json!({}));
    // 検出の結果を直接入れたときは、初回準備を始めない。
    assert_eq!(
        response,
        Ok(status_json(
            found_json("version_too_old", &fake, "1.99.0"),
            not_started_json()
        ))
    );
    finisher.join().unwrap();
}

#[test]
fn environment_updated_is_sent_when_detection_finishes() {
    let fake = FakeRust::new("1.101.0");
    let (app, window, _data) = app_prepared_for(&fake);
    let (sender, receiver) = mpsc::channel();
    app.listen(ENVIRONMENT_UPDATED, move |event| {
        let payload: Value = serde_json::from_str(event.payload()).unwrap();
        let _ = sender.send(payload);
    });
    let home = empty_dir();
    start_detection(
        app.handle(),
        cargo_search_only(search(None, &[dir_of(&fake)], home.path())),
    );
    let payload = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    let expected = status_json(found_json("found", &fake, "1.101.0"), ready_json());
    assert_eq!(payload, expected);
    // 送った後に読んでも、同じ状態が返る。
    let response = invoke(&window, LOCAL_URL, "get_environment_status", json!({}));
    assert_eq!(response, Ok(expected));
}

#[test]
fn required_rustc_version_is_reported_from_the_constant() {
    let status = Environment::detected(DetectedTools::default()).status();
    assert_eq!(status.cargo.required_rustc_version, REQUIRED_RUSTC_VERSION);
}

/// 検出は、実行ファイルのパスを書き換えない（シンボリックリンクの先にしない）。rustup の
/// `~/.cargo/bin/cargo` は、起動した名前でどのツールかを決めるので、リンクの先では動かない。
#[test]
fn found_paths_are_not_resolved_through_symlinks() {
    let fake = FakeRust::new("1.100.0");
    let home = empty_dir();
    let found: PathBuf = search(None, &[dir_of(&fake)], home.path())
        .detect()
        .unwrap()
        .cargo;
    assert_eq!(found, fake.cargo());
    assert!(
        fs::symlink_metadata(&found)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

// ---- claude の検出 ----

fn claude_dir(fake: &FakeClaude) -> &Path {
    fake.path().parent().unwrap()
}

/// `claude` を探す入力。`npm` は、npm のグローバルインストール先（アプリでは /opt/homebrew/bin と
/// /usr/local/bin）の代わり。テストが作者の Mac にある実物の `claude` を見つけないよう、決めた場所だけを探す。
fn claude_search(
    configured: Option<&Path>,
    path: &[&Path],
    home: &Path,
    npm: &[&Path],
) -> ClaudeSearch {
    ClaudeSearch {
        configured_claude: configured.map(Path::to_path_buf),
        path_var: Some(std::env::join_paths(path).unwrap()),
        home: Some(home.to_path_buf()),
        npm_global_dirs: npm.iter().map(|dir| dir.to_path_buf()).collect(),
        env: ClaudeEnv::from_process(),
        timeout: VERSION_TIMEOUT,
    }
}

/// 検出の結果を、`get_environment_status` の `claude` の形にする。
fn claude_status(search: &ClaudeSearch) -> Value {
    let status = Environment::detected(DetectedTools {
        cargo: None,
        claude: search.detect(),
        clang: None,
    })
    .status();
    serde_json::to_value(status.claude).unwrap()
}

fn claude_found_json(fake: &FakeClaude, version: &str) -> Value {
    json!({
        "status": "found",
        "path": fake.path().to_str().unwrap(),
        "version": version,
    })
}

fn claude_not_found_json() -> Value {
    json!({ "status": "not_found", "path": null, "version": null })
}

/// `--version` で `version` を返す偽の `claude` を、`dir` に置く。
fn claude_with_version(dir: &Path, version: &str) -> FakeClaude {
    let fake = FakeClaude::place(dir);
    fake.set_version_stdout(&format!("{version} (Claude Code)\n"));
    fake
}

/// 偽の `claude` の `--version` を失敗させ、その場所を使えなくする。
fn make_unusable(fake: &FakeClaude) {
    fake.set_version_exit_code(1);
}

#[test]
fn claude_in_path_is_found() {
    let fake = FakeClaude::new();
    let home = empty_dir();
    assert_eq!(
        claude_status(&claude_search(None, &[claude_dir(&fake)], home.path(), &[])),
        claude_found_json(&fake, "2.1.329")
    );
}

#[test]
fn nothing_found_is_claude_not_found() {
    let path = empty_dir();
    let home = empty_dir();
    let npm = empty_dir();
    assert_eq!(
        claude_status(&claude_search(
            None,
            &[path.path()],
            home.path(),
            &[npm.path()]
        )),
        claude_not_found_json()
    );
    // PATH もホームディレクトリも分からず、npm のグローバルインストール先も無いとき。
    assert_eq!(
        claude_status(&ClaudeSearch::nowhere()),
        claude_not_found_json()
    );
}

// ---- claude を探す順 ----

/// 設定のパス → PATH → ~/.local/bin → ~/.claude/local → npm のグローバルインストール先の順に探し、
/// `claude --version` が成功した最初の場所を使う。それより後の場所は調べない。
#[test]
fn claude_is_searched_in_order() {
    let configured_dir = empty_dir();
    let path_dir = empty_dir();
    let home = empty_dir();
    let homebrew = empty_dir();
    let usr_local = empty_dir();
    let places = [
        claude_with_version(configured_dir.path(), "1.0.1"),
        claude_with_version(path_dir.path(), "1.0.2"),
        claude_with_version(&home.path().join(".local/bin"), "1.0.3"),
        claude_with_version(&home.path().join(".claude/local"), "1.0.4"),
        claude_with_version(homebrew.path(), "1.0.5"),
        claude_with_version(usr_local.path(), "1.0.6"),
    ];
    let search = claude_search(
        Some(places[0].path()),
        &[path_dir.path()],
        home.path(),
        &[homebrew.path(), usr_local.path()],
    );
    for (index, place) in places.iter().enumerate() {
        let version = format!("1.0.{}", index + 1);
        assert_eq!(
            claude_status(&search),
            claude_found_json(place, &version),
            "{version} の場所が使われるはず"
        );
        // 使った場所より後の場所は、調べていない。
        for later in &places[index + 1..] {
            assert!(
                later.calls().is_empty(),
                "{} を調べた",
                later.path().display()
            );
        }
        make_unusable(place);
    }
    assert_eq!(claude_status(&search), claude_not_found_json());
}

#[test]
fn claude_path_directories_are_searched_in_order() {
    let empty = empty_dir();
    let first_dir = empty_dir();
    let first = claude_with_version(first_dir.path(), "1.0.1");
    let second_dir = empty_dir();
    let second = claude_with_version(second_dir.path(), "1.0.2");
    let home = empty_dir();
    assert_eq!(
        claude_status(&claude_search(
            None,
            &[empty.path(), first_dir.path(), second_dir.path()],
            home.path(),
            &[]
        )),
        claude_found_json(&first, "1.0.1")
    );
    assert!(second.calls().is_empty());
}

/// 設定ファイル（settings.json）の `tool_paths.claude` を、探す場所の最初に使う。
#[test]
fn tool_paths_claude_in_settings_json_is_used() {
    let data = empty_dir();
    let data_dir = AppDataDir::new(data.path());
    let configured = FakeClaude::new();
    configured.set_version_stdout("2.0.0 (Claude Code)\n");
    let mut settings = Settings::default();
    settings.tool_paths.claude = Some(configured.path().to_str().unwrap().to_string());
    data_dir.write_settings(&settings).unwrap();

    let in_path = FakeClaude::new();
    let home = empty_dir();
    let mut search = claude_search(None, &[claude_dir(&in_path)], home.path(), &[]);
    search.configured_claude = configured_claude(&data_dir);
    assert_eq!(search.configured_claude.as_deref(), Some(configured.path()));
    assert_eq!(
        claude_status(&search),
        claude_found_json(&configured, "2.0.0")
    );
}

/// `tool_paths.claude` が `null` のときと、settings.json が無いときは、設定のパスを使わない。
#[test]
fn without_tool_paths_claude_the_settings_give_no_path() {
    let data = empty_dir();
    let data_dir = AppDataDir::new(data.path());
    assert_eq!(configured_claude(&data_dir), None);
    data_dir.write_settings(&Settings::default()).unwrap();
    assert_eq!(configured_claude(&data_dir), None);
}

// ---- claude の使えない場所は飛ばす ----

/// `unusable` の場所だけでは見つからず、後ろに使える場所があれば、そちらを使う。
fn assert_skipped(unusable: &Path) {
    let home = empty_dir();
    assert_eq!(
        claude_status(&claude_search(Some(unusable), &[], home.path(), &[])),
        claude_not_found_json(),
        "{}",
        unusable.display()
    );
    let next = FakeClaude::new();
    assert_eq!(
        claude_status(&claude_search(
            Some(unusable),
            &[claude_dir(&next)],
            home.path(),
            &[]
        )),
        claude_found_json(&next, "2.1.329"),
        "{}",
        unusable.display()
    );
}

#[test]
fn a_missing_configured_claude_is_skipped() {
    let dir = empty_dir();
    assert_skipped(&dir.path().join("no-such-dir/claude"));
}

#[test]
fn a_relative_configured_claude_is_skipped() {
    assert_skipped(Path::new("claude"));
    assert_skipped(Path::new("bin/claude"));
}

#[test]
fn a_claude_that_is_not_executable_is_skipped() {
    let dir = empty_dir();
    let path = dir.path().join("claude");
    fs::write(&path, "#!/bin/sh\necho '2.1.329 (Claude Code)'\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_skipped(&path);
    // ディレクトリも使わない。
    let dir_named_claude = dir.path().join("sub/claude");
    fs::create_dir_all(&dir_named_claude).unwrap();
    assert_skipped(&dir_named_claude);
}

#[test]
fn a_claude_whose_version_fails_is_skipped() {
    let failing = FakeClaude::new();
    failing.set_version_exit_code(1);
    assert_skipped(failing.path());
    // 起動して確かめたうえで飛ばした（失敗の終了コードなら、バージョンの出力があっても使わない）。
    assert!(!failing.calls().is_empty());
}

#[test]
fn a_claude_with_empty_version_output_is_skipped() {
    for output in ["", "\n", "   \n", "\n2.1.329 (Claude Code)\n"] {
        let empty = FakeClaude::new();
        empty.set_version_stdout(output);
        assert_skipped(empty.path());
    }
}

/// `claude --version` が時間内に終わらない場所は、止めて飛ばす。アプリでは 10 秒で打ち切る
/// （VERSION_TIMEOUT。src/environment/mod.rs の単体テストで確かめる）が、ここでは短くする。
#[test]
fn a_claude_that_does_not_finish_in_time_is_skipped() {
    let hanging = FakeClaude::new();
    hanging.set_version_hang();
    let next = FakeClaude::new();
    let home = empty_dir();
    let mut search = claude_search(Some(hanging.path()), &[claude_dir(&next)], home.path(), &[]);
    search.timeout = Duration::from_secs(2);
    let started = std::time::Instant::now();
    assert_eq!(claude_status(&search), claude_found_json(&next, "2.1.329"));
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(hanging.calls().len(), 1, "応答しない claude を起動したはず");

    // 応答しない場所のほかに無ければ、見つからない。
    search.path_var = None;
    assert_eq!(claude_status(&search), claude_not_found_json());
}

// ---- claude に渡す環境変数 ----

/// `claude --version` の呼び出しで、偽の `claude` が受け取った環境変数と引数を確かめる。
fn recorded_version_env(env: ClaudeEnv) -> BTreeMap<String, String> {
    let fake = FakeClaude::new();
    let home = empty_dir();
    let mut search = claude_search(None, &[claude_dir(&fake)], home.path(), &[]);
    search.env = env;
    assert_eq!(claude_status(&search), claude_found_json(&fake, "2.1.329"));
    let calls = fake.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].args, ["--version"]);
    assert_eq!(calls[0].stdin, None, "--version では標準入力を渡さない");
    calls[0].env.clone()
}

fn env_map(vars: &[(&str, &str)]) -> BTreeMap<String, String> {
    vars.iter()
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

#[test]
fn claude_receives_only_the_six_env_vars() {
    let six = [
        ("HOME", "/Users/learner"),
        ("USER", "learner"),
        ("LOGNAME", "learner"),
        ("PATH", "/usr/bin:/bin:/usr/sbin:/sbin"),
        ("SHELL", "/bin/zsh"),
        ("TMPDIR", "/var/folders/xx/progrust-test/T/"),
    ];
    let mut received = six.to_vec();
    received.extend([
        ("ANTHROPIC_API_KEY", "sk-test"),
        ("CLAUDE_CONFIG_DIR", "/tmp/claude-config"),
        ("HTTPS_PROXY", "http://proxy.example:8080"),
        ("LANG", "ja_JP.UTF-8"),
    ]);
    assert_eq!(
        recorded_version_env(ClaudeEnv::from_vars(received)),
        env_map(&six)
    );
}

/// アプリが受け取っていない変数は渡さない。
#[test]
fn claude_does_not_receive_env_vars_the_app_did_not_receive() {
    let received = [("HOME", "/Users/learner"), ("PATH", "/usr/bin:/bin")];
    assert_eq!(
        recorded_version_env(ClaudeEnv::from_vars(received)),
        env_map(&received)
    );
}

/// アプリのプロセスの環境変数（テストでは cargo が足した CARGO_* なども含む）から、6 つだけを渡す。
#[test]
fn claude_receives_the_six_env_vars_of_the_app_process() {
    let expected: BTreeMap<String, String> = std::env::vars()
        .filter(|(name, _)| ENV_VAR_NAMES.contains(&name.as_str()))
        .collect();
    assert!(
        std::env::vars().any(|(name, _)| !ENV_VAR_NAMES.contains(&name.as_str())),
        "6 つのほかの環境変数が無く、確かめられない"
    );
    assert_eq!(recorded_version_env(ClaudeEnv::from_process()), expected);
}

// ---- claude の取り出し口と get_environment_status ----

fn claude_tool(fake: &FakeClaude) -> ClaudeTool {
    ClaudeTool {
        path: fake.path().to_path_buf(),
        version: "2.1.329".into(),
    }
}

#[test]
fn claude_returns_the_detected_claude() {
    let fake = FakeClaude::new();
    let environment = Environment::detected(DetectedTools {
        cargo: None,
        claude: Some(claude_tool(&fake)),
        clang: None,
    });
    assert_eq!(environment.claude(), Some(claude_tool(&fake)));
    assert_eq!(
        Environment::detected(DetectedTools::default()).claude(),
        None
    );
}

#[test]
fn claude_waits_for_detection() {
    let fake = FakeClaude::new();
    let environment = Environment::pending();
    let (sender, receiver) = mpsc::channel();
    let reader = environment.clone();
    let handle = thread::spawn(move || sender.send(reader.claude()).unwrap());
    // 検出が終わるまでは返らない。
    assert!(receiver.recv_timeout(Duration::from_millis(300)).is_err());
    environment.finish_detection(DetectedTools {
        cargo: None,
        claude: Some(claude_tool(&fake)),
        clang: None,
    });
    let found = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(found, Some(claude_tool(&fake)));
    handle.join().unwrap();
}

/// 起動時の検出で、`cargo` と一緒に `claude` も探し、`get_environment_status` と
/// `environment_updated` で返す。
#[test]
fn get_environment_status_returns_the_detected_claude() {
    let rust = FakeRust::new("1.100.0");
    let (app, window, _data) = app_prepared_for(&rust);
    let (sender, receiver) = mpsc::channel();
    app.listen(ENVIRONMENT_UPDATED, move |event| {
        let payload: Value = serde_json::from_str(event.payload()).unwrap();
        let _ = sender.send(payload);
    });
    let claude = FakeClaude::new();
    let home = empty_dir();
    start_detection(
        app.handle(),
        ToolSearch {
            cargo: search(None, &[dir_of(&rust)], home.path()),
            claude: claude_search(None, &[claude_dir(&claude)], home.path(), &[]),
            clang: ClangSearch::nowhere(),
        },
    );
    let expected = json!({
        "cargo": found_json("found", &rust, "1.100.0"),
        "claude": claude_found_json(&claude, "2.1.329"),
        "clang": { "status": "not_found", "path": null, "version": null },
        "crates": ready_json(),
    });
    let payload = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(payload, expected);
    let response = invoke(&window, LOCAL_URL, "get_environment_status", json!({}));
    assert_eq!(response, Ok(expected));
    assert_eq!(
        app.state::<Environment>().claude(),
        Some(claude_tool(&claude))
    );
}

/// 検出は、`claude` のパスを書き換えない（シンボリックリンクの先にしない）。
#[test]
fn found_claude_path_is_not_resolved_through_symlinks() {
    let fake = FakeClaude::new();
    let home = empty_dir();
    let found = claude_search(None, &[claude_dir(&fake)], home.path(), &[])
        .detect()
        .unwrap()
        .path;
    assert_eq!(found, fake.path());
    assert!(
        fs::symlink_metadata(&found)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}
