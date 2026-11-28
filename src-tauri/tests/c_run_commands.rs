//! C の演習の実行（`run_code`。spec/exercise-runner/c-run.md、spec/architecture/commands/run.md「run_code」、
//! spec/exercise-runner/attempt-log.md「C の演習」）を、モックの実行環境で確かめる。
//!
//! - どの OS でも実行するテスト: 受け付ける前に確かめること（`mode`、ファイルパス、`clang`）と、`cargo` の有無と
//!   初回準備の状態に左右されないこと。ビルドは偽の `clang` で行う（偽の `clang` はビルドに失敗する）。
//! - macOS のときだけ実行するテスト: 実物の `clang` でビルドして動かす実行（spec/non-functional/testing.md「CI」）。

mod common;
#[path = "fake_tools/cargo_behavior.rs"]
mod fake_cargo;
#[path = "fake_tools/rust.rs"]
mod fake_rust;

use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::symlink,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use common::{LOCAL_URL, app, invoke};
use fake_cargo::{cargo_calls, hanging_pid, set_behavior};
use fake_rust::FakeRust;
use progrust_lib::attempts::{AttemptLog, Event, RunEvent};
use progrust_lib::content::ContentDir;
use progrust_lib::environment::preparation::{
    start_crate_preparation, stop_crate_preparation_on_exit,
};
use progrust_lib::environment::{ClangTool, CratesState, DetectedTools, Environment, RustTools};
use progrust_lib::runner::RunnerDir;
use progrust_lib::runner::run::{RUN_FINISHED, RUN_OUTPUT, RUN_STARTED};
use progrust_lib::runner::runs::{RunLimits, Runs};
use progrust_lib::storage::AppDataDir;
use serde_json::{Value, json};
use tauri::test::MockRuntime;
use tauri::{App, Listener, Manager, WebviewWindow};
use tempfile::TempDir;

/// 状態が変わるのを待つ最長の時間。
const WAIT: Duration = Duration::from_secs(60);

/// C の演習。
const C_LESSON: &str = "c01-01";
/// Rust の演習（実行・提出を C の演習と合わせて 1 つだけ行うことを確かめる）。
const RUST_LESSON: &str = "ch01-01";

const MAIN_C: &str =
    "#include <stdio.h>\nint main(void) {\n    printf(\"hello\\n\");\n    return 0;\n}\n";

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

/// C の章 c01 に C の演習 [`C_LESSON`]、Rust の章 ch01 に Rust の演習 [`RUST_LESSON`] がある教材。
fn make_content() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "chapters.json",
        &json!([{
            "id": "c01",
            "title": "C の第1章",
            "description": "",
            "language": "c",
            "lessons": [C_LESSON],
        }, {
            "id": "ch01",
            "title": "第1章",
            "description": "",
            "lessons": [RUST_LESSON],
        }])
        .to_string(),
    );
    write(
        root,
        &format!("lessons/{C_LESSON}/lesson.json"),
        &json!({
            "id": C_LESSON,
            "kind": "lesson",
            "title": C_LESSON,
            "exercise": {
                "layout": "single",
                "judge": { "mode": "stdout", "target": "main.c" },
                "readonly_files": [],
                "case_labels": {},
            },
        })
        .to_string(),
    );
    write(
        root,
        &format!("lessons/{RUST_LESSON}/lesson.json"),
        &json!({
            "id": RUST_LESSON,
            "kind": "lesson",
            "title": RUST_LESSON,
            "exercise": {
                "layout": "single",
                "judge": { "mode": "stdout", "target": "src/main.rs" },
                "dependencies": [],
                "readonly_files": [],
                "case_labels": {},
            },
        })
        .to_string(),
    );
    dir
}

/// 受け取ったイベント。
enum Received {
    Started(Value),
    Output(Value),
    /// `run_finished` と、それを受け取った時点の試行ログ（レッスン [`C_LESSON`] の未完了のセッション）。
    Finished(Value, Option<AttemptLog>),
}

struct Fixture {
    _content: TempDir,
    data: TempDir,
    app: App<MockRuntime>,
    window: WebviewWindow<MockRuntime>,
    received: mpsc::Receiver<Received>,
}

impl Fixture {
    /// `cargo` が見つからず、`clang` が `clang` の状態。
    fn new(clang: Option<ClangTool>, limits: RunLimits) -> Self {
        let content = make_content();
        let data = tempfile::tempdir().unwrap();
        let (app, window) = app();
        app.manage(ContentDir::new(content.path()));
        app.manage(AppDataDir::new(data.path()));
        app.manage(RunnerDir::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../runner"
        )));
        app.manage(Environment::detected(DetectedTools {
            cargo: None,
            claude: None,
            clang,
        }));
        app.manage(Runs::new(limits));

        let (sender, received) = mpsc::channel();
        {
            let sender = sender.clone();
            app.listen(RUN_STARTED, move |event| {
                let payload = serde_json::from_str(event.payload()).unwrap();
                let _ = sender.send(Received::Started(payload));
            });
        }
        {
            let sender = sender.clone();
            app.listen(RUN_OUTPUT, move |event| {
                let payload = serde_json::from_str(event.payload()).unwrap();
                let _ = sender.send(Received::Output(payload));
            });
        }
        let data_path = data.path().to_path_buf();
        app.listen(RUN_FINISHED, move |event| {
            let payload = serde_json::from_str(event.payload()).unwrap();
            let log = AppDataDir::new(&data_path)
                .unfinished_session(C_LESSON)
                .unwrap();
            let _ = sender.send(Received::Finished(payload, log));
        });
        Self {
            _content: content,
            data,
            app,
            window,
            received,
        }
    }

    fn invoke(&self, cmd: &str, args: Value) -> Result<Value, Value> {
        invoke(&self.window, LOCAL_URL, cmd, args)
    }

    fn run_code(
        &self,
        request_id: &str,
        mode: &str,
        files: &[(&str, &str)],
    ) -> Result<Value, Value> {
        let files: BTreeMap<&str, &str> = files.iter().copied().collect();
        self.invoke(
            "run_code",
            json!({ "request_id": request_id, "lesson_id": C_LESSON, "mode": mode, "files": files }),
        )
    }

    /// `run_finished` を受け取るまで、イベントを受け取る。受け取った出力は `outputs` に足す。
    fn wait_finished_after(&self, request_id: &str, mut outputs: Vec<Value>) -> Finished {
        let mut started = 0;
        loop {
            match self
                .received
                .recv_timeout(WAIT)
                .expect("run_finished が届かない")
            {
                Received::Started(payload) => {
                    assert_eq!(payload, json!({ "request_id": request_id }));
                    started += 1;
                }
                Received::Output(output) => {
                    assert_eq!(output["request_id"], request_id);
                    outputs.push(output);
                }
                Received::Finished(payload, log) => {
                    assert_eq!(payload["request_id"], request_id);
                    return Finished {
                        payload,
                        outputs,
                        started,
                        log,
                    };
                }
            }
        }
    }

    fn wait_finished(&self, request_id: &str) -> Finished {
        self.wait_finished_after(request_id, Vec::new())
    }

    fn runs(&self) -> Runs {
        self.app.state::<Runs>().inner().clone()
    }

    fn workspace(&self) -> PathBuf {
        self.data.path().join("workspace")
    }

    fn draft(&self) -> Option<progrust_lib::drafts::Draft> {
        AppDataDir::new(self.data.path())
            .read_draft(C_LESSON)
            .unwrap()
    }
}

struct Finished {
    payload: Value,
    outputs: Vec<Value>,
    /// `run_finished` までに受け取った `run_started` の数。
    started: usize,
    log: Option<AttemptLog>,
}

impl Finished {
    fn status(&self) -> &Value {
        &self.payload["run"]["status"]
    }

    #[cfg(target_os = "macos")]
    fn exit_code(&self) -> &Value {
        &self.payload["run"]["exit_code"]
    }

    fn text(&self, stream: &str) -> String {
        text_of(&self.outputs, stream)
    }

    #[cfg(target_os = "macos")]
    fn chunks(&self) -> Vec<Value> {
        self.outputs
            .iter()
            .flat_map(|output| output["chunks"].as_array().unwrap().clone())
            .collect()
    }

    /// `system` の断片の文字列。
    #[cfg(target_os = "macos")]
    fn system_chunks(&self) -> Vec<String> {
        self.chunks()
            .into_iter()
            .filter(|chunk| chunk["stream"] == "system")
            .map(|chunk| chunk["text"].as_str().unwrap().to_string())
            .collect()
    }

    fn run_events(&self) -> Vec<RunEvent> {
        run_events(self.log.clone())
    }
}

fn text_of(outputs: &[Value], stream: &str) -> String {
    outputs
        .iter()
        .flat_map(|output| output["chunks"].as_array().unwrap())
        .filter(|chunk| chunk["stream"] == stream)
        .map(|chunk| chunk["text"].as_str().unwrap())
        .collect()
}

fn run_events(log: Option<AttemptLog>) -> Vec<RunEvent> {
    log.map(|log| log.events)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|event| match event {
            Event::Run(run) => Some(run),
            _ => None,
        })
        .collect()
}

fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "{what}");
        thread::sleep(Duration::from_millis(20));
    }
}

fn error_code(result: Result<Value, Value>) -> Value {
    result.unwrap_err()["code"].clone()
}

/// 一時ディレクトリに置いた偽の `clang`（tests/fake_tools/clang を指すシンボリックリンク）。ビルドの引数には
/// 対応せず、`fake clang: 対応していない引数` を標準エラー出力に書いて、終了コード 1 で失敗する。偽の `clang` の
/// 手助け（fake_tools/clang.rs）は、ここで使わない機能が多いので読み込まない。
struct FakeClang {
    _dir: TempDir,
    path: PathBuf,
}

impl FakeClang {
    fn new() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let original = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fake_tools/clang");
        let path = dir.path().join("clang");
        symlink(original, &path).unwrap();
        Self { _dir: dir, path }
    }

    /// 検出した `clang` として渡すもの。
    fn tool(&self) -> ClangTool {
        ClangTool {
            path: self.path.clone(),
            version: Some("21.0.0".into()),
        }
    }

    /// これまでの呼び出しの引数と環境変数（`env` の出力の行）。呼び出した順に並ぶ。
    fn calls(&self) -> Vec<(Vec<String>, Vec<String>)> {
        let calls_dir = self.path.with_file_name("calls");
        (1..)
            .map(|number| calls_dir.join(number.to_string()))
            .take_while(|dir| dir.is_dir())
            .map(|dir| {
                let read = |name: &str| fs::read_to_string(dir.join(name)).unwrap();
                (
                    read("args").lines().map(String::from).collect(),
                    read("env").lines().map(String::from).collect(),
                )
            })
            .collect()
    }
}

/// `claude` に渡す 6 つのほかの環境変数として、cargo がテストのプロセスに渡す `CARGO_` で始まるものを、
/// `名前=値` の形で返す。値に改行を含むものは、行ごとに比べられないので除く。
fn cargo_env_lines() -> Vec<String> {
    let lines: Vec<String> = std::env::vars()
        .filter(|(name, value)| name.starts_with("CARGO_") && !value.contains('\n'))
        .map(|(name, value)| format!("{name}={value}"))
        .collect();
    assert!(
        !lines.is_empty(),
        "cargo がテストに渡す環境変数が無く、確かめられない"
    );
    lines
}

// ---- 受け付ける前に確かめること（どの OS でも実行する） ----

#[test]
fn test_mode_is_invalid_for_c_exercises_even_without_clang() {
    let fake = FakeClang::new();
    for clang in [Some(fake.tool()), None] {
        let fx = Fixture::new(clang, RunLimits::default());
        assert_eq!(
            error_code(fx.run_code("r1", "test", &[("main.c", MAIN_C)])),
            "invalid_argument"
        );
        assert!(fx.runs().current().is_none());
        assert!(fx.draft().is_none());
        assert!(!fx.workspace().exists());
    }
    // clang を起動していない。
    assert!(fake.calls().is_empty());
}

#[test]
fn paths_other_than_main_c_are_invalid_even_without_clang_and_write_nothing() {
    let fake = FakeClang::new();
    let outside = tempfile::tempdir().unwrap();
    let outside_file = outside.path().join("outside.c");
    for clang in [Some(fake.tool()), None] {
        let fx = Fixture::new(clang, RunLimits::default());
        let absolute = outside_file.to_string_lossy().into_owned();
        for extra in [
            "../outside.c",
            "../../outside.c",
            absolute.as_str(),
            "src/main.rs",
            "main.h",
        ] {
            assert_eq!(
                error_code(fx.run_code("r1", "main", &[("main.c", MAIN_C), (extra, "// 外")])),
                "invalid_argument",
                "{extra}"
            );
        }
        // main.c が無いときも拒否する。
        assert_eq!(
            error_code(fx.run_code("r1", "main", &[("src/main.rs", "fn main() {}\n")])),
            "invalid_argument"
        );
        assert!(fx.runs().current().is_none());
        assert!(fx.draft().is_none());
        assert!(!fx.workspace().exists());
        assert!(!fx.data.path().join("outside.c").exists());
    }
    assert!(!outside_file.exists());
    assert!(fake.calls().is_empty());
}

#[test]
fn without_clang_a_valid_request_is_clang_unavailable() {
    let fx = Fixture::new(None, RunLimits::default());
    assert_eq!(
        error_code(fx.run_code("r1", "main", &[("main.c", MAIN_C)])),
        "clang_unavailable"
    );
    assert!(fx.runs().current().is_none());
    assert!(!fx.workspace().exists());
    // 下書きは、確かめる前に保存している（Rust の演習の cargo_unavailable と同じ）。
    assert_eq!(fx.draft().unwrap().files["main.c"], MAIN_C);
}

/// C の演習は、`cargo` が見つからなくても、初回準備の状態（`not_started`・`preparing`・`failed`）にかかわらず、
/// `clang` で実行できる。ここでは偽の `clang` でビルドするので、ビルドの失敗（`compile_error`）で終わる。
#[test]
fn c_exercises_run_without_cargo_in_any_preparation_state() {
    let fake = FakeClang::new();
    let fx = Fixture::new(Some(fake.tool()), RunLimits::default());
    let environment = fx.app.state::<Environment>().inner().clone();

    let run = |request_id: &str| {
        assert_eq!(
            fx.run_code(request_id, "main", &[("main.c", MAIN_C)]),
            Ok(Value::Null)
        );
        let finished = fx.wait_finished(request_id);
        assert_eq!(
            finished.payload["error"],
            Value::Null,
            "{}",
            finished.payload
        );
        assert_eq!(finished.status(), "compile_error");
        assert_eq!(finished.started, 0);
        // 偽の clang の出力を、ビルドの出力として送る。
        assert!(
            finished.text("stderr").contains("fake clang"),
            "{}",
            finished.text("stderr")
        );
        assert!(fx.runs().current().is_none());
        finished
    };

    // not_started。
    assert_eq!(environment.crates_status().status, CratesState::NotStarted);
    let finished = run("r1");
    // 検査なしのビルドの引数で、clang を起動した。
    let calls = fake.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(
        calls[0].0,
        [
            "-std=c17",
            "-g",
            "-O0",
            "-Wall",
            "-Wextra",
            "-fcolor-diagnostics",
            "-o",
            "main",
            "main.c",
        ]
    );
    // clang には、アプリが受け取った環境変数をそのまま渡す（`claude` に渡す 6 つに絞らない）。
    for line in cargo_env_lines() {
        assert!(calls[0].1.contains(&line), "{line} が clang に渡っていない");
    }
    // `c-project/` は {app_data_dir}/workspace/c-project/。Rust の演習の project/ は作らない。
    assert_eq!(
        fs::read_to_string(fx.workspace().join("c-project/main.c")).unwrap(),
        MAIN_C
    );
    assert!(!fx.workspace().join("project").exists());
    // 試行ログに記録した。
    let events = finished.run_events();
    assert_eq!(events.len(), 1);
    let event = serde_json::to_value(&events[0]).unwrap();
    assert_eq!(event["mode"], "main");
    assert_eq!(event["files"], json!({ "main.c": MAIN_C }));
    assert_eq!(event["result"]["status"], "compile_error");
    assert_eq!(event["result"]["error_codes"], json!([]));

    // failed: 初回準備を、失敗する偽の cargo で動かす。
    let failing = FakeRust::new("1.100.0");
    set_behavior(failing.cargo(), "fail", "fail");
    assert!(start_crate_preparation(
        fx.app.handle(),
        &rust_tools(&failing)
    ));
    wait_until("初回準備が失敗しない", || {
        environment.crates_status().status == CratesState::Failed
    });
    run("r2");

    // preparing: 初回準備を、止めるまで終わらない偽の cargo で動かす。
    let hanging = FakeRust::new("1.100.0");
    // 偽の cargo の一時ディレクトリを消す前に、初回準備を止める（後に宣言したものから先に drop する）。
    let _stop = StopPreparationOnDrop(&fx);
    set_behavior(hanging.cargo(), "hang", "hang");
    assert!(start_crate_preparation(
        fx.app.handle(),
        &rust_tools(&hanging)
    ));
    wait_until("初回準備の cargo が動き出さない", || {
        hanging_pid(hanging.cargo()).is_some()
    });
    assert_eq!(environment.crates_status().status, CratesState::Preparing);
    let cargo_calls_before = (cargo_calls(failing.cargo()), cargo_calls(hanging.cargo()));
    run("r3");
    // C の演習の実行では、cargo を起動しない。
    assert_eq!(
        (cargo_calls(failing.cargo()), cargo_calls(hanging.cargo())),
        cargo_calls_before
    );
    assert_eq!(environment.crates_status().status, CratesState::Preparing);
    assert_eq!(fake.calls().len(), 3);
}

fn rust_tools(fake: &FakeRust) -> RustTools {
    RustTools {
        cargo: fake.cargo().to_path_buf(),
        rustc: fake.rustc().to_path_buf(),
        rustc_version: "1.100.0".parse().unwrap(),
    }
}

/// テストが途中で失敗しても、終わらない偽の cargo を残さない。
struct StopPreparationOnDrop<'a>(&'a Fixture);

impl Drop for StopPreparationOnDrop<'_> {
    fn drop(&mut self) {
        stop_crate_preparation_on_exit(self.0.app.handle());
    }
}

// ---- 実物の clang での実行（macOS のときだけ実行する） ----

#[cfg(target_os = "macos")]
mod macos {
    use super::*;

    use std::fs::OpenOptions;
    use std::os::unix::fs::OpenOptionsExt;
    use std::process::Command;

    use progrust_lib::environment::ClangSearch;

    /// 色の指定（`ESC [ … m`）を除く。
    fn strip_colors(text: &str) -> String {
        let mut stripped = String::new();
        let mut chars = text.chars();
        while let Some(c) = chars.next() {
            if c == '\u{1b}' {
                for c in chars.by_ref() {
                    if c.is_ascii_alphabetic() {
                        break;
                    }
                }
            } else {
                stripped.push(c);
            }
        }
        stripped
    }

    fn real_clang() -> ClangTool {
        ClangSearch::from_process(None)
            .detect()
            .expect("clang が見つからない（Command Line Tools を入れる）")
    }

    fn fixture() -> Fixture {
        Fixture::new(Some(real_clang()), RunLimits::default())
    }

    /// 標準出力に `expected` が現れるまで、`run_output` を受け取る。受け取った出力を返す。
    fn wait_stdout(fx: &Fixture, expected: &str) -> Vec<Value> {
        let mut outputs = Vec::new();
        let deadline = Instant::now() + WAIT;
        while !text_of(&outputs, "stdout").contains(expected) {
            let left = deadline.saturating_duration_since(Instant::now());
            match fx.received.recv_timeout(left) {
                Ok(Received::Started(_)) => {}
                Ok(Received::Output(output)) => outputs.push(output),
                Ok(Received::Finished(payload, _)) => {
                    panic!("{expected:?} の前に終わった: {payload}")
                }
                Err(_) => panic!("{expected:?} が届かない: {}", text_of(&outputs, "stdout")),
            }
        }
        outputs
    }

    /// `run_started` を受け取るまで、イベントを受け取る。受け取った出力を返す。
    fn wait_started(fx: &Fixture, request_id: &str) -> Vec<Value> {
        let mut outputs = Vec::new();
        loop {
            match fx.received.recv_timeout(WAIT) {
                Ok(Received::Started(payload)) => {
                    assert_eq!(payload, json!({ "request_id": request_id }));
                    return outputs;
                }
                Ok(Received::Output(output)) => outputs.push(output),
                Ok(Received::Finished(payload, _)) => {
                    panic!("run_started の前に終わった: {payload}")
                }
                Err(_) => panic!("run_started が届かない"),
            }
        }
    }

    /// 警告が 1 つ出て、標準入力の 1 行を読み、標準出力と標準エラー出力に書くプログラム。
    const GREET: &str = "#include <stdio.h>\n\
         int main(void) {\n\
             int unused;\n\
             char line[64];\n\
             printf(\"名前: \");\n\
             fflush(stdout);\n\
             if (fgets(line, sizeof line, stdin) == NULL) {\n\
                 return 1;\n\
             }\n\
             printf(\"こんにちは、%s\", line);\n\
             fprintf(stderr, \"標準エラー出力\\n\");\n\
             return 0;\n\
         }\n";

    #[test]
    fn a_run_builds_without_checks_runs_main_and_is_recorded() {
        let fx = fixture();
        fx.run_code("r1", "main", &[("main.c", GREET)]).unwrap();
        let outputs = wait_started(&fx, "r1");
        // ビルドの出力（警告）は、プログラムを起動する前に stderr で送る。色が付いている。
        let build = text_of(&outputs, "stderr");
        assert!(
            strip_colors(&build).contains("warning: unused variable 'unused'"),
            "{build}"
        );
        assert!(build.contains('\u{1b}'), "{build}");
        fx.invoke(
            "send_stdin",
            json!({ "request_id": "r1", "text": "太郎\n" }),
        )
        .unwrap();
        let finished = fx.wait_finished_after("r1", outputs);

        assert_eq!(finished.payload["kind"], "run");
        assert_eq!(finished.payload["error"], Value::Null);
        assert_eq!(
            finished.payload["run"],
            json!({ "mode": "main", "status": "success", "exit_code": 0 })
        );
        assert_eq!(finished.text("stdout"), "名前: こんにちは、太郎\n");
        assert!(
            finished.text("stderr").ends_with("標準エラー出力\n"),
            "{}",
            finished.text("stderr")
        );
        assert!(finished.system_chunks().is_empty());

        // run_finished を受け取った時点で、試行ログに追記されている。
        let events = finished.run_events();
        assert_eq!(events.len(), 1);
        let event = serde_json::to_value(&events[0]).unwrap();
        assert_eq!(event["at"], finished.payload["finished_at"]);
        assert_eq!(event["mode"], "main");
        assert_eq!(event["files"], json!({ "main.c": GREET }));
        let result = &event["result"];
        assert_eq!(result["status"], "success");
        assert_eq!(result["error_codes"], json!([]));
        assert_eq!(result["stdout"], "名前: こんにちは、太郎\n");
        assert_eq!(result["exit_code"], 0);
        assert_eq!(result["stdin"], "太郎\n");
        // stderr は、ANSI エスケープを除いたビルドの出力と、プログラムの標準エラー出力を、この順に連結したもの。
        let stderr = result["stderr"].as_str().unwrap();
        assert!(
            stderr.starts_with("main.c:3:"),
            "ビルドの出力から始まらない: {stderr}"
        );
        assert!(
            stderr.contains("warning: unused variable 'unused'"),
            "{stderr}"
        );
        assert!(!stderr.contains('\u{1b}'), "{stderr}");
        assert!(stderr.ends_with("標準エラー出力\n"), "{stderr}");

        // 実行の前に下書きを保存した。実行ファイルは c-project/main。
        assert_eq!(
            fx.draft().unwrap().files,
            BTreeMap::from([("main.c".to_string(), GREET.to_string())])
        );
        assert!(fx.workspace().join("c-project/main").is_file());
        assert!(fx.runs().current().is_none());
    }

    #[test]
    fn compile_errors_send_the_clang_output_and_are_recorded() {
        let fx = fixture();
        let code = "int main(void) {\n    return undefined_name;\n}\n";
        fx.run_code("r1", "main", &[("main.c", code)]).unwrap();
        let finished = fx.wait_finished("r1");

        assert_eq!(finished.status(), "compile_error");
        assert_eq!(finished.exit_code(), &Value::Null);
        assert_eq!(finished.started, 0);
        let stderr = finished.text("stderr");
        assert!(stderr.contains("error:"), "{stderr}");
        assert!(stderr.contains("undefined_name"), "{stderr}");
        assert_eq!(finished.text("stdout"), "");

        let events = finished.run_events();
        let event = serde_json::to_value(&events[0]).unwrap();
        assert_eq!(event["result"]["status"], "compile_error");
        assert_eq!(event["result"]["error_codes"], json!([]));
        assert_eq!(event["result"]["exit_code"], Value::Null);
        let recorded = event["result"]["stderr"].as_str().unwrap();
        assert!(recorded.contains("undefined_name"), "{recorded}");
        assert!(!recorded.contains('\u{1b}'), "{recorded}");
    }

    #[test]
    fn a_nonzero_exit_code_is_a_runtime_error_without_a_signal_line() {
        let fx = fixture();
        let code = "int main(void) {\n    return 3;\n}\n";
        fx.run_code("r1", "main", &[("main.c", code)]).unwrap();
        let finished = fx.wait_finished("r1");
        assert_eq!(
            finished.payload["run"],
            json!({ "mode": "main", "status": "runtime_error", "exit_code": 3 })
        );
        assert!(finished.system_chunks().is_empty());
        assert_eq!(finished.run_events()[0].result.exit_code, Some(3));
    }

    /// NULL ポインタを読み書きすると SIGSEGV で終わり、出力を送り終えた後に、シグナルの名前と意味の 1 行を送る。
    #[test]
    fn a_program_killed_by_sigsegv_shows_the_signal_line_after_its_output() {
        for access in ["*p = 1;", "printf(\"%d\\n\", *p);"] {
            let fx = fixture();
            let code = format!(
                "#include <stdio.h>\n\
                 #include <stddef.h>\n\
                 int main(void) {{\n\
                     int *p = NULL;\n\
                     printf(\"before\\n\");\n\
                     fflush(stdout);\n\
                     fprintf(stderr, \"err\\n\");\n\
                     {access}\n\
                     printf(\"after\\n\");\n\
                     return 0;\n\
                 }}\n"
            );
            fx.run_code("r1", "main", &[("main.c", &code)]).unwrap();
            let finished = fx.wait_finished("r1");

            assert_eq!(
                finished.payload["run"],
                json!({ "mode": "main", "status": "runtime_error", "exit_code": null }),
                "{access}"
            );
            assert_eq!(finished.text("stdout"), "before\n", "{access}");
            let line = "シグナル SIGSEGV で終了しました（不正なメモリアクセス）";
            assert_eq!(finished.system_chunks(), [line], "{access}");
            // 学習者のプログラムの出力を送り終えた後の、最後の断片である。
            let chunks = finished.chunks();
            assert_eq!(
                chunks.last().unwrap(),
                &json!({ "stream": "system", "text": line }),
                "{access}"
            );

            // 試行ログには、シグナルの 1 行を入れない。
            let event = &finished.run_events()[0];
            assert_eq!(
                serde_json::to_value(event).unwrap()["result"]["status"],
                "runtime_error"
            );
            assert_eq!(event.result.exit_code, None);
            assert_eq!(event.result.stdout, "before\n");
            assert!(
                !event.result.stderr.contains("SIGSEGV"),
                "{}",
                event.result.stderr
            );
            assert!(
                event.result.stderr.ends_with("err\n"),
                "{}",
                event.result.stderr
            );
        }
    }

    #[test]
    fn other_signals_show_their_line() {
        let fx = fixture();
        let code = "#include <stdlib.h>\nint main(void) {\n    abort();\n}\n";
        fx.run_code("r1", "main", &[("main.c", code)]).unwrap();
        let finished = fx.wait_finished("r1");
        assert_eq!(finished.status(), "runtime_error");
        assert_eq!(
            finished.system_chunks(),
            ["シグナル SIGABRT で終了しました（異常終了。abort の呼び出しなど）"]
        );
    }

    /// 実行ではメモリの検査をしない。解放後の使用と範囲外アクセスがあっても、メモリの誤りとして扱わない。
    #[test]
    fn memory_errors_are_not_checked_in_a_run() {
        let fx = fixture();
        let code = "#include <stdio.h>\n\
             #include <stdlib.h>\n\
             int main(void) {\n\
                 int *a = malloc(4 * sizeof(int));\n\
                 a[0] = 7;\n\
                 int outside = a[5];\n\
                 free(a);\n\
                 int freed = a[0];\n\
                 (void)outside;\n\
                 (void)freed;\n\
                 printf(\"done\\n\");\n\
                 return 0;\n\
             }\n";
        fx.run_code("r1", "main", &[("main.c", code)]).unwrap();
        let finished = fx.wait_finished("r1");
        assert_eq!(
            finished.payload["run"],
            json!({ "mode": "main", "status": "success", "exit_code": 0 })
        );
        assert_eq!(finished.text("stdout"), "done\n");
        assert!(finished.system_chunks().is_empty());
        let stderr = finished.text("stderr");
        assert!(!stderr.contains("AddressSanitizer"), "{stderr}");
        // 検査つきの実行ファイルは作らない。
        assert!(!fx.workspace().join("c-project/main-checked").exists());
        assert!(!fx.workspace().join("c-project/reports").exists());
    }

    const ENDLESS: &str = "#include <stdio.h>\n\
         #include <unistd.h>\n\
         int main(void) {\n\
             printf(\"started\\n\");\n\
             fflush(stdout);\n\
             for (;;) {\n\
                 sleep(1);\n\
             }\n\
         }\n";

    /// 停止ボタンで止めたときは、シグナルの 1 行を送らない。
    #[test]
    fn stopping_does_not_show_the_signal_line() {
        let fx = fixture();
        fx.run_code("r1", "main", &[("main.c", ENDLESS)]).unwrap();
        let outputs = wait_stdout(&fx, "started");
        // 実行・提出は、C の演習と Rust の演習を合わせて、同時に 1 つだけ行う。
        let files = BTreeMap::from([("src/main.rs", "fn main() {}\n")]);
        for (cmd, args) in [
            (
                "run_code",
                json!({ "request_id": "r2", "lesson_id": RUST_LESSON, "mode": "main", "files": files }),
            ),
            (
                "submit_code",
                json!({ "request_id": "r3", "lesson_id": RUST_LESSON, "files": files }),
            ),
            (
                "run_code",
                json!({ "request_id": "r4", "lesson_id": C_LESSON, "mode": "main", "files": { "main.c": MAIN_C } }),
            ),
        ] {
            assert_eq!(error_code(fx.invoke(cmd, args)), "busy", "{cmd}");
        }
        fx.invoke("stop_run", json!({ "request_id": "r1" }))
            .unwrap();
        let finished = fx.wait_finished_after("r1", outputs);
        assert_eq!(
            finished.payload["run"],
            json!({ "mode": "main", "status": "stopped", "exit_code": null })
        );
        assert!(finished.system_chunks().is_empty());
        let event = serde_json::to_value(&finished.run_events()[0]).unwrap();
        assert_eq!(event["result"]["status"], "stopped");
    }

    /// 演習画面を離れたとき（`end_session`）も、停止ボタンと同じく止め、`stopped` を追記し終えてから返る。
    #[test]
    fn end_session_stops_the_run_and_waits_for_the_record() {
        let fx = fixture();
        fx.invoke("open_lesson", json!({ "lesson_id": C_LESSON }))
            .unwrap();
        fx.run_code("r1", "main", &[("main.c", ENDLESS)]).unwrap();
        let outputs = wait_stdout(&fx, "started");
        fx.invoke("end_session", json!({ "lesson_id": C_LESSON }))
            .unwrap();
        // 返ったときには、止めた実行の追記が終わっている。
        let events = run_events(
            AppDataDir::new(fx.data.path())
                .unfinished_session(C_LESSON)
                .unwrap(),
        );
        assert_eq!(events.len(), 1);
        assert_eq!(
            serde_json::to_value(&events[0]).unwrap()["result"]["status"],
            "stopped"
        );
        let finished = fx.wait_finished_after("r1", outputs);
        assert_eq!(finished.status(), "stopped");
        assert!(finished.system_chunks().is_empty());
    }

    /// 実行時間の制限で止めたときは、タイムアウトの旨だけを送り、シグナルの 1 行を送らない。
    #[test]
    fn the_time_limit_does_not_show_the_signal_line() {
        let fx = Fixture::new(
            Some(real_clang()),
            RunLimits {
                run_time: Duration::from_secs(1),
                ..RunLimits::default()
            },
        );
        fx.run_code("r1", "main", &[("main.c", ENDLESS)]).unwrap();
        let finished = fx.wait_finished("r1");
        assert_eq!(
            finished.payload["run"],
            json!({ "mode": "main", "status": "timeout", "exit_code": null })
        );
        assert_eq!(
            finished.system_chunks(),
            ["実行時間の制限（1 秒）を超えたため停止しました。"]
        );
    }

    /// 出力量の上限で止めたときは、上限で停止した旨だけを送り、シグナルの 1 行を送らない。
    #[test]
    fn the_output_limit_does_not_show_the_signal_line() {
        let fx = Fixture::new(
            Some(real_clang()),
            RunLimits {
                output: 1024,
                ..RunLimits::default()
            },
        );
        let code = "#include <stdio.h>\nint main(void) {\n    for (;;) {\n        printf(\"x\");\n    }\n}\n";
        fx.run_code("r1", "main", &[("main.c", code)]).unwrap();
        let finished = fx.wait_finished("r1");
        assert_eq!(
            finished.payload["run"],
            json!({ "mode": "main", "status": "runtime_error", "exit_code": null })
        );
        assert_eq!(
            finished.system_chunks(),
            ["出力量の上限（1 KB）を超えたため停止しました。"]
        );
        assert_eq!(finished.text("stdout").len(), 1024);
    }

    /// 名前付きパイプを書く側として、待たずに開く。読む側（`clang`）がいなければ失敗する（ENXIO）。
    fn open_writer(path: &Path) -> std::io::Result<fs::File> {
        OpenOptions::new()
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open(path)
    }

    /// ビルド中に停止ボタンで止めると、`clang` を止めて `stopped` になり、ビルドの出力を送らない。次の実行は成功する。
    #[test]
    fn stopping_during_the_build_stops_clang() {
        let fx = fixture();
        // `#include` すると読み終わらない名前付きパイプ。
        let temp = tempfile::tempdir().unwrap();
        let fifo = temp.path().join("never-ending.h");
        assert!(
            Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        let code = format!(
            "#include \"{}\"\nint unused_warning(void) {{ int x; return 0; }}\nint main(void) {{ return 0; }}\n",
            fifo.display()
        );
        fx.run_code("r1", "main", &[("main.c", &code)]).unwrap();
        // clang が名前付きパイプを読み始めるまで待つ。書く側は、止めるまで開いたままにする（閉じると EOF になる）。
        let mut writer = None;
        wait_until(
            "clang が名前付きパイプを読み始めない",
            || {
                writer = open_writer(&fifo).ok();
                writer.is_some()
            },
        );
        fx.invoke("stop_run", json!({ "request_id": "r1" }))
            .unwrap();
        let finished = fx.wait_finished("r1");
        drop(writer);

        assert_eq!(
            finished.payload["run"],
            json!({ "mode": "main", "status": "stopped", "exit_code": null })
        );
        assert_eq!(finished.started, 0);
        assert!(finished.outputs.is_empty(), "{:?}", finished.outputs);
        // clang は、もう名前付きパイプを読んでいない。
        wait_until("clang が残っている", || open_writer(&fifo).is_err());
        let event = serde_json::to_value(&finished.run_events()[0]).unwrap();
        assert_eq!(event["result"]["status"], "stopped");

        fx.run_code("r2", "main", &[("main.c", MAIN_C)]).unwrap();
        let finished = fx.wait_finished("r2");
        assert_eq!(finished.status(), "success");
        assert_eq!(finished.text("stdout"), "hello\n");
    }

    /// 学習者のプログラムには、アプリが受け取った環境変数をそのまま渡す（`claude` に渡す 6 つに絞らない）。
    #[test]
    fn environment_variables_are_not_narrowed_for_the_program() {
        let others = cargo_env_lines();
        let fx = fixture();
        let code = "#include <stdio.h>\n\
             extern char **environ;\n\
             int main(void) {\n\
                 for (char **e = environ; *e != NULL; e++) {\n\
                     printf(\"%s\\n\", *e);\n\
                 }\n\
                 return 0;\n\
             }\n";
        fx.run_code("r1", "main", &[("main.c", code)]).unwrap();
        let finished = fx.wait_finished("r1");
        assert_eq!(finished.status(), "success");
        let stdout = finished.text("stdout");
        let lines: Vec<&str> = stdout.lines().collect();
        for line in &others {
            assert!(
                lines.contains(&line.as_str()),
                "{line} がプログラムに渡っていない"
            );
        }
    }
}
