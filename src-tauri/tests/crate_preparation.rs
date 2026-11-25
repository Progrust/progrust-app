//! アプリの起動時の初回準備の判定と開始、初回準備の状態と `environment_updated`、`retry_crate_preparation`、
//! アプリの終了での停止（spec/exercise-runner/crates-and-workspace.md「初回準備」、
//! spec/architecture/commands/environment.md、ADR 0100、ADR 0236）を、偽の `cargo`・`rustc` とモックの実行環境で
//! 確かめる。
//!
//! 偽の `cargo` は、`build`・`test --no-run` を、成功する・失敗する・止めるまで終わらない、のどれかにできる
//! （tests/fake_tools/rust.rs）。初回準備を `ready` まで進めるテストは、手順の 4 の `cp -c`（APFS のクローン）が
//! macOS にしか無いので、macOS のときだけ実行する（spec/non-functional/testing.md「CI」）。`failed`・`preparing` までの
//! テストは CI でも実行する。

mod common;
#[path = "fake_tools/cargo_behavior.rs"]
mod fake_cargo;
#[path = "fake_tools/rust.rs"]
mod fake_rust;

use std::{
    fs,
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};

use common::{LOCAL_URL, app, invoke};
use fake_cargo::{cargo_calls, hanging_pid, set_behavior};
use fake_rust::FakeRust;
use progrust_lib::environment::preparation::{
    Judgment, judge, start_crate_preparation, stop_crate_preparation_on_exit,
};
use progrust_lib::environment::{
    CargoSearch, ClaudeSearch, CratesState, DetectedTools, ENVIRONMENT_UPDATED, Environment,
    RustTools, ToolSearch, start_detection,
};
use progrust_lib::runner::RunnerDir;
use progrust_lib::runner::prepared::{
    PreparedRecord, PreparedStatus, crates_hash, read_prepared, rustc_verbose_version,
    write_prepared,
};
use progrust_lib::runner::workspace::WorkspaceDir;
use progrust_lib::storage::AppDataDir;
use serde_json::{Value, json};
use tauri::test::MockRuntime;
use tauri::{App, Listener, Manager, WebviewWindow};
use tempfile::TempDir;

/// 状態が変わるのを待つ最長の時間。
const WAIT: Duration = Duration::from_secs(30);

/// 前の初回準備が終わった日時（テストで書く `prepared.json` の `prepared_at`）。
const PREPARED_AT: &str = "2026-11-24T19:02:00+09:00";

/// テストごとの学習者データのディレクトリと同梱の許可クレートの定義を置き、`fake` の `cargo` を探すアプリ。
struct Fixture {
    _dirs: Vec<TempDir>,
    app: App<MockRuntime>,
    window: WebviewWindow<MockRuntime>,
    data_dir: AppDataDir,
    runner: RunnerDir,
    /// 受け取った `environment_updated` の内容。
    updates: mpsc::Receiver<Value>,
}

impl Fixture {
    /// アプリの状態（学習者データのディレクトリ・同梱の許可クレートの定義）を置き、`environment_updated` を受け取る
    /// 用意をする。起動時の検出は、まだ始めない。
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
            updates,
        }
    }

    fn workspace(&self) -> WorkspaceDir {
        WorkspaceDir::in_app_data(&self.data_dir)
    }

    /// `fake` の `rustc` と、今の一覧で初回準備が済んだ `prepared.json` を書く。
    fn write_ready_record(&self, fake: &FakeRust) {
        self.write_record(PreparedRecord::ready(
            crates_hash(&self.runner).unwrap(),
            rustc_verbose_version(fake.rustc()).unwrap(),
            PREPARED_AT,
        ));
    }

    fn write_record(&self, record: PreparedRecord) {
        write_prepared(&self.workspace(), &record).unwrap();
    }

    /// アプリの起動時の検出を、`fake` の `cargo` だけがある場所で始める（`fake` が `None` なら、何も無い場所）。
    fn start(&mut self, fake: Option<&FakeRust>) {
        let place = TempDir::new().unwrap();
        let dir = fake.map_or(place.path(), |fake| fake.cargo().parent().unwrap());
        let search = ToolSearch {
            cargo: CargoSearch {
                configured_cargo: None,
                path_var: Some(std::env::join_paths([dir]).unwrap()),
                home: Some(place.path().to_path_buf()),
            },
            claude: ClaudeSearch::nowhere(),
        };
        start_detection(self.app.handle(), search);
        self._dirs.push(place);
    }

    fn invoke(&self, cmd: &str) -> Result<Value, Value> {
        invoke(&self.window, LOCAL_URL, cmd, json!({}))
    }

    /// `get_environment_status` の `crates`。
    fn crates(&self) -> Value {
        self.invoke("get_environment_status").unwrap()["crates"].clone()
    }

    fn environment(&self) -> Environment {
        self.app.state::<Environment>().inner().clone()
    }

    /// `crates.status` が `status` の `environment_updated` を受け取るまで待ち、その `crates` を返す。
    fn wait_update(&self, status: &str) -> Value {
        let deadline = Instant::now() + WAIT;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            let payload = self
                .updates
                .recv_timeout(left)
                .unwrap_or_else(|_| panic!("crates が {status} の environment_updated が届かない"));
            if payload["crates"]["status"] == status {
                return payload["crates"].clone();
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

fn tools_of(fake: &FakeRust) -> RustTools {
    RustTools {
        cargo: fake.cargo().to_path_buf(),
        rustc: fake.rustc().to_path_buf(),
        rustc_version: "1.100.0".parse().unwrap(),
    }
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

/// プロセス `pid` が終わって刈り取られるまで待つ。
fn wait_gone(pid: u32) {
    let pid = libc::pid_t::try_from(pid).unwrap();
    let deadline = Instant::now() + WAIT;
    // SAFETY: シグナル 0 は、プロセスがあるかを確かめるだけで、何も送らない。
    while unsafe { libc::kill(pid, 0) } == 0 {
        assert!(Instant::now() < deadline, "cargo（{pid}）が止まらない");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn error_code(response: Result<Value, Value>) -> String {
    let error = response.expect_err("エラーにならない");
    error["code"].as_str().unwrap().to_string()
}

// ---- 起動時の判定 ----

/// `cargo` が見つからないときは、判定せず `not_started` のまま。作業領域も作らない。
#[test]
fn without_cargo_the_preparation_is_not_judged() {
    let mut fx = Fixture::new();
    fx.start(None);
    assert_eq!(fx.crates(), crates_json("not_started"));
    assert!(!fx.workspace().root().exists());
}

/// `prepared.json` が無いときは、起動時に初回準備を始める。`get_environment_status` の最初の状態から `preparing`。
#[test]
fn without_prepared_json_the_preparation_starts() {
    let mut fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    set_behavior(fake.cargo(), "hang", "hang");
    fx.start(Some(&fake));
    assert_eq!(fx.crates(), crates_json("preparing"));
    assert_eq!(fx.wait_update("preparing"), crates_json("preparing"));
    wait_hanging(&fake);
    assert_eq!(cargo_calls(fake.cargo()), ["build --quiet"]);
}

/// 「実行する条件」のどれにも当てはまらないときは、初回準備をせずに `ready` にし、`prepared_at` を返す。
#[test]
fn a_matching_prepared_json_is_ready_without_preparing() {
    let mut fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    set_behavior(fake.cargo(), "hang", "hang");
    fx.write_ready_record(&fake);
    fx.start(Some(&fake));
    let ready = json!({ "status": "ready", "prepared_at": PREPARED_AT, "error": null });
    assert_eq!(fx.crates(), ready);
    assert_eq!(fx.wait_update("ready"), ready);
    assert!(cargo_calls(fake.cargo()).is_empty());
    assert_eq!(fx.environment().crates_status().status, CratesState::Ready);
}

/// 初回準備が済んだ記録を受け取り、条件に当てはまるように書き換えて書く。
type WriteRecord = fn(&Fixture, PreparedRecord);

/// 「実行する条件」のそれぞれに当てはまる `prepared.json` では、起動時に初回準備を始める。
#[test]
fn each_condition_starts_the_preparation() {
    let fake = FakeRust::new("1.100.0");
    let ready = |fx: &Fixture| {
        PreparedRecord::ready(
            crates_hash(&fx.runner).unwrap(),
            rustc_verbose_version(fake.rustc()).unwrap(),
            PREPARED_AT,
        )
    };
    let cases: [(&str, WriteRecord); 3] = [
        ("crates_hash が異なる", |fx, record| {
            fx.write_record(PreparedRecord {
                crates_hash: "0".repeat(64),
                ..record
            })
        }),
        ("status が failed", |fx, record| {
            fx.write_record(PreparedRecord {
                status: PreparedStatus::Failed,
                error: Some("前の失敗".into()),
                ..record
            })
        }),
        ("読み込めない prepared.json", |fx, _| {
            fs::create_dir_all(fx.workspace().root()).unwrap();
            fs::write(fx.workspace().prepared_json(), "{ 壊れた").unwrap()
        }),
    ];
    for (case, write) in cases {
        let mut fx = Fixture::new();
        let fake = FakeRust::new("1.100.0");
        set_behavior(fake.cargo(), "hang", "hang");
        write(&fx, ready(&fx));
        fx.start(Some(&fake));
        assert_eq!(fx.crates(), crates_json("preparing"), "{case}");
        wait_hanging(&fake);
    }
}

/// `rustc -vV` の出力が変わった後（`rustup update` など）の起動では、初回準備をやり直す（ADR 0100）。
#[test]
fn a_changed_rustc_starts_the_preparation_again() {
    let mut fx = Fixture::new();
    // 前の初回準備は rustc 1.100.0 で済ませた。
    let before = FakeRust::new("1.100.0");
    fx.write_ready_record(&before);
    assert_eq!(
        judge(before.rustc(), &fx.runner, &fx.workspace()),
        Judgment::Ready {
            prepared_at: PREPARED_AT.into()
        }
    );

    // rustc を 1.101.0 にしてから起動する。
    let fake = FakeRust::new("1.101.0");
    set_behavior(fake.cargo(), "fail", "fail");
    fx.start(Some(&fake));
    fx.wait_update("failed");
    // 初回準備を動かし、今の rustc の出力を記録した。
    assert_eq!(
        cargo_calls(fake.cargo()),
        ["build --quiet"],
        "初回準備の cargo build を動かしていない"
    );
    let record = read_prepared(&fx.workspace()).unwrap().unwrap();
    assert_eq!(
        Some(record.rustc_version),
        rustc_verbose_version(fake.rustc())
    );
    assert!(
        rustc_verbose_version(fake.rustc())
            .unwrap()
            .contains("1.101.0")
    );
}

// ---- 失敗と再試行 ----

/// 初回準備に失敗すると `failed` になり、エラーの内容を返す。`retry_crate_preparation` で始め直せる。
#[test]
fn a_failed_preparation_can_be_retried() {
    let mut fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    set_behavior(fake.cargo(), "succeed", "fail");
    fx.start(Some(&fake));
    let failed = fx.wait_update("failed");
    let error = failed["error"].as_str().unwrap();
    assert!(
        error.starts_with("`cargo test --no-run`が失敗しました（終了コード 101）。"),
        "{error}"
    );
    assert_eq!(failed["prepared_at"], Value::Null);
    fx.wait_finished();
    assert_eq!(fx.crates(), failed);
    // prepared.json にも同じエラーの内容を書いた。
    let record = read_prepared(&fx.workspace()).unwrap().unwrap();
    assert_eq!(record.status, PreparedStatus::Failed);
    assert_eq!(record.error.as_deref(), Some(error));

    // 再試行では、始めたら null を返し、経過は environment_updated で知らせる。
    set_behavior(fake.cargo(), "hang", "hang");
    assert_eq!(fx.invoke("retry_crate_preparation"), Ok(Value::Null));
    assert_eq!(fx.crates(), crates_json("preparing"));
    assert_eq!(fx.wait_update("preparing"), crates_json("preparing"));
    wait_hanging(&fake);
    assert_eq!(
        cargo_calls(fake.cargo()),
        ["build --quiet", "test --no-run --quiet", "build --quiet"]
    );
}

/// 再試行でも失敗すると、また `failed` になり、もう一度やり直せる。
#[test]
fn a_retry_that_fails_again_is_failed() {
    let mut fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    set_behavior(fake.cargo(), "fail", "fail");
    fx.start(Some(&fake));
    fx.wait_update("failed");
    fx.wait_finished();
    for _ in 0..2 {
        assert_eq!(fx.invoke("retry_crate_preparation"), Ok(Value::Null));
        fx.wait_update("preparing");
        fx.wait_update("failed");
        fx.wait_finished();
    }
    assert_eq!(cargo_calls(fake.cargo()).len(), 3);
}

/// 初回準備を `ready` まで進める。手順の 4（`cp -c`）があるので、macOS のときだけ実行する。
#[cfg(target_os = "macos")]
#[test]
fn a_retried_preparation_becomes_ready() {
    let mut fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    set_behavior(fake.cargo(), "fail", "fail");
    fx.start(Some(&fake));
    fx.wait_update("failed");
    fx.wait_finished();

    set_behavior(fake.cargo(), "succeed", "succeed");
    assert_eq!(fx.invoke("retry_crate_preparation"), Ok(Value::Null));
    let ready = fx.wait_update("ready");
    let record = read_prepared(&fx.workspace()).unwrap().unwrap();
    assert_eq!(record.status, PreparedStatus::Ready);
    assert_eq!(
        ready,
        json!({ "status": "ready", "prepared_at": record.prepared_at, "error": null })
    );
    assert!(fx.workspace().target_validate().is_dir());
    fx.wait_finished();
    assert_eq!(fx.crates(), ready);
    // 準備が済んだので、許可クレートを使う演習を受け付ける。
    assert_eq!(fx.environment().ensure_crates_ready(), Ok(()));
    // 次の起動では、初回準備をしない。
    assert_eq!(
        judge(fake.rustc(), &fx.runner, &fx.workspace()),
        Judgment::Ready {
            prepared_at: record.prepared_at
        }
    );
}

/// 再試行は、初回準備の状態が `failed` のときだけ受け付ける。ほかの状態では `invalid_state`。
#[test]
fn a_retry_when_not_failed_is_invalid_state() {
    // ready
    let mut fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    set_behavior(fake.cargo(), "hang", "hang");
    fx.write_ready_record(&fake);
    fx.start(Some(&fake));
    assert_eq!(
        error_code(fx.invoke("retry_crate_preparation")),
        "invalid_state"
    );
    assert!(cargo_calls(fake.cargo()).is_empty());

    // preparing（重ねて始めない）
    let mut fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    set_behavior(fake.cargo(), "hang", "hang");
    fx.start(Some(&fake));
    wait_hanging(&fake);
    assert_eq!(
        error_code(fx.invoke("retry_crate_preparation")),
        "invalid_state"
    );
    assert_eq!(cargo_calls(fake.cargo()), ["build --quiet"]);

    // not_started（cargo を直接入れて、判定していない）
    let fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    fx.app.manage(Environment::detected(DetectedTools {
        cargo: Some(tools_of(&fake)),
        claude: None,
    }));
    assert_eq!(
        error_code(fx.invoke("retry_crate_preparation")),
        "invalid_state"
    );
    assert!(cargo_calls(fake.cargo()).is_empty());
}

/// `cargo` が見つからないときの再試行は `cargo_unavailable`。
#[test]
fn a_retry_without_cargo_is_cargo_unavailable() {
    let mut fx = Fixture::new();
    fx.start(None);
    assert_eq!(
        error_code(fx.invoke("retry_crate_preparation")),
        "cargo_unavailable"
    );

    // 失敗した後に cargo が見つからなくなったとき（再検出）も、cargo_unavailable。
    let fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    set_behavior(fake.cargo(), "fail", "fail");
    let environment = Environment::detected(DetectedTools {
        cargo: Some(tools_of(&fake)),
        claude: None,
    });
    fx.app.manage(environment.clone());
    assert!(start_crate_preparation(fx.app.handle(), &tools_of(&fake)));
    fx.wait_update("failed");
    fx.wait_finished();
    environment.finish_detection(DetectedTools::default());
    assert_eq!(fx.crates()["status"], "failed");
    assert_eq!(
        error_code(fx.invoke("retry_crate_preparation")),
        "cargo_unavailable"
    );
    assert_eq!(cargo_calls(fake.cargo()).len(), 1);
}

// ---- 1 か所の開始 ----

/// 初回準備を始める処理は、`preparing` のときに重ねて始めない。
#[test]
fn the_preparation_does_not_start_twice() {
    let fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    set_behavior(fake.cargo(), "hang", "hang");
    fx.app.manage(Environment::detected(DetectedTools {
        cargo: Some(tools_of(&fake)),
        claude: None,
    }));
    assert_eq!(fx.crates(), crates_json("not_started"));
    assert!(start_crate_preparation(fx.app.handle(), &tools_of(&fake)));
    assert_eq!(fx.wait_update("preparing"), crates_json("preparing"));
    wait_hanging(&fake);
    assert!(!start_crate_preparation(fx.app.handle(), &tools_of(&fake)));
    assert_eq!(cargo_calls(fake.cargo()), ["build --quiet"]);
    assert_eq!(fx.crates(), crates_json("preparing"));
}

// ---- アプリの終了 ----

/// アプリを終了するときに初回準備が動いていたら、`cargo` を止め、`prepared.json` を書かない（ADR 0236）。
#[test]
fn exiting_the_app_stops_the_preparation_without_writing_prepared_json() {
    let mut fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    set_behavior(fake.cargo(), "hang", "hang");
    fx.start(Some(&fake));
    let pid = wait_hanging(&fake);

    stop_crate_preparation_on_exit(fx.app.handle());

    // 止めた初回準備のスレッドは、待つ間に終わっている。
    assert!(fx.environment().wait_preparation_finished(Duration::ZERO));
    wait_gone(pid);
    assert!(!fx.workspace().prepared_json().exists());
    assert_eq!(read_prepared(&fx.workspace()).unwrap(), None);
    // 終了するところでは、初回準備を新しく始めない。
    assert!(!start_crate_preparation(fx.app.handle(), &tools_of(&fake)));
    assert_eq!(cargo_calls(fake.cargo()), ["build --quiet"]);
}

/// 前に失敗していた `prepared.json` は、終了で止めても書き換えず、次の起動でやり直す。
#[test]
fn exiting_the_app_keeps_the_previous_prepared_json() {
    let mut fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    set_behavior(fake.cargo(), "hang", "hang");
    let previous = PreparedRecord::failed(
        crates_hash(&fx.runner).unwrap(),
        rustc_verbose_version(fake.rustc()).unwrap(),
        PREPARED_AT,
        "前の失敗",
    );
    fx.write_record(previous.clone());
    fx.start(Some(&fake));
    let pid = wait_hanging(&fake);

    stop_crate_preparation_on_exit(fx.app.handle());
    wait_gone(pid);

    assert_eq!(read_prepared(&fx.workspace()).unwrap(), Some(previous));
    assert_eq!(
        judge(fake.rustc(), &fx.runner, &fx.workspace()),
        Judgment::Needed
    );
}

/// 初回準備が動いていないときの終了は、何もせずにすぐ返る。
#[test]
fn exiting_the_app_without_a_preparation_returns_at_once() {
    let mut fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    fx.write_ready_record(&fake);
    fx.start(Some(&fake));
    assert_eq!(fx.crates()["status"], "ready");
    let started = Instant::now();
    stop_crate_preparation_on_exit(fx.app.handle());
    assert!(started.elapsed() < Duration::from_secs(1));
    assert_eq!(fx.crates()["status"], "ready");
}

// ---- 判定 ----

/// 判定に使うものが読めないときは、初回準備をする（初回準備が、その理由を失敗として記録する）。
#[test]
fn unreadable_inputs_need_the_preparation() {
    let fx = Fixture::new();
    let fake = FakeRust::new("1.100.0");
    fx.write_ready_record(&fake);
    let workspace = fx.workspace();
    assert!(matches!(
        judge(fake.rustc(), &fx.runner, &workspace),
        Judgment::Ready { .. }
    ));
    // rustc -vV を実行できない。
    assert_eq!(
        judge(
            &Path::new("/nonexistent").join("rustc"),
            &fx.runner,
            &workspace
        ),
        Judgment::Needed
    );
    // 同梱の Cargo.lock が無い。
    fs::remove_file(fx.runner.cargo_lock()).unwrap();
    assert_eq!(
        judge(fake.rustc(), &fx.runner, &workspace),
        Judgment::Needed
    );
}
