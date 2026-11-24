//! 実行のコマンド `run_code`・`send_stdin`・`close_stdin`・`stop_run` とイベント `run_output`・`run_finished`
//! （spec/architecture/commands/run.md、spec/exercise-runner/run.md、spec/exercise-runner/attempt-log.md）を、
//! モックの実行環境と、実物の `cargo`（このテストを動かしている cargo）で確かめる。

mod common;

use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use common::{LOCAL_URL, app, invoke};
use progrust_lib::attempts::{AttemptLog, Event, RunEvent, RunStatus, SessionKind};
use progrust_lib::commands::run::stop_runs_on_exit;
use progrust_lib::content::ContentDir;
use progrust_lib::environment::{Environment, RustTools};
use progrust_lib::runner::RunnerDir;
use progrust_lib::runner::run::{RUN_FINISHED, RUN_OUTPUT};
use progrust_lib::runner::runs::{RunLimits, Runs};
use progrust_lib::storage::AppDataDir;
use serde_json::{Value, json};
use tauri::test::MockRuntime;
use tauri::{App, Listener, Manager, WebviewWindow};
use tempfile::TempDir;

/// 状態が変わるのを待つ最長の時間。
const WAIT: Duration = Duration::from_secs(60);

/// 許可クレートを使わない演習。
const LESSON: &str = "ch01-01";
/// 許可クレートを使う演習。
const CRATES_LESSON: &str = "ch01-02";
/// 動画のみのレッスン。
const VIDEO_LESSON: &str = "ch01-03";

const HELLO: &str = "fn main() {\n    println!(\"hello\");\n}\n";

/// コンパイルが終わらないコード（定数の評価が終わらない）。
const ENDLESS_COMPILE: &str = "#![allow(long_running_const_eval)]\n\
     const X: u64 = {\n\
         let mut i: u64 = 0;\n\
         loop {\n\
             i = i.wrapping_add(1);\n\
         }\n\
     };\n\
     fn main() {\n\
         println!(\"{X}\");\n\
     }\n";

/// 自分自身を子プロセスとして起動し、`started` を出力してから動き続けるプログラム。
const SPAWNS_A_CHILD: &str = "use std::{env, process::Command, thread, time::Duration};\n\
     fn main() {\n\
         if env::args().nth(1).as_deref() == Some(\"child\") {\n\
             loop {\n\
                 thread::sleep(Duration::from_secs(1));\n\
             }\n\
         }\n\
         let _child = Command::new(env::current_exe().unwrap()).arg(\"child\").spawn().unwrap();\n\
         println!(\"started\");\n\
         loop {\n\
             thread::sleep(Duration::from_secs(1));\n\
         }\n\
     }\n";

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn make_content() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "chapters.json",
        &json!([{
            "id": "ch01",
            "title": "第1章",
            "description": "",
            "lessons": [LESSON, CRATES_LESSON, VIDEO_LESSON],
        }])
        .to_string(),
    );
    for (id, dependencies) in [(LESSON, json!([])), (CRATES_LESSON, json!(["rand"]))] {
        write(
            root,
            &format!("lessons/{id}/lesson.json"),
            &json!({
                "id": id,
                "kind": "lesson",
                "title": id,
                "exercise": {
                    "layout": "single",
                    "judge": { "mode": "stdout", "target": "src/main.rs" },
                    "dependencies": dependencies,
                    "readonly_files": [],
                    "case_labels": {},
                },
            })
            .to_string(),
        );
    }
    write(
        root,
        &format!("lessons/{VIDEO_LESSON}/lesson.json"),
        &json!({ "id": VIDEO_LESSON, "kind": "video_only", "title": "動画" }).to_string(),
    );
    dir
}

/// このテストを動かしている cargo。
fn real_cargo() -> RustTools {
    let cargo = PathBuf::from(env!("CARGO"));
    RustTools {
        rustc: cargo.with_file_name("rustc"),
        cargo,
        rustc_version: "1.100.0".parse().unwrap(),
    }
}

/// 受け取ったイベント。
enum Received {
    Output(Value),
    /// `run_finished` と、それを受け取った時点の試行ログ（レッスン [`LESSON`] の未完了のセッション）。
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
    fn new() -> Self {
        Self::with(RunLimits::default(), Some(real_cargo()))
    }

    fn with(limits: RunLimits, tools: Option<RustTools>) -> Self {
        Self::with_runner(
            limits,
            tools,
            RunnerDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../runner")),
        )
    }

    fn with_runner(limits: RunLimits, tools: Option<RustTools>, runner: RunnerDir) -> Self {
        let content = make_content();
        let data = tempfile::tempdir().unwrap();
        let (app, window) = app();
        app.manage(ContentDir::new(content.path()));
        app.manage(AppDataDir::new(data.path()));
        app.manage(runner);
        app.manage(Environment::detected(tools));
        app.manage(Runs::new(limits));

        let (sender, received) = mpsc::channel();
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
                .unfinished_session(LESSON)
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
        self.run_lesson(request_id, LESSON, mode, files)
    }

    fn run_lesson(
        &self,
        request_id: &str,
        lesson_id: &str,
        mode: &str,
        files: &[(&str, &str)],
    ) -> Result<Value, Value> {
        let files: BTreeMap<&str, &str> = files.iter().copied().collect();
        self.invoke(
            "run_code",
            json!({ "request_id": request_id, "lesson_id": lesson_id, "mode": mode, "files": files }),
        )
    }

    /// `run_finished` を受け取るまで、イベントを受け取る。
    fn wait_finished(&self, request_id: &str) -> Finished {
        let mut outputs = Vec::new();
        loop {
            match self
                .received
                .recv_timeout(WAIT)
                .expect("run_finished が届かない")
            {
                Received::Output(output) => {
                    assert_eq!(output["request_id"], request_id);
                    outputs.push(output);
                }
                Received::Finished(payload, log) => {
                    assert_eq!(payload["request_id"], request_id);
                    return Finished {
                        payload,
                        outputs,
                        log,
                    };
                }
            }
        }
    }

    /// 標準出力に `expected` が現れるまで、`run_output` を受け取る。受け取った出力は `outputs` に足す。
    fn wait_stdout(&self, outputs: &mut Vec<Value>, expected: &str) {
        let deadline = Instant::now() + WAIT;
        while !text_of(outputs, "stdout").contains(expected) {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.received.recv_timeout(left) {
                Ok(Received::Output(output)) => outputs.push(output),
                Ok(Received::Finished(payload, _)) => {
                    panic!("{expected:?} の前に終わった: {payload}")
                }
                Err(_) => panic!("{expected:?} が届かない: {}", text_of(outputs, "stdout")),
            }
        }
    }

    /// `run_finished` が届かないことを確かめる。
    fn assert_no_finished_within(&self, duration: Duration) {
        let deadline = Instant::now() + duration;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.received.recv_timeout(left) {
                Ok(Received::Output(_)) => {}
                Ok(Received::Finished(payload, _)) => panic!("run_finished が届いた: {payload}"),
                Err(_) => return,
            }
        }
    }

    fn runs(&self) -> Runs {
        self.app.state::<Runs>().inner().clone()
    }

    /// 学習者データのディレクトリの名前。作業領域のパス（ビルドの rustc の引数、学習者のプログラムのパス）に入る。
    fn marker(&self) -> String {
        self.data
            .path()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned()
    }

    /// 引数にこのテストの作業領域を含むプロセスの数。
    fn process_count(&self) -> usize {
        let output = Command::new("pgrep")
            .args(["-f", &self.marker()])
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).lines().count()
    }

    fn run_events(&self) -> Vec<RunEvent> {
        run_events(
            AppDataDir::new(self.data.path())
                .unfinished_session(LESSON)
                .unwrap(),
        )
    }
}

struct Finished {
    payload: Value,
    outputs: Vec<Value>,
    log: Option<AttemptLog>,
}

impl Finished {
    fn status(&self) -> &Value {
        &self.payload["run"]["status"]
    }

    fn exit_code(&self) -> &Value {
        &self.payload["run"]["exit_code"]
    }

    fn text(&self, stream: &str) -> String {
        text_of(&self.outputs, stream)
    }

    fn chunks(&self) -> Vec<Value> {
        self.outputs
            .iter()
            .flat_map(|output| output["chunks"].as_array().unwrap().clone())
            .collect()
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

fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "{what}");
        thread::sleep(Duration::from_millis(50));
    }
}

/// 動いている cargo・rustc のプロセスの一覧（失敗したときの手がかり）。
fn cargo_processes() -> String {
    let output = Command::new("ps")
        .args(["-Ao", "pid,etime,command"])
        .output()
        .unwrap();
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|line| line.contains("cargo") || line.contains("rustc"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn error_code(result: Result<Value, Value>) -> Value {
    result.unwrap_err()["code"].clone()
}

// ---- 結果と試行ログ ----

#[test]
fn a_run_is_recorded_in_the_attempt_log_before_run_finished_is_sent() {
    let fx = Fixture::new();
    assert_eq!(
        fx.run_code("r1", "main", &[("src/main.rs", HELLO)]),
        Ok(Value::Null)
    );
    let finished = fx.wait_finished("r1");

    assert_eq!(finished.payload["kind"], "run");
    assert_eq!(finished.payload["error"], Value::Null);
    assert_eq!(
        finished.payload["run"],
        json!({ "mode": "main", "status": "success", "exit_code": 0 })
    );
    assert_eq!(finished.payload["submit"], Value::Null);
    assert_eq!(finished.payload["progress"], Value::Null);
    assert_eq!(finished.text("stdout"), "hello\n");

    // run_finished を受け取った時点で、試行ログに追記されている。
    let events = run_events(finished.log);
    assert_eq!(events.len(), 1);
    let event = &events[0];
    assert_eq!(event.at, finished.payload["finished_at"].as_str().unwrap());
    assert_eq!(
        serde_json::to_value(event).unwrap(),
        json!({
            "at": event.at,
            "mode": "main",
            "files": { "src/main.rs": HELLO },
            "result": {
                "status": "success",
                "error_codes": [],
                "stdout": "hello\n",
                "stderr": "",
                "exit_code": 0,
                "stdin": "",
            },
        })
    );
    // 実行の前に下書きを保存した。
    let draft = AppDataDir::new(fx.data.path())
        .read_draft(LESSON)
        .unwrap()
        .unwrap();
    assert_eq!(draft.files["src/main.rs"], HELLO);
    // 作業領域は {app_data_dir}/workspace/。
    assert!(
        fx.data
            .path()
            .join("workspace/project/src/main.rs")
            .is_file()
    );
    // 終わった後は、次の実行を始められる。
    assert!(fx.runs().current().is_none());
}

#[test]
fn compile_errors_send_the_build_output_and_record_error_codes() {
    let fx = Fixture::new();
    let code = "fn main() {\n    let s = String::from(\"a\");\n    let t = s;\n    println!(\"{s} {t}\");\n}\n";
    fx.run_code("r1", "main", &[("src/main.rs", code)]).unwrap();
    let finished = fx.wait_finished("r1");

    assert_eq!(finished.status(), "compile_error");
    assert_eq!(finished.exit_code(), &Value::Null);
    let stderr = finished.text("stderr");
    assert!(stderr.contains("E0382"), "{stderr}");
    assert!(stderr.contains("could not compile"), "{stderr}");
    assert_eq!(finished.text("stdout"), "");

    let events = run_events(finished.log);
    assert_eq!(events[0].result.error_codes, ["E0382"]);
    assert!(events[0].result.stderr.contains("E0382"));
    assert!(!events[0].result.stderr.contains('\u{1b}'));
}

#[test]
fn a_failure_on_the_app_side_is_sent_as_the_error_of_run_finished() {
    // 同梱の Cargo.lock が無いので、project/ を組み立てられない。
    let empty_runner = tempfile::tempdir().unwrap();
    let fx = Fixture::with_runner(
        RunLimits::default(),
        Some(real_cargo()),
        RunnerDir::new(empty_runner.path()),
    );
    assert_eq!(
        fx.run_code("r1", "main", &[("src/main.rs", HELLO)]),
        Ok(Value::Null)
    );
    let finished = fx.wait_finished("r1");

    assert_eq!(finished.payload["kind"], "run");
    assert_eq!(finished.payload["error"]["code"], "io_error");
    assert_eq!(finished.payload["run"], Value::Null);
    assert_eq!(finished.payload["submit"], Value::Null);
    assert_eq!(finished.payload["progress"], Value::Null);
    assert!(finished.outputs.is_empty(), "{:?}", finished.outputs);
    // 結果の状態が決まっていないので、試行ログには追記しない。
    assert!(run_events(finished.log).is_empty());
    assert!(fx.runs().current().is_none());
}

// ---- 制限 ----

#[test]
fn an_endless_loop_stops_at_the_time_limit_as_timeout() {
    let limits = RunLimits {
        run_time: Duration::from_secs(3),
        ..RunLimits::default()
    };
    let fx = Fixture::with(limits, Some(real_cargo()));
    fx.run_code(
        "r1",
        "main",
        &[("src/main.rs", "fn main() {\n    loop {}\n}\n")],
    )
    .unwrap();
    let started = Instant::now();
    let finished = fx.wait_finished("r1");

    assert!(started.elapsed() >= Duration::from_secs(3));
    assert_eq!(finished.status(), "timeout");
    assert_eq!(finished.exit_code(), &Value::Null);
    assert_eq!(
        finished.chunks().last(),
        Some(&json!({
            "stream": "system",
            "text": "実行時間の制限（3 秒）を超えたため停止しました。",
        }))
    );
    assert_eq!(
        run_events(finished.log)[0].result.status,
        progrust_lib::attempts::RunStatus::Timeout
    );
    wait_until("学習者のプログラムが止まらない", || {
        fx.process_count() == 0
    });
    // アプリの制限値は 60 秒（spec/exercise-runner/limits.md）。
    assert_eq!(RunLimits::default().run_time, Duration::from_secs(60));
}

#[test]
fn endless_output_stops_at_1_mb_as_runtime_error() {
    let fx = Fixture::new();
    fx.run_code(
        "r1",
        "main",
        &[(
            "src/main.rs",
            "fn main() {\n    loop {\n        println!(\"{}\", \"x\".repeat(99));\n    }\n}\n",
        )],
    )
    .unwrap();
    let finished = fx.wait_finished("r1");

    assert_eq!(finished.status(), "runtime_error");
    assert_eq!(finished.exit_code(), &Value::Null);
    // 上限までの出力だけを送る。
    assert_eq!(finished.text("stdout").len(), 1024 * 1024);
    assert_eq!(
        finished.chunks().last(),
        Some(&json!({
            "stream": "system",
            "text": "出力量の上限（1 MB）を超えたため停止しました。",
        }))
    );
    // まとめて送るので、イベントの数は少ない（1 行ごとなら 1 万を超える）。
    assert!(
        finished.outputs.len() < 1000,
        "run_output が多すぎる: {}",
        finished.outputs.len()
    );
    wait_until("学習者のプログラムが止まらない", || {
        fx.process_count() == 0
    });
}

// ---- 停止 ----

#[test]
fn stopping_a_program_also_stops_its_children_in_the_same_group() {
    let fx = Fixture::new();
    fx.run_code("r1", "main", &[("src/main.rs", SPAWNS_A_CHILD)])
        .unwrap();
    let mut outputs = Vec::new();
    fx.wait_stdout(&mut outputs, "started\n");
    wait_until("子プロセスが起動しない", || {
        fx.process_count() >= 2
    });

    assert_eq!(
        fx.invoke("stop_run", json!({ "request_id": "r1" })),
        Ok(Value::Null)
    );
    let finished = fx.wait_finished("r1");
    assert_eq!(finished.status(), "stopped");
    assert_eq!(finished.exit_code(), &Value::Null);
    wait_until("子プロセスが止まらない", || {
        fx.process_count() == 0
    });
    assert_eq!(
        run_events(finished.log)[0].result.status,
        progrust_lib::attempts::RunStatus::Stopped
    );
    // 終わった実行の ID には、何もしない。
    assert_eq!(
        fx.invoke("stop_run", json!({ "request_id": "r1" })),
        Ok(Value::Null)
    );
}

#[test]
fn stopping_during_the_build_gives_stopped_and_the_next_build_succeeds() {
    let fx = Fixture::new();
    let started = Instant::now();
    fx.run_code("r1", "main", &[("src/main.rs", ENDLESS_COMPILE)])
        .unwrap();
    let accepted = started.elapsed();
    let deadline = Instant::now() + WAIT;
    while fx.process_count() == 0 {
        // ビルドが終わってしまったときは、その結果を示して失敗する。
        if let Ok(Received::Finished(payload, _)) = fx.received.try_recv() {
            panic!("ビルドを止める前に終わった（受け付けに {accepted:?}）: {payload}");
        }
        assert!(
            Instant::now() < deadline,
            "ビルドの rustc が起動しない（受け付けに {accepted:?}）。cargo・rustc のプロセス:\n{}",
            cargo_processes()
        );
        thread::sleep(Duration::from_millis(50));
    }

    fx.invoke("stop_run", json!({ "request_id": "r1" }))
        .unwrap();
    let finished = fx.wait_finished("r1");
    assert_eq!(finished.status(), "stopped");
    assert_eq!(finished.exit_code(), &Value::Null);
    // ビルドの出力は送らずに捨てる。
    assert!(finished.outputs.is_empty(), "{:?}", finished.outputs);
    wait_until("ビルドの rustc が止まらない", || {
        fx.process_count() == 0
    });

    fx.run_code("r2", "main", &[("src/main.rs", HELLO)])
        .unwrap();
    let next = fx.wait_finished("r2");
    assert_eq!(next.status(), "success", "{:?}", next.outputs);
    assert_eq!(next.text("stdout"), "hello\n");
    let statuses: Vec<_> = run_events(next.log)
        .iter()
        .map(|event| event.result.status)
        .collect();
    assert_eq!(
        statuses,
        [
            progrust_lib::attempts::RunStatus::Stopped,
            progrust_lib::attempts::RunStatus::Success
        ]
    );
}

#[test]
fn a_running_request_makes_others_busy_and_its_id_invalid() {
    let fx = Fixture::new();
    fx.run_code("r1", "main", &[("src/main.rs", SPAWNS_A_CHILD)])
        .unwrap();
    let mut outputs = Vec::new();
    fx.wait_stdout(&mut outputs, "started\n");

    assert_eq!(
        error_code(fx.run_code("r1", "main", &[("src/main.rs", HELLO)])),
        "invalid_argument"
    );
    assert_eq!(
        error_code(fx.run_code("r2", "test", &[("src/main.rs", HELLO)])),
        "busy"
    );
    // busy で断った実行は、下書きを書き換えない。
    let draft = AppDataDir::new(fx.data.path())
        .read_draft(LESSON)
        .unwrap()
        .unwrap();
    assert_eq!(draft.files["src/main.rs"], SPAWNS_A_CHILD);

    fx.invoke("stop_run", json!({ "request_id": "r1" }))
        .unwrap();
    assert_eq!(fx.wait_finished("r1").status(), "stopped");
}

#[test]
fn quitting_the_app_stops_the_run_without_run_finished_or_a_record() {
    let fx = Fixture::new();
    fx.run_code("r1", "main", &[("src/main.rs", SPAWNS_A_CHILD)])
        .unwrap();
    let mut outputs = Vec::new();
    fx.wait_stdout(&mut outputs, "started\n");

    stop_runs_on_exit(fx.app.handle());
    wait_until("学習者のプログラムが止まらない", || {
        fx.process_count() == 0
    });
    fx.assert_no_finished_within(Duration::from_secs(2));
    assert!(fx.runs().current().is_none());
    assert!(fx.run_events().is_empty());
}

// ---- 画面を離れたとき（end_session） ----

impl Fixture {
    /// レッスン [`LESSON`] の試行セッションを始め、正解済みにして、その ID を返す（解説を閉じた後の演習画面）。
    fn start_passed_session(&self) -> String {
        let data_dir = AppDataDir::new(self.data.path());
        let session_id = data_dir
            .start_or_continue_session(LESSON, SessionKind::Lesson)
            .unwrap()
            .session_id;
        data_dir
            .mark_session_passed(&session_id, "2026-11-24T10:00:00+09:00")
            .unwrap()
            .unwrap();
        session_id
    }

    fn attempt_log(&self, session_id: &str) -> AttemptLog {
        AppDataDir::new(self.data.path())
            .read_attempt_log(session_id)
            .unwrap()
            .unwrap()
    }

    fn end_session(&self, lesson_id: &str) -> Result<Value, Value> {
        self.invoke("end_session", json!({ "lesson_id": lesson_id }))
    }

    /// 動き続けるプログラムを実行し、起動するまで待つ。
    fn start_endless_run(&self, request_id: &str) {
        self.run_code(request_id, "main", &[("src/main.rs", SPAWNS_A_CHILD)])
            .unwrap();
        let mut outputs = Vec::new();
        self.wait_stdout(&mut outputs, "started\n");
    }
}

/// `end_session` が返った時点で、止めた実行が `stopped` として終える前のセッションに追記され、そのセッションが終わっている。
fn assert_stopped_run_recorded_before_ending(fx: &Fixture, session_id: &str) {
    let log = fx.attempt_log(session_id);
    assert!(log.ended_at.is_some(), "セッションが終わっていない");
    let events = run_events(Some(log));
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].result.status, RunStatus::Stopped);
    // 終えた後のセッションに追記したり、新しいセッションを始めたりしていない。
    assert!(
        AppDataDir::new(fx.data.path())
            .unfinished_session(LESSON)
            .unwrap()
            .is_none()
    );
}

#[test]
fn end_session_stops_a_running_program_and_records_it_before_ending_the_session() {
    let fx = Fixture::new();
    let session_id = fx.start_passed_session();
    fx.start_endless_run("r1");

    assert_eq!(fx.end_session(LESSON), Ok(Value::Null));
    // run_finished を待たずに、end_session が返った時点で確かめる。
    assert_stopped_run_recorded_before_ending(&fx, &session_id);
    assert!(fx.runs().current().is_none());

    assert_eq!(fx.wait_finished("r1").status(), "stopped");
    wait_until("学習者のプログラムが止まらない", || {
        fx.process_count() == 0
    });
}

#[test]
fn end_session_after_stop_run_also_records_the_stopped_run_before_ending() {
    let fx = Fixture::new();
    let session_id = fx.start_passed_session();
    fx.start_endless_run("r1");

    // フロントエンドが stop_run を先に呼び、止めた結果を待たずに end_session を呼ぶ。
    assert_eq!(
        fx.invoke("stop_run", json!({ "request_id": "r1" })),
        Ok(Value::Null)
    );
    assert_eq!(fx.end_session(LESSON), Ok(Value::Null));
    assert_stopped_run_recorded_before_ending(&fx, &session_id);

    assert_eq!(fx.wait_finished("r1").status(), "stopped");
    wait_until("学習者のプログラムが止まらない", || {
        fx.process_count() == 0
    });
}

#[test]
fn end_session_stops_a_build_in_progress_and_records_it_before_ending() {
    let fx = Fixture::new();
    let session_id = fx.start_passed_session();
    fx.run_code("r1", "main", &[("src/main.rs", ENDLESS_COMPILE)])
        .unwrap();
    wait_until("ビルドの rustc が起動しない", || {
        fx.process_count() > 0
    });

    assert_eq!(fx.end_session(LESSON), Ok(Value::Null));
    assert_stopped_run_recorded_before_ending(&fx, &session_id);
    assert_eq!(fx.wait_finished("r1").status(), "stopped");
    wait_until("ビルドの rustc が止まらない", || {
        fx.process_count() == 0
    });
}

#[test]
fn end_session_without_a_pass_stops_and_records_the_run_but_keeps_the_session() {
    let fx = Fixture::new();
    fx.start_endless_run("r1");

    assert_eq!(fx.end_session(LESSON), Ok(Value::Null));
    // 正解していないので、セッションは終えない。止めた実行は、そのセッションに追記されている。
    let log = AppDataDir::new(fx.data.path())
        .unfinished_session(LESSON)
        .unwrap()
        .expect("未完了のセッションが無い");
    let events = run_events(Some(log));
    assert_eq!(events.len(), 1, "{events:?}");
    assert_eq!(events[0].result.status, RunStatus::Stopped);
    assert_eq!(fx.wait_finished("r1").status(), "stopped");
}

#[test]
fn end_session_of_another_lesson_does_not_stop_the_run() {
    let fx = Fixture::new();
    fx.start_endless_run("r1");

    assert_eq!(fx.end_session(CRATES_LESSON), Ok(Value::Null));
    let run = fx.runs().find("r1").expect("実行が止まった");
    assert_eq!(run.stop_cause(), None);
    fx.assert_no_finished_within(Duration::from_millis(500));

    fx.invoke("stop_run", json!({ "request_id": "r1" }))
        .unwrap();
    assert_eq!(fx.wait_finished("r1").status(), "stopped");
}

// ---- 標準入力 ----

#[test]
fn lines_sent_to_stdin_are_read_by_read_line() {
    let fx = Fixture::new();
    let program = "use std::io::{self, Write};\n\
         fn main() {\n\
             print!(\"名前: \");\n\
             io::stdout().flush().unwrap();\n\
             loop {\n\
                 let mut line = String::new();\n\
                 if io::stdin().read_line(&mut line).unwrap() == 0 {\n\
                     break;\n\
                 }\n\
                 println!(\"こんにちは、{}さん\", line.trim_end());\n\
             }\n\
             println!(\"eof\");\n\
         }\n";
    fx.run_code("r1", "main", &[("src/main.rs", program)])
        .unwrap();
    let mut outputs = Vec::new();
    fx.wait_stdout(&mut outputs, "名前: ");

    let send = |text: &str| fx.invoke("send_stdin", json!({ "request_id": "r1", "text": text }));
    assert_eq!(send("ferris\n"), Ok(Value::Null));
    fx.wait_stdout(&mut outputs, "こんにちは、ferrisさん\n");
    assert_eq!(send("日本語\n"), Ok(Value::Null));
    fx.wait_stdout(&mut outputs, "こんにちは、日本語さん\n");
    assert_eq!(
        fx.invoke("close_stdin", json!({ "request_id": "r1" })),
        Ok(Value::Null)
    );
    // 閉じた後は書けない。
    assert_eq!(error_code(send("後\n")), "invalid_state");

    let finished = fx.wait_finished("r1");
    assert_eq!(finished.status(), "success");
    outputs.extend(finished.outputs);
    assert_eq!(
        text_of(&outputs, "stdout"),
        "名前: こんにちは、ferrisさん\nこんにちは、日本語さん\neof\n"
    );
    assert_eq!(run_events(finished.log)[0].result.stdin, "ferris\n日本語\n");

    // 終わった後と、無い ID は invalid_state。
    assert_eq!(error_code(send("x\n")), "invalid_state");
    assert_eq!(
        error_code(fx.invoke("close_stdin", json!({ "request_id": "r1" }))),
        "invalid_state"
    );
    assert_eq!(
        error_code(fx.invoke("send_stdin", json!({ "request_id": "none", "text": "x\n" }))),
        "invalid_state"
    );
}

// ---- テスト実行 ----

#[test]
fn test_mode_runs_unit_integration_and_doc_tests_without_stopping_at_a_failure() {
    let fx = Fixture::new();
    let lib = "/// 足し算。\n\
         ///\n\
         /// ```\n\
         /// assert_eq!(progrust_exercise::add(1, 2), 3);\n\
         /// ```\n\
         pub fn add(a: i32, b: i32) -> i32 {\n\
             a + b\n\
         }\n\
         \n\
         #[cfg(test)]\n\
         mod tests {\n\
             use super::*;\n\
             #[test]\n\
             fn fails() {\n\
                 assert_eq!(add(1, 1), 3);\n\
             }\n\
             #[test]\n\
             fn passes() {\n\
                 assert_eq!(add(1, 1), 2);\n\
             }\n\
         }\n";
    let integration =
        "#[test]\nfn from_outside() {\n    assert_eq!(progrust_exercise::add(2, 2), 4);\n}\n";
    fx.run_code(
        "r1",
        "test",
        &[
            ("src/main.rs", "fn main() {}\n"),
            ("src/lib.rs", lib),
            ("tests/it.rs", integration),
        ],
    )
    .unwrap();
    let finished = fx.wait_finished("r1");

    assert_eq!(
        finished.payload["run"],
        json!({ "mode": "test", "status": "runtime_error", "exit_code": 101 })
    );
    let console = strip_colors(&(finished.text("stdout") + &finished.text("stderr")));
    for line in [
        "test tests::fails ... FAILED",
        "test tests::passes ... ok",
        "test from_outside ... ok",
        "Doc-tests progrust_exercise",
        "test src/lib.rs - add (line 3) ... ok",
        "Running tests/it.rs",
    ] {
        assert!(console.contains(line), "{line:?} が無い:\n{console}");
    }
    // cargo 自身の行（`Running ...` など）は、CARGO_TERM_COLOR=always で色の指定が付き、そのまま送る。
    assert!(finished.text("stderr").contains('\u{1b}'));

    let events = run_events(finished.log);
    assert_eq!(events[0].mode, progrust_lib::attempts::RunMode::Test);
    assert_eq!(events[0].result.stdin, "");
    assert_eq!(events[0].result.exit_code, Some(101));
}

// ---- 受け付ける前に断るもの ----

#[test]
fn paths_with_parent_segments_are_rejected_without_writing_outside_project() {
    let fx = Fixture::new();
    for path in ["../outside.rs", "src/../../outside.rs", "/tmp/outside.rs"] {
        assert_eq!(
            error_code(fx.run_code("r1", "main", &[("src/main.rs", HELLO), (path, "// 外")])),
            "invalid_argument",
            "{path}"
        );
    }
    // 何も書かず（下書きも project/ も作らず）、次の実行を受け付けられる。
    let mut written = Vec::new();
    for entry in fs::read_dir(fx.data.path()).unwrap() {
        written.push(entry.unwrap().path());
    }
    assert!(written.is_empty(), "{written:?}");
    assert!(!fx.data.path().parent().unwrap().join("outside.rs").exists());
    assert!(fx.runs().current().is_none());
    fx.assert_no_finished_within(Duration::from_millis(200));
}

#[test]
fn without_cargo_the_draft_is_saved_and_cargo_unavailable_is_returned() {
    let fx = Fixture::with(RunLimits::default(), None);
    assert_eq!(
        error_code(fx.run_code("r1", "main", &[("src/main.rs", HELLO)])),
        "cargo_unavailable"
    );
    let draft = AppDataDir::new(fx.data.path())
        .read_draft(LESSON)
        .unwrap()
        .unwrap();
    assert_eq!(draft.files["src/main.rs"], HELLO);
    assert!(fx.runs().current().is_none());
}

#[test]
fn exercises_with_crates_need_the_preparation_to_be_ready() {
    let fx = Fixture::new();
    assert_eq!(
        error_code(fx.run_lesson("r1", CRATES_LESSON, "main", &[("src/main.rs", HELLO)])),
        "crates_not_ready"
    );
    assert!(fx.runs().current().is_none());
    assert!(!fx.data.path().join("workspace").exists());
}

#[test]
fn lessons_without_an_exercise_and_unknown_lessons_are_rejected() {
    let fx = Fixture::new();
    assert_eq!(
        error_code(fx.run_lesson("r1", VIDEO_LESSON, "main", &[("src/main.rs", HELLO)])),
        "invalid_argument"
    );
    assert_eq!(
        error_code(fx.run_lesson("r1", "gen-x", "main", &[("src/main.rs", HELLO)])),
        "not_found"
    );
    assert!(fx.runs().current().is_none());
}
