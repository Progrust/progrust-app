//! `clang` の検出（spec/architecture/c-toolchain.md「検出」「見つからない場合」）と、環境の状態の `clang`
//! （spec/architecture/commands/environment.md「環境の状態」）を、偽の `clang`・`cargo`・`rustc`・`claude` と
//! モックの実行環境で確かめる。
//!
//! 探す場所の最後の `/usr/bin` は、作者の Mac にも CI の実行環境にも実物の `clang` があるので、テストでは一時
//! ディレクトリに差し替える（`ClangSearch::system_dir`）。アプリで `/usr/bin` を調べることは、
//! src/environment/mod.rs の単体テストで確かめる。

#[path = "fake_tools/clang.rs"]
mod clang;
mod common;
#[path = "fake_tools/rust.rs"]
mod fake_rust;

use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use clang::FakeClang;
use common::{LOCAL_URL, app, invoke};
use fake_rust::FakeRust;
use progrust_lib::claude::{ClaudeEnv, ENV_VAR_NAMES};
use progrust_lib::commands::error::ErrorCode;
use progrust_lib::environment::{
    CargoSearch, ClangSearch, ClangTool, ClaudeSearch, ClaudeTool, DetectedTools,
    ENVIRONMENT_UPDATED, Environment, RustTools, ToolSearch, VERSION_TIMEOUT, configured_clang,
    start_detection,
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

/// 偽のツールのほかに、何も無いディレクトリ（`PATH` や、`/usr/bin` の代わりに使う）。
fn empty_dir() -> TempDir {
    TempDir::new().unwrap()
}

fn dir_of(fake: &FakeClang) -> &Path {
    fake.path().parent().unwrap()
}

/// `clang` を探す入力。`system` は、最後に調べる `/usr/bin` の代わり。
fn search(configured: Option<&Path>, path: &[&Path], system: Option<&Path>) -> ClangSearch {
    ClangSearch {
        configured_clang: configured.map(Path::to_path_buf),
        path_var: Some(std::env::join_paths(path).unwrap()),
        system_dir: system.map(Path::to_path_buf),
        timeout: VERSION_TIMEOUT,
    }
}

/// 検出の結果を、`get_environment_status` の `clang` の形にする。
fn clang_status(search: &ClangSearch) -> Value {
    let status = Environment::detected(DetectedTools {
        clang: search.detect(),
        ..DetectedTools::default()
    })
    .status();
    serde_json::to_value(status.clang).unwrap()
}

fn found_json(fake: &FakeClang, version: Option<&str>) -> Value {
    json!({
        "status": "found",
        "path": fake.path().to_str().unwrap(),
        "version": version,
    })
}

fn not_found_json() -> Value {
    json!({ "status": "not_found", "path": null, "version": null })
}

/// `--version` の 1 行目を `Apple clang version {version} (…)` にした偽の `clang` を、`dir` に置く。
fn clang_with_version(dir: &Path, version: &str) -> FakeClang {
    let fake = FakeClang::place(dir);
    fake.set_version_stdout(&format!(
        "Apple clang version {version} (clang-2100.4.1.1)\nTarget: arm64-apple-darwin27.2.0\n"
    ));
    fake
}

#[test]
fn clang_in_path_is_found_with_its_version() {
    let fake = FakeClang::new();
    let system = empty_dir();
    assert_eq!(
        clang_status(&search(None, &[dir_of(&fake)], Some(system.path()))),
        found_json(&fake, Some("21.0.0"))
    );
    let calls = fake.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].args, ["--version"]);
}

#[test]
fn nothing_found_is_clang_not_found() {
    let path = empty_dir();
    let system = empty_dir();
    assert_eq!(
        clang_status(&search(None, &[path.path()], Some(system.path()))),
        not_found_json()
    );
    // PATH が分からず、最後に調べる場所も無いとき。
    assert_eq!(clang_status(&ClangSearch::nowhere()), not_found_json());
}

// ---- 探す順 ----

/// 設定のパス → PATH → /usr/bin（の代わり）の順に探し、`clang --version` が成功した最初の場所を使う。
/// それより後の場所は調べない。
#[test]
fn clang_is_searched_in_order() {
    let configured_dir = empty_dir();
    let path_dir = empty_dir();
    let system = empty_dir();
    let places = [
        clang_with_version(configured_dir.path(), "1.0.1"),
        clang_with_version(path_dir.path(), "1.0.2"),
        clang_with_version(system.path(), "1.0.3"),
    ];
    let search = search(
        Some(places[0].path()),
        &[path_dir.path()],
        Some(system.path()),
    );
    for (index, place) in places.iter().enumerate() {
        let version = format!("1.0.{}", index + 1);
        assert_eq!(
            clang_status(&search),
            found_json(place, Some(&version)),
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
        place.set_version_exit_code(1);
    }
    assert_eq!(clang_status(&search), not_found_json());
}

#[test]
fn path_directories_are_searched_in_order() {
    let empty = empty_dir();
    let first_dir = empty_dir();
    let first = clang_with_version(first_dir.path(), "1.0.1");
    let second_dir = empty_dir();
    let second = clang_with_version(second_dir.path(), "1.0.2");
    assert_eq!(
        clang_status(&search(
            None,
            &[empty.path(), first_dir.path(), second_dir.path()],
            None
        )),
        found_json(&first, Some("1.0.1"))
    );
    assert!(second.calls().is_empty());
}

/// `PATH` に見つからないときも、見つかったときも、最後に /usr/bin（の代わり）を探す。macOS で Finder から
/// 起動すると、`PATH` が最小のものになり、`clang` の場所を含まないことがある。
#[test]
fn the_system_dir_is_searched_when_not_in_path() {
    let system = empty_dir();
    let fake = FakeClang::place(system.path());
    let path = empty_dir();
    assert_eq!(
        clang_status(&search(None, &[path.path()], Some(system.path()))),
        found_json(&fake, Some("21.0.0"))
    );
    // PATH が分からないとき。
    let without_path = ClangSearch {
        system_dir: Some(system.path().to_path_buf()),
        ..ClangSearch::nowhere()
    };
    assert_eq!(
        clang_status(&without_path),
        found_json(&fake, Some("21.0.0"))
    );
}

/// 設定ファイル（settings.json）の `tool_paths.clang` を、探す場所の最初に使う。
#[test]
fn tool_paths_clang_in_settings_json_is_used() {
    let data = empty_dir();
    let data_dir = AppDataDir::new(data.path());
    let configured_dir = empty_dir();
    let configured = clang_with_version(configured_dir.path(), "20.1.0");
    let mut settings = Settings::default();
    settings.tool_paths.clang = Some(configured.path().to_str().unwrap().to_string());
    data_dir.write_settings(&settings).unwrap();

    let in_path = FakeClang::new();
    let mut search = search(None, &[dir_of(&in_path)], None);
    search.configured_clang = configured_clang(&data_dir);
    assert_eq!(search.configured_clang.as_deref(), Some(configured.path()));
    assert_eq!(
        clang_status(&search),
        found_json(&configured, Some("20.1.0"))
    );
    assert!(in_path.calls().is_empty());
}

/// `tool_paths.clang` が `null` のときと、settings.json が無いときは、設定のパスを使わない。
#[test]
fn without_tool_paths_clang_the_settings_give_no_path() {
    let data = empty_dir();
    let data_dir = AppDataDir::new(data.path());
    assert_eq!(configured_clang(&data_dir), None);
    data_dir.write_settings(&Settings::default()).unwrap();
    assert_eq!(configured_clang(&data_dir), None);
}

// ---- 使えない場所は飛ばす ----

/// `unusable` の場所だけでは見つからず、後ろに使える場所があれば、そちらを使う。
fn assert_skipped(unusable: &Path) {
    assert_eq!(
        clang_status(&search(Some(unusable), &[], None)),
        not_found_json(),
        "{}",
        unusable.display()
    );
    let system = empty_dir();
    let next = FakeClang::place(system.path());
    assert_eq!(
        clang_status(&search(Some(unusable), &[], Some(system.path()))),
        found_json(&next, Some("21.0.0")),
        "{}",
        unusable.display()
    );
}

#[test]
fn a_missing_configured_clang_is_skipped() {
    let dir = empty_dir();
    assert_skipped(&dir.path().join("no-such-dir/clang"));
}

#[test]
fn a_relative_configured_clang_is_skipped() {
    assert_skipped(Path::new("clang"));
    assert_skipped(Path::new("bin/clang"));
}

#[test]
fn a_clang_that_is_not_executable_is_skipped() {
    let dir = empty_dir();
    let path = dir.path().join("clang");
    fs::write(&path, "#!/bin/sh\necho 'Apple clang version 21.0.0'\n").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
    assert_skipped(&path);
    // ディレクトリも使わない。
    let dir_named_clang = dir.path().join("sub/clang");
    fs::create_dir_all(&dir_named_clang).unwrap();
    assert_skipped(&dir_named_clang);
}

#[test]
fn a_clang_whose_version_fails_is_skipped() {
    let failing = FakeClang::new();
    failing.set_version_exit_code(1);
    assert_skipped(failing.path());
    // 起動して確かめたうえで飛ばした（失敗の終了コードなら、バージョンの出力があっても使わない）。
    assert!(!failing.calls().is_empty());
}

/// 出力の 1 行目が空の場所は飛ばす。
#[test]
fn a_clang_with_empty_version_output_is_skipped() {
    for output in ["", "\n", "\nApple clang version 21.0.0\n"] {
        let empty = FakeClang::new();
        empty.set_version_stdout(output);
        assert_skipped(empty.path());
    }
}

/// `clang --version` が時間内に終わらない場所は、止めて飛ばす。アプリでは 10 秒で打ち切る
/// （VERSION_TIMEOUT。src/environment/mod.rs の単体テストで確かめる）が、ここでは短くする。
#[test]
fn a_clang_that_does_not_finish_in_time_is_skipped() {
    let hanging = FakeClang::new();
    hanging.set_version_hang();
    let system = empty_dir();
    let next = FakeClang::place(system.path());
    let mut search = search(Some(hanging.path()), &[], Some(system.path()));
    search.timeout = Duration::from_secs(2);
    let started = Instant::now();
    assert_eq!(clang_status(&search), found_json(&next, Some("21.0.0")));
    assert!(
        started.elapsed() < Duration::from_secs(8),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(hanging.calls().len(), 1, "応答しない clang を起動したはず");

    // 応答しない場所のほかに無ければ、見つからない。
    search.system_dir = None;
    assert_eq!(clang_status(&search), not_found_json());
}

// ---- バージョン ----

/// バージョンを取り出せなくても、その場所を使い、バージョンは `null` にする。1 行目が空白だけのときも、空では
/// ないので使う。
#[test]
fn a_clang_without_a_version_word_is_found_without_a_version() {
    for output in [
        "   \n",
        "Apple clang\n",
        "Apple clang version\n",
        "clang 21.0.0\n",
        "Apple clang\nversion 21.0.0\n",
    ] {
        let fake = FakeClang::new();
        fake.set_version_stdout(output);
        assert_eq!(
            clang_status(&search(Some(fake.path()), &[], None)),
            found_json(&fake, None),
            "{output:?}"
        );
    }
}

#[test]
fn the_version_is_the_word_after_version() {
    for (output, expected) in [
        ("Apple clang version 21.0.0 (clang-2100.4.1.1)\n", "21.0.0"),
        (
            "Homebrew clang version 20.1.8\nTarget: arm64-apple-darwin27.2.0\n",
            "20.1.8",
        ),
        ("clang version 19.1.7", "19.1.7"),
    ] {
        let fake = FakeClang::new();
        fake.set_version_stdout(output);
        assert_eq!(
            clang_status(&search(Some(fake.path()), &[], None)),
            found_json(&fake, Some(expected)),
            "{output:?}"
        );
    }
}

/// 検出は、`clang` のパスを書き換えない（シンボリックリンクの先にしない）。
#[test]
fn found_clang_path_is_not_resolved_through_symlinks() {
    let fake = FakeClang::new();
    let found = search(None, &[dir_of(&fake)], None).detect().unwrap().path;
    assert_eq!(found, fake.path());
    assert!(
        fs::symlink_metadata(&found)
            .unwrap()
            .file_type()
            .is_symlink()
    );
}

// ---- 環境変数 ----

/// `clang --version` は、`claude` に渡す 6 つの環境変数の決まりを当てはめず、アプリのプロセスの環境変数の
/// ままで起動する（spec/claude-integration/invocation.md「環境変数」）。テストのプロセスには、cargo が足した
/// CARGO_* などの、6 つのほかの環境変数がある。
#[test]
fn clang_receives_the_env_vars_of_the_app_process() {
    let fake = FakeClang::new();
    assert_eq!(
        clang_status(&search(Some(fake.path()), &[], None)),
        found_json(&fake, Some("21.0.0"))
    );
    let calls = fake.calls();
    assert_eq!(calls.len(), 1);
    // sh が自分で足したり変えたりする変数と、sh が渡さないことがある名前の変数と、値に改行を含む変数（記録から
    // 正しく読めない）は比べない。macOS は、偽の clang が使う /bin/sh（システムの保護されたプログラム）を起動する
    // ときに DYLD_* を消すので、それも比べない。
    let expected: Vec<(String, String)> = std::env::vars()
        .filter(|(name, value)| {
            !["PWD", "OLDPWD", "SHLVL", "_"].contains(&name.as_str())
                && !name.starts_with("DYLD_")
                && is_shell_name(name)
                && !value.contains('\n')
        })
        .collect();
    assert!(
        expected
            .iter()
            .any(|(name, _)| !ENV_VAR_NAMES.contains(&name.as_str())),
        "6 つのほかの環境変数が無く、確かめられない"
    );
    for (name, value) in &expected {
        assert_eq!(calls[0].env.get(name), Some(value), "{name} が届いていない");
    }
}

/// sh の変数の名前の形（英字か `_` で始まり、英数字と `_` が続く）か。sh によっては、この形でない名前の環境変数を、
/// 起動したプログラムに渡さない。
fn is_shell_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_ascii_alphabetic() || first == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

// ---- 取り出し口と get_environment_status ----

fn clang_tool(fake: &FakeClang) -> ClangTool {
    ClangTool {
        path: fake.path().to_path_buf(),
        version: Some("21.0.0".into()),
    }
}

#[test]
fn clang_returns_the_detected_clang() {
    let fake = FakeClang::new();
    let environment = Environment::detected(DetectedTools {
        clang: Some(clang_tool(&fake)),
        ..DetectedTools::default()
    });
    assert_eq!(environment.clang(), Ok(clang_tool(&fake)));
}

/// 見つからないときは、C の演習が返すエラー `clang_unavailable` になり、Command Line Tools の導入のコマンドを
/// 案内する（spec/architecture/c-toolchain.md「見つからない場合」）。
#[test]
fn clang_is_clang_unavailable_when_not_found() {
    let error = Environment::detected(DetectedTools::default())
        .clang()
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::ClangUnavailable);
    assert!(
        error.message.contains("xcode-select --install"),
        "{}",
        error.message
    );
    assert_eq!(
        serde_json::to_value(&error).unwrap()["code"],
        json!("clang_unavailable")
    );
}

#[test]
fn clang_waits_for_detection() {
    let fake = FakeClang::new();
    let environment = Environment::pending();
    let (sender, receiver) = mpsc::channel();
    let reader = environment.clone();
    let handle = thread::spawn(move || sender.send(reader.clang()).unwrap());
    // 検出が終わるまでは返らない。
    assert!(receiver.recv_timeout(Duration::from_millis(300)).is_err());
    environment.finish_detection(DetectedTools {
        clang: Some(clang_tool(&fake)),
        ..DetectedTools::default()
    });
    let found = receiver.recv_timeout(Duration::from_secs(10)).unwrap();
    assert_eq!(found, Ok(clang_tool(&fake)));
    handle.join().unwrap();
}

/// 前の初回準備が終わった日時（テストで書く `prepared.json` の `prepared_at`）。
const PREPARED_AT: &str = "2026-11-24T19:02:00+09:00";

/// 学習者データのディレクトリと同梱の許可クレートの定義を置き、`fake` の `rustc` と同梱の一覧で初回準備が済んで
/// いるアプリ。起動時の判定は `ready` になり、初回準備は動かない。
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

/// `dir` に置いた偽の `claude`（tests/fake_tools/claude を指すシンボリックリンク）のパス。何も用意しないので、
/// `--version` は `2.1.329 (Claude Code)` を返す。
fn place_claude(dir: &Path) -> PathBuf {
    let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fake_tools/claude");
    let path = dir.join("claude");
    symlink(original, &path).unwrap();
    path
}

/// 起動時の検出を、`rust` の `cargo`・`rustc` と、`claude` と、`clang` の探す場所 `clang` で始め、最初の
/// `environment_updated` と、その後の `get_environment_status` の内容と、アプリの状態の [`Environment`] を返す。
/// `claude` と `cargo` は、偽のものを `PATH` に置いて見つかるようにする。
fn start_with_clang(rust: &FakeRust, clang: ClangSearch) -> (Value, Value, Environment, PathBuf) {
    let (app, window, data) = app_prepared_for(rust);
    let (sender, receiver) = mpsc::channel();
    app.listen(ENVIRONMENT_UPDATED, move |event| {
        let payload: Value = serde_json::from_str(event.payload()).unwrap();
        let _ = sender.send(payload);
    });
    let home = empty_dir();
    let claude_dir = empty_dir();
    let claude = place_claude(claude_dir.path());
    start_detection(
        app.handle(),
        ToolSearch {
            cargo: CargoSearch {
                configured_cargo: None,
                path_var: Some(std::env::join_paths([rust.cargo().parent().unwrap()]).unwrap()),
                home: Some(home.path().to_path_buf()),
            },
            claude: ClaudeSearch {
                configured_claude: None,
                path_var: Some(std::env::join_paths([claude_dir.path()]).unwrap()),
                home: Some(home.path().to_path_buf()),
                npm_global_dirs: Vec::new(),
                env: ClaudeEnv::from_process(),
                timeout: VERSION_TIMEOUT,
            },
            clang,
        },
    );
    let payload = receiver.recv_timeout(Duration::from_secs(30)).unwrap();
    let response = invoke(&window, LOCAL_URL, "get_environment_status", json!({})).unwrap();
    let environment = app.state::<Environment>().inner().clone();
    drop(data);
    (payload, response, environment, claude)
}

fn expected_status(rust: &FakeRust, claude: &Path, clang: Value) -> Value {
    json!({
        "cargo": {
            "status": "found",
            "cargo_path": rust.cargo().to_str().unwrap(),
            "rustc_path": rust.rustc().to_str().unwrap(),
            "rustc_version": "1.100.0",
            "required_rustc_version": "1.100.0",
        },
        "claude": { "status": "found", "path": claude.to_str().unwrap(), "version": "2.1.329" },
        "clang": clang,
        "crates": { "status": "ready", "prepared_at": PREPARED_AT, "error": null },
    })
}

/// 起動時の検出で、`cargo`・`claude` と一緒に `clang` も探し、`get_environment_status` と
/// `environment_updated` で返す。C の演習のビルドは、検出した `clang` を取り出せる。
#[test]
fn get_environment_status_returns_the_detected_clang() {
    let rust = FakeRust::new("1.100.0");
    let clang = FakeClang::new();
    let (payload, response, environment, claude) =
        start_with_clang(&rust, search(None, &[dir_of(&clang)], None));
    let expected = expected_status(&rust, &claude, found_json(&clang, Some("21.0.0")));
    assert_eq!(payload, expected);
    assert_eq!(response, expected);
    assert_eq!(environment.clang(), Ok(clang_tool(&clang)));
}

/// `clang` が見つからない・使えないとき（失敗する、出力が空、応答しない）も、`cargo`・`claude` の検出と初回準備の
/// 判定はそのまま行い、Rust の演習の実行・テスト・提出が使う `cargo`・`rustc` を取り出せる
/// （spec/architecture/c-toolchain.md「見つからない場合」）。
#[test]
fn cargo_and_claude_do_not_depend_on_clang() {
    let failing = FakeClang::new();
    failing.set_version_exit_code(1);
    let empty_output = FakeClang::new();
    empty_output.set_version_stdout("");
    let hanging = FakeClang::new();
    hanging.set_version_hang();
    let nothing = empty_dir();
    let mut hanging_search = search(Some(hanging.path()), &[], None);
    hanging_search.timeout = Duration::from_secs(2);
    for clang in [
        search(None, &[nothing.path()], Some(nothing.path())),
        search(Some(failing.path()), &[], None),
        search(Some(empty_output.path()), &[], None),
        hanging_search,
    ] {
        let rust = FakeRust::new("1.100.0");
        let (payload, response, environment, claude) = start_with_clang(&rust, clang);
        let expected = expected_status(&rust, &claude, not_found_json());
        assert_eq!(payload, expected);
        assert_eq!(response, expected);
        assert_eq!(
            environment.rust_tools(),
            Ok(RustTools {
                cargo: rust.cargo().to_path_buf(),
                rustc: rust.rustc().to_path_buf(),
                rustc_version: "1.100.0".parse().unwrap(),
            })
        );
        assert_eq!(
            environment.claude(),
            Some(ClaudeTool {
                path: claude.clone(),
                version: "2.1.329".into(),
            })
        );
        assert_eq!(environment.ensure_crates_ready(), Ok(()));
        assert_eq!(
            environment.clang().unwrap_err().code,
            ErrorCode::ClangUnavailable
        );
    }
    assert_eq!(hanging.calls().len(), 1, "応答しない clang を起動したはず");
}
