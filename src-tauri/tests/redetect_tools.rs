//! 「再検出」のコマンド `redetect_tools`（spec/architecture/commands/environment.md「`redetect_tools`」、
//! spec/exercise-runner/crates-and-workspace.md「初回準備」、ADR 0234）を、偽の `cargo`・`rustc`・`claude` と
//! モックの実行環境で確かめる。
//!
//! 偽の `cargo` の `build`・`test --no-run` は、失敗するか、止めるまで終わらないようにする
//! （tests/fake_tools/cargo_behavior.rs）。初回準備を `ready` まで進めない（手順の 4 の `cp -c` が macOS にしか
//! 無い）ので、どのテストも CI で実行する。

mod common;
#[path = "fake_tools/cargo_behavior.rs"]
mod fake_cargo;
#[path = "fake_tools/rust.rs"]
mod fake_rust;

use std::{
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    sync::mpsc,
    time::{Duration, Instant},
};

use common::{LOCAL_URL, app, invoke};
use fake_cargo::{cargo_calls, hanging_pid, set_behavior};
use fake_rust::FakeRust;
use progrust_lib::claude::ClaudeEnv;
use progrust_lib::environment::preparation::stop_crate_preparation_on_exit;
use progrust_lib::environment::{
    CargoSearch, ClaudeSearch, ClaudeTool, ENVIRONMENT_UPDATED, Environment, RustTools, ToolSearch,
    VERSION_TIMEOUT, start_detection,
};
use progrust_lib::runner::RunnerDir;
use progrust_lib::runner::prepared::{
    PreparedRecord, crates_hash, read_prepared, rustc_verbose_version, write_prepared,
};
use progrust_lib::runner::workspace::WorkspaceDir;
use progrust_lib::settings::Settings;
use progrust_lib::storage::AppDataDir;
use serde_json::{Value, json};
use tauri::test::MockRuntime;
use tauri::{App, Listener, Manager, WebviewWindow};
use tempfile::TempDir;

/// 状態が変わるのを待つ最長の時間。
const WAIT: Duration = Duration::from_secs(30);

/// 前の初回準備が終わった日時（テストで書く `prepared.json` の `prepared_at`）。
const PREPARED_AT: &str = "2026-11-24T19:02:00+09:00";

/// テストごとの学習者データのディレクトリと同梱の許可クレートの定義を置いたアプリ。起動時の検出は、`PATH` を
/// `path_dir` だけにし、ホームディレクトリと `claude` の既定の場所には何も無い状態で探す。`path_dir` に後から
/// 偽の `cargo` を置くと、Rust を導入したことになる。
struct Fixture {
    _dirs: Vec<TempDir>,
    app: App<MockRuntime>,
    window: WebviewWindow<MockRuntime>,
    data_dir: AppDataDir,
    runner: RunnerDir,
    /// 起動時の検出で `PATH` に入れるディレクトリ。
    path_dir: TempDir,
    /// 受け取った `environment_updated` の内容。
    updates: mpsc::Receiver<Value>,
}

impl Fixture {
    fn new() -> Self {
        let (app, window) = app();
        let data = TempDir::new().unwrap();
        let runner_dir = TempDir::new().unwrap();
        let data_dir = AppDataDir::new(data.path());
        let runner = RunnerDir::new(runner_dir.path());
        // 偽の cargo は中身を読まないので、一覧と Cargo.lock は小さいものでよい。
        fs::write(runner.allowed_crates(), "[dependencies]\n").unwrap();
        fs::write(
            runner.cargo_lock(),
            "version = 4\n\n[[package]]\nname = \"progrust_allowed_crates\"\nversion = \"0.1.0\"\n",
        )
        .unwrap();
        app.manage(AppDataDir::new(data.path()));
        app.manage(runner.clone());
        let (sender, updates) = mpsc::channel();
        app.listen(ENVIRONMENT_UPDATED, move |event| {
            let payload: Value = serde_json::from_str(event.payload()).unwrap();
            let _ = sender.send(payload);
        });
        Self {
            _dirs: vec![data, runner_dir],
            app,
            window,
            data_dir,
            runner,
            path_dir: TempDir::new().unwrap(),
            updates,
        }
    }

    fn workspace(&self) -> WorkspaceDir {
        WorkspaceDir::in_app_data(&self.data_dir)
    }

    /// `fake` の `rustc` と、今の一覧で初回準備が済んだ `prepared.json` を書く。
    fn write_ready_record(&self, fake: &FakeRust) {
        write_prepared(
            &self.workspace(),
            &PreparedRecord::ready(
                crates_hash(&self.runner).unwrap(),
                rustc_verbose_version(fake.rustc()).unwrap(),
                PREPARED_AT,
            ),
        )
        .unwrap();
    }

    /// `PATH` のディレクトリに、`version` の偽の `cargo`・`rustc` を置く。
    fn place_in_path(&self, version: &str) -> FakeRust {
        FakeRust::place(self.path_dir.path(), version)
    }

    /// アプリの起動時の検出を始め、終わるまで待つ。
    fn start(&mut self) {
        let empty = TempDir::new().unwrap();
        let search = ToolSearch {
            cargo: CargoSearch {
                configured_cargo: None,
                path_var: Some(std::env::join_paths([self.path_dir.path()]).unwrap()),
                home: Some(empty.path().to_path_buf()),
            },
            claude: ClaudeSearch {
                configured_claude: None,
                path_var: Some(std::env::join_paths([self.path_dir.path()]).unwrap()),
                home: Some(empty.path().to_path_buf()),
                npm_global_dirs: Vec::new(),
                env: ClaudeEnv::from_process(),
                timeout: VERSION_TIMEOUT,
            },
        };
        start_detection(self.app.handle(), search);
        self._dirs.push(empty);
        // 起動時の検出の終わりを待ち、そのときの environment_updated を読み捨てる。
        let _ = self.invoke("get_environment_status");
        self.next_update();
    }

    fn invoke(&self, cmd: &str) -> Result<Value, Value> {
        invoke(&self.window, LOCAL_URL, cmd, json!({}))
    }

    fn redetect(&self) -> Value {
        self.invoke("redetect_tools")
            .expect("redetect_tools が失敗した")
    }

    fn status(&self) -> Value {
        self.invoke("get_environment_status").unwrap()
    }

    /// 設定の `tool_paths` を、設定画面と同じく `update_settings` で変える。
    fn set_tool_paths(&self, cargo: Option<&Path>, claude: Option<&Path>) {
        let mut settings = Settings::default();
        settings.tool_paths.cargo = cargo.map(|path| path.to_str().unwrap().to_string());
        settings.tool_paths.claude = claude.map(|path| path.to_str().unwrap().to_string());
        let settings = serde_json::to_value(settings).unwrap();
        invoke(
            &self.window,
            LOCAL_URL,
            "update_settings",
            json!({ "settings": settings }),
        )
        .expect("update_settings が失敗した");
    }

    fn environment(&self) -> Environment {
        self.app.state::<Environment>().inner().clone()
    }

    /// 次の `environment_updated` の内容。
    fn next_update(&self) -> Value {
        self.updates
            .recv_timeout(WAIT)
            .expect("environment_updated が届かない")
    }

    /// `crates.status` が `status` の `environment_updated` を受け取るまで待ち、その内容を返す。
    fn wait_update(&self, status: &str) -> Value {
        let deadline = Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let payload = self
                .updates
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("crates が {status} の environment_updated が届かない"));
            if payload["crates"]["status"] == status {
                return payload;
            }
        }
    }

    /// 初回準備のスレッドが終わるまで待つ。
    fn wait_finished(&self) {
        assert!(
            self.environment().wait_preparation_finished(WAIT),
            "初回準備が終わらない"
        );
    }
}

impl Drop for Fixture {
    /// 終わらない偽の cargo を残さない。
    fn drop(&mut self) {
        stop_crate_preparation_on_exit(self.app.handle());
    }
}

fn crates_json(status: &str) -> Value {
    json!({ "status": status, "prepared_at": null, "error": null })
}

fn ready_json() -> Value {
    json!({ "status": "ready", "prepared_at": PREPARED_AT, "error": null })
}

fn cargo_found_json(fake: &FakeRust, version: &str) -> Value {
    json!({
        "status": "found",
        "cargo_path": fake.cargo().to_str().unwrap(),
        "rustc_path": fake.rustc().to_str().unwrap(),
        "rustc_version": version,
        "required_rustc_version": "1.100.0",
    })
}

fn cargo_not_found_json() -> Value {
    json!({
        "status": "not_found",
        "cargo_path": null,
        "rustc_path": null,
        "rustc_version": null,
        "required_rustc_version": "1.100.0",
    })
}

fn claude_not_found_json() -> Value {
    json!({ "status": "not_found", "path": null, "version": null })
}

/// `dir` に置いた偽の `claude`（tests/fake_tools/claude を指すシンボリックリンク）のパス。何も用意しないので、
/// `--version` は `2.1.329 (Claude Code)` を返す。偽の `claude` の手助け（fake_tools/claude.rs）は、ここで使わない
/// 機能が多いので読み込まない。
fn place_claude(dir: &Path) -> PathBuf {
    let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fake_tools/claude");
    let path = dir.join("claude");
    symlink(original, &path).unwrap();
    path
}

/// 偽の `cargo` が止めるまで終わらない状態に入るまで待ち、そのプロセス ID を返す。
fn wait_hanging(fake: &FakeRust) -> u32 {
    let deadline = Instant::now() + WAIT;
    loop {
        if let Some(pid) = hanging_pid(fake.cargo()) {
            return pid;
        }
        assert!(Instant::now() < deadline, "偽の cargo が動き出さない");
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// プロセス `pid` が動いているか。
fn is_running(pid: u32) -> bool {
    let pid = libc::pid_t::try_from(pid).unwrap();
    // SAFETY: シグナル 0 は、プロセスがあるかを確かめるだけで、何も送らない。
    unsafe { libc::kill(pid, 0) == 0 }
}

// ---- 設定のパス ----

/// `tool_paths` を変えてから `redetect_tools` を呼ぶと、指定したパスの `cargo`・`claude` を使う。`update_settings`
/// だけでは、検出し直さない。
#[test]
fn the_changed_tool_paths_are_used_after_redetect() {
    let mut fx = Fixture::new();
    let first = fx.place_in_path("1.100.0");
    set_behavior(first.cargo(), "hang", "hang");
    fx.write_ready_record(&first);
    fx.start();
    let before = json!({
        "cargo": cargo_found_json(&first, "1.100.0"),
        "claude": claude_not_found_json(),
        "crates": ready_json(),
    });
    assert_eq!(fx.status(), before);

    // 同じ版の rustc を、別の場所に置いて指定する。prepared.json はそのまま使える。
    let configured = FakeRust::new("1.100.0");
    set_behavior(configured.cargo(), "hang", "hang");
    let claude_dir = TempDir::new().unwrap();
    let claude = place_claude(claude_dir.path());
    fx.set_tool_paths(Some(configured.cargo()), Some(&claude));
    assert_eq!(fx.status(), before, "update_settings で検出し直した");

    let expected = json!({
        "cargo": cargo_found_json(&configured, "1.100.0"),
        "claude": {
            "status": "found",
            "path": claude.to_str().unwrap(),
            "version": "2.1.329",
        },
        "crates": ready_json(),
    });
    assert_eq!(fx.redetect(), expected);
    assert_eq!(fx.status(), expected);
    // 実行・提出と claude を起動する処理も、指定したパスを使う。
    let tools = fx.environment().rust_tools().unwrap();
    assert_eq!(
        tools,
        RustTools {
            cargo: configured.cargo().to_path_buf(),
            rustc: configured.rustc().to_path_buf(),
            rustc_version: "1.100.0".parse().unwrap(),
        }
    );
    assert_eq!(
        fx.environment().claude(),
        Some(ClaudeTool {
            path: claude.clone(),
            version: "2.1.329".into(),
        })
    );
    assert!(cargo_calls(first.cargo()).is_empty());
    assert!(cargo_calls(configured.cargo()).is_empty());

    // 設定のパスを消して検出し直すと、PATH の cargo に戻る。
    fx.set_tool_paths(None, None);
    assert_eq!(fx.redetect(), before);
}

// ---- 初回準備を始めるかどうか ----

/// 起動時に `cargo` が見つからず `not_started` だった後、再検出で見つかると、初回準備を始める。
#[test]
fn cargo_found_after_startup_starts_the_preparation() {
    let mut fx = Fixture::new();
    fx.start();
    assert_eq!(fx.status()["cargo"], cargo_not_found_json());
    assert_eq!(fx.status()["crates"], crates_json("not_started"));

    // Rust を導入する。
    let fake = fx.place_in_path("1.100.0");
    set_behavior(fake.cargo(), "hang", "hang");
    let status = fx.redetect();
    assert_eq!(status["cargo"], cargo_found_json(&fake, "1.100.0"));
    assert_eq!(status["crates"], crates_json("preparing"));
    wait_hanging(&fake);
    assert_eq!(cargo_calls(fake.cargo()), ["build --quiet"]);
}

/// 起動時に `cargo` が見つからなかった後、再検出で見つかり、前の `prepared.json` がそのまま使えるときは、初回準備
/// をせずに `ready` にする。
#[test]
fn cargo_found_after_startup_with_a_usable_prepared_json_is_ready() {
    let mut fx = Fixture::new();
    fx.start();
    assert_eq!(fx.status()["crates"], crates_json("not_started"));
    assert!(fx.environment().ensure_crates_ready().is_err());

    let fake = fx.place_in_path("1.100.0");
    set_behavior(fake.cargo(), "hang", "hang");
    fx.write_ready_record(&fake);
    let status = fx.redetect();
    assert_eq!(status["cargo"], cargo_found_json(&fake, "1.100.0"));
    assert_eq!(status["crates"], ready_json());
    assert_eq!(fx.status(), status);
    assert!(cargo_calls(fake.cargo()).is_empty());
    // 許可クレートを使う演習を受け付ける。
    assert_eq!(fx.environment().ensure_crates_ready(), Ok(()));
}

/// 初回準備の状態が `failed` のときに再検出すると、初回準備を始め直す。`prepared.json` が「実行する条件」に
/// 当てはまらなくても、`failed` であること自体が条件に当たる。
#[test]
fn redetect_after_a_failure_starts_the_preparation_again() {
    let mut fx = Fixture::new();
    let fake = fx.place_in_path("1.100.0");
    set_behavior(fake.cargo(), "fail", "fail");
    fx.start();
    // 失敗は起動時の検出の終わりより先に届くことがあるので、environment_updated ではなく、スレッドの終わりを待つ。
    fx.wait_finished();
    assert_eq!(fx.status()["crates"]["status"], "failed");
    // prepared.json だけを見れば、そのまま使える状態にしておく（prepared.json に書けずに失敗したときなど）。
    fx.write_ready_record(&fake);

    set_behavior(fake.cargo(), "hang", "hang");
    let status = fx.redetect();
    assert_eq!(status["crates"], crates_json("preparing"));
    wait_hanging(&fake);
    assert_eq!(
        cargo_calls(fake.cargo()),
        ["build --quiet", "build --quiet"]
    );
}

/// 再検出で別の版の `rustc` になり、`rustc -vV` の出力が `prepared.json` のものと変わったときは、初回準備を始める。
#[test]
fn a_different_rustc_after_redetect_starts_the_preparation() {
    let mut fx = Fixture::new();
    let first = fx.place_in_path("1.100.0");
    fx.write_ready_record(&first);
    fx.start();
    assert_eq!(fx.status()["crates"], ready_json());

    let newer = FakeRust::new("1.101.0");
    set_behavior(newer.cargo(), "hang", "hang");
    fx.set_tool_paths(Some(newer.cargo()), None);
    let status = fx.redetect();
    assert_eq!(status["cargo"], cargo_found_json(&newer, "1.101.0"));
    assert_eq!(status["crates"], crates_json("preparing"));
    assert_eq!(
        fx.wait_update("preparing")["crates"],
        crates_json("preparing")
    );
    wait_hanging(&newer);
    // 新しい rustc の cargo で準備する。
    assert_eq!(cargo_calls(newer.cargo()), ["build --quiet"]);
    assert!(cargo_calls(first.cargo()).is_empty());
}

/// 初回準備の状態が `preparing` のときに再検出しても、動いている初回準備を止めず、始め直さない。別の版の `rustc`
/// になっても、判定し直さない。
#[test]
fn redetect_while_preparing_keeps_the_running_preparation() {
    let mut fx = Fixture::new();
    let first = fx.place_in_path("1.100.0");
    set_behavior(first.cargo(), "hang", "hang");
    fx.start();
    assert_eq!(fx.status()["crates"], crates_json("preparing"));
    let pid = wait_hanging(&first);

    let newer = FakeRust::new("1.101.0");
    set_behavior(newer.cargo(), "hang", "hang");
    fx.set_tool_paths(Some(newer.cargo()), None);
    let status = fx.redetect();
    assert_eq!(status["cargo"], cargo_found_json(&newer, "1.101.0"));
    assert_eq!(status["crates"], crates_json("preparing"));
    // 動いている初回準備の cargo は、止まらずに動き続けている。
    std::thread::sleep(Duration::from_millis(300));
    assert!(is_running(pid), "動いていた初回準備の cargo が止まった");
    assert_eq!(hanging_pid(first.cargo()), Some(pid));
    assert_eq!(cargo_calls(first.cargo()), ["build --quiet"]);
    assert!(cargo_calls(newer.cargo()).is_empty());
    assert!(!fx.environment().wait_preparation_finished(Duration::ZERO));
}

/// 再検出で `cargo` が見つからなくなったときは、初回準備の状態を変えない。
#[test]
fn cargo_lost_after_redetect_keeps_the_crates_status() {
    let mut fx = Fixture::new();
    let fake = fx.place_in_path("1.100.0");
    fx.write_ready_record(&fake);
    fx.start();
    assert_eq!(fx.status()["crates"], ready_json());

    fs::remove_file(fake.cargo()).unwrap();
    fs::remove_file(fake.rustc()).unwrap();
    let status = fx.redetect();
    assert_eq!(status["cargo"], cargo_not_found_json());
    assert_eq!(status["crates"], ready_json());
    // prepared.json もそのまま。
    assert_eq!(
        read_prepared(&fx.workspace()).unwrap().unwrap().prepared_at,
        PREPARED_AT
    );
}

// ---- environment_updated ----

/// 再検出すると、検出し直した後の環境の状態を `environment_updated` で送る。何も変わらなかったときも送る。
#[test]
fn redetect_sends_environment_updated() {
    let mut fx = Fixture::new();
    fx.start();
    let unchanged = fx.redetect();
    assert_eq!(fx.next_update(), unchanged);

    let fake = fx.place_in_path("1.100.0");
    fx.write_ready_record(&fake);
    let status = fx.redetect();
    assert_eq!(status["cargo"], cargo_found_json(&fake, "1.100.0"));
    assert_eq!(fx.next_update(), status);
}
