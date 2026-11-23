//! 提出のコマンド `submit_code` とイベント `submit_progress`・`run_output`・`run_finished`、提出を止める `stop_run`
//! （spec/architecture/commands/run.md、spec/exercise-runner/submit.md、spec/exercise-runner/attempt-log.md、
//! spec/lesson-flow/completion.md）を、モックの実行環境と、実物の `cargo`（このテストを動かしている cargo）で確かめる。

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
use progrust_lib::attempts::{AttemptLog, Event, SubmitEvent, SubmitStatus};
use progrust_lib::commands::run::stop_runs_on_exit;
use progrust_lib::content::ContentDir;
use progrust_lib::environment::{Environment, RustTools};
use progrust_lib::progress::LessonStatus;
use progrust_lib::runner::RunnerDir;
use progrust_lib::runner::run::{RUN_FINISHED, RUN_OUTPUT};
use progrust_lib::runner::runs::{RunLimits, Runs};
use progrust_lib::runner::submit::SUBMIT_PROGRESS;
use progrust_lib::storage::AppDataDir;
use serde_json::{Value, json};
use tauri::test::MockRuntime;
use tauri::{App, Listener, Manager, WebviewWindow};
use tempfile::TempDir;

/// 状態が変わるのを待つ最長の時間。
const WAIT: Duration = Duration::from_secs(60);

/// `stdout` 方式の演習。2 つの数の和を出力する。
const STDOUT_LESSON: &str = "ch01-01";
/// `test` 方式の演習。`add` を実装する。学習者が書いたテストは判定に含めない。
const TEST_LESSON: &str = "ch01-02";
/// `test` 方式の演習。`min_learner_tests` が 1。
const LEARNER_TESTS_LESSON: &str = "ch01-03";
/// `test` 方式の演習。隠しテストの実行に時間がかかる（止める操作を確かめる）。
const SLOW_TEST_LESSON: &str = "ch01-04";

/// 2 つの数の和を出力する、正しいコード。
const SUM: &str = "use std::io::Read;\n\
     fn main() {\n\
         let mut input = String::new();\n\
         std::io::stdin().read_to_string(&mut input).unwrap();\n\
         let sum: i64 = input.split_whitespace().map(|n| n.parse::<i64>().unwrap()).sum();\n\
         println!(\"{sum}\");\n\
     }\n";

/// 3 秒待ってから、正しい答えを出力するコード。
const SLOW_SUM: &str = "use std::io::Read;\n\
     fn main() {\n\
         std::thread::sleep(std::time::Duration::from_secs(3));\n\
         let mut input = String::new();\n\
         std::io::stdin().read_to_string(&mut input).unwrap();\n\
         let sum: i64 = input.split_whitespace().map(|n| n.parse::<i64>().unwrap()).sum();\n\
         println!(\"{sum}\");\n\
     }\n";

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

/// `test` 方式の隠しテスト。
const ADD_JUDGE_TESTS: &str = "use super::*;\n\
     \n\
     #[test]\n\
     fn adds_positive() {\n\
         assert_eq!(add(1, 2), 3);\n\
     }\n\
     \n\
     #[test]\n\
     fn adds_negative() {\n\
         assert_eq!(add(-1, -2), -3);\n\
     }\n";

/// 時間のかかる隠しテスト。
const SLOW_JUDGE_TESTS: &str = "use super::*;\n\
     \n\
     #[test]\n\
     fn slow_a() {\n\
         std::thread::sleep(std::time::Duration::from_secs(4));\n\
         assert_eq!(add(1, 2), 3);\n\
     }\n\
     \n\
     #[test]\n\
     fn slow_b() {\n\
         std::thread::sleep(std::time::Duration::from_secs(4));\n\
         assert_eq!(add(2, 2), 4);\n\
     }\n";

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn lesson_json(id: &str, judge: Value, case_labels: Value) -> String {
    json!({
        "id": id,
        "kind": "lesson",
        "title": id,
        "exercise": {
            "layout": "single",
            "judge": judge,
            "dependencies": [],
            "readonly_files": [],
            "case_labels": case_labels,
        },
    })
    .to_string()
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
            "lessons": [STDOUT_LESSON, TEST_LESSON, LEARNER_TESTS_LESSON, SLOW_TEST_LESSON],
        }])
        .to_string(),
    );
    write(
        root,
        &format!("lessons/{STDOUT_LESSON}/lesson.json"),
        &lesson_json(
            STDOUT_LESSON,
            json!({ "mode": "stdout", "target": "src/main.rs" }),
            json!({}),
        ),
    );
    write(
        root,
        &format!("lessons/{STDOUT_LESSON}/exercise/cases.json"),
        &json!([
            { "name": "正の数", "stdin": "1 2\n", "expected_stdout": "3\n" },
            { "name": "負の数", "stdin": "-1 -2\n", "expected_stdout": "-3\n" },
        ])
        .to_string(),
    );
    for (id, judge, judge_tests) in [
        (
            TEST_LESSON,
            json!({ "mode": "test", "target": "src/main.rs" }),
            ADD_JUDGE_TESTS,
        ),
        (
            LEARNER_TESTS_LESSON,
            json!({ "mode": "test", "target": "src/main.rs", "min_learner_tests": 1 }),
            ADD_JUDGE_TESTS,
        ),
        (
            SLOW_TEST_LESSON,
            json!({ "mode": "test", "target": "src/main.rs" }),
            SLOW_JUDGE_TESTS,
        ),
    ] {
        write(
            root,
            &format!("lessons/{id}/lesson.json"),
            &lesson_json(
                id,
                judge,
                json!({ "adds_positive": "正の数の足し算", "adds_negative": "負の数の足し算" }),
            ),
        );
        write(
            root,
            &format!("lessons/{id}/exercise/judge_tests.rs"),
            judge_tests,
        );
    }
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
    Progress(Value),
    /// `run_finished` と、それを受け取った時点の、そのレッスンの未完了のセッション。
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
        Self::with(Some(real_cargo()))
    }

    fn with(tools: Option<RustTools>) -> Self {
        let content = make_content();
        let data = tempfile::tempdir().unwrap();
        let (app, window) = app();
        app.manage(ContentDir::new(content.path()));
        app.manage(AppDataDir::new(data.path()));
        app.manage(RunnerDir::new(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../runner"
        )));
        app.manage(Environment::detected(tools));
        app.manage(Runs::new(RunLimits::default()));

        let (sender, received) = mpsc::channel();
        {
            let sender = sender.clone();
            app.listen(RUN_OUTPUT, move |event| {
                let payload = serde_json::from_str(event.payload()).unwrap();
                let _ = sender.send(Received::Output(payload));
            });
        }
        {
            let sender = sender.clone();
            app.listen(SUBMIT_PROGRESS, move |event| {
                let payload = serde_json::from_str(event.payload()).unwrap();
                let _ = sender.send(Received::Progress(payload));
            });
        }
        let data_path = data.path().to_path_buf();
        let runs = app.state::<Runs>().inner().clone();
        app.listen(RUN_FINISHED, move |event| {
            let payload: Value = serde_json::from_str(event.payload()).unwrap();
            // run_finished を送る前に、要求を外している。
            assert!(runs.current().is_none());
            let lesson_id = lesson_of(payload["request_id"].as_str().unwrap());
            let log = AppDataDir::new(&data_path)
                .unfinished_session(&lesson_id)
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

    /// `lesson_id` の演習に `files` を提出する。`request_id` は `{lesson_id}/{name}` にする。
    fn submit(&self, lesson_id: &str, name: &str, files: &[(&str, &str)]) -> Result<Value, Value> {
        let files: BTreeMap<&str, &str> = files.iter().copied().collect();
        self.invoke(
            "submit_code",
            json!({ "request_id": request_id(lesson_id, name), "lesson_id": lesson_id, "files": files }),
        )
    }

    /// `run_finished` を受け取るまで、イベントを受け取る。
    fn wait_finished(&self, request_id: &str) -> Finished {
        let mut finished = Finished::default();
        self.receive_until(request_id, &mut finished, |_| false);
        finished
    }

    /// `stop_when` が真になるイベントか、`run_finished` を受け取るまで、イベントを受け取る。
    /// `run_finished` を受け取ったら真を返す。
    fn receive_until(
        &self,
        request_id: &str,
        finished: &mut Finished,
        mut stop_when: impl FnMut(&Received) -> bool,
    ) -> bool {
        loop {
            let received = self
                .received
                .recv_timeout(WAIT)
                .expect("run_finished が届かない");
            let stop = stop_when(&received);
            match received {
                Received::Output(output) => {
                    assert_eq!(output["request_id"], request_id);
                    finished.outputs.push(output);
                }
                Received::Progress(progress) => {
                    assert_eq!(progress["request_id"], request_id);
                    finished.progress.push(progress);
                }
                Received::Finished(payload, log) => {
                    assert_eq!(payload["request_id"], request_id);
                    finished.payload = payload;
                    finished.log = log;
                    return true;
                }
            }
            if stop {
                return false;
            }
        }
    }

    /// 判定の段階（`phase` が `judge`）の `submit_progress` を受け取るまで待つ。
    fn wait_judging(&self, request_id: &str) -> Finished {
        let mut finished = Finished::default();
        let ended = self.receive_until(request_id, &mut finished, |received| {
            matches!(received, Received::Progress(progress) if progress["phase"] == "judge")
        });
        assert!(!ended, "判定の段階の前に終わった: {}", finished.payload);
        finished
    }

    /// `run_finished` が届かないことを確かめる。
    fn assert_no_finished_within(&self, duration: Duration) {
        let deadline = Instant::now() + duration;
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.received.recv_timeout(left) {
                Ok(Received::Finished(payload, _)) => panic!("run_finished が届いた: {payload}"),
                Ok(_) => {}
                Err(_) => return,
            }
        }
    }

    fn runs(&self) -> Runs {
        self.app.state::<Runs>().inner().clone()
    }

    fn data_dir(&self) -> AppDataDir {
        AppDataDir::new(self.data.path())
    }

    /// 引数にこのテストの作業領域を含むプロセスの数（ビルドの rustc、判定で動かすプログラム）。
    fn process_count(&self) -> usize {
        let marker = self.data.path().file_name().unwrap().to_string_lossy();
        let output = Command::new("pgrep")
            .args(["-f", &marker])
            .output()
            .unwrap();
        String::from_utf8_lossy(&output.stdout).lines().count()
    }

    /// レッスン `lesson_id` の `submit` イベント（未完了のセッション）。
    fn submit_events(&self, lesson_id: &str) -> Vec<SubmitEvent> {
        submit_events(self.data_dir().unfinished_session(lesson_id).unwrap())
    }

    fn lesson_completed(&self, lesson_id: &str) -> bool {
        self.data_dir()
            .read_progress()
            .unwrap()
            .lessons
            .get(lesson_id)
            .is_some_and(|lesson| lesson.status == LessonStatus::Completed)
    }
}

fn request_id(lesson_id: &str, name: &str) -> String {
    format!("{lesson_id}/{name}")
}

fn lesson_of(request_id: &str) -> String {
    request_id.split('/').next().unwrap().to_string()
}

#[derive(Default)]
struct Finished {
    payload: Value,
    outputs: Vec<Value>,
    progress: Vec<Value>,
    log: Option<AttemptLog>,
}

impl Finished {
    fn submit(&self) -> &Value {
        &self.payload["submit"]
    }

    fn text(&self, stream: &str) -> String {
        self.outputs
            .iter()
            .flat_map(|output| output["chunks"].as_array().unwrap())
            .filter(|chunk| chunk["stream"] == stream)
            .map(|chunk| chunk["text"].as_str().unwrap())
            .collect()
    }

    /// 受け取った `submit_progress` の (`phase`, `completed`, `total`)。
    fn progress_steps(&self) -> Vec<(String, u64, Option<u64>)> {
        self.progress
            .iter()
            .map(|progress| {
                (
                    progress["phase"].as_str().unwrap().to_string(),
                    progress["completed"].as_u64().unwrap(),
                    progress["total"].as_u64(),
                )
            })
            .collect()
    }
}

fn submit_events(log: Option<AttemptLog>) -> Vec<SubmitEvent> {
    log.map(|log| log.events)
        .unwrap_or_default()
        .into_iter()
        .filter_map(|event| match event {
            Event::Submit(submit) => Some(submit),
            _ => None,
        })
        .collect()
}

fn case_names(submit: &Value) -> Vec<&str> {
    submit["cases"]
        .as_array()
        .unwrap()
        .iter()
        .map(|case| case["name"].as_str().unwrap())
        .collect()
}

fn wait_until(what: &str, mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + WAIT;
    while !condition() {
        assert!(Instant::now() < deadline, "{what}");
        thread::sleep(Duration::from_millis(50));
    }
}

fn error_code(result: Result<Value, Value>) -> Value {
    result.unwrap_err()["code"].clone()
}

// ---- 判定結果と記録 ----

#[test]
fn a_correct_stdout_submit_passes_records_the_pass_and_completes_the_lesson() {
    let fx = Fixture::new();
    assert_eq!(
        fx.submit(STDOUT_LESSON, "s1", &[("src/main.rs", SUM)]),
        Ok(Value::Null)
    );
    let finished = fx.wait_finished(&request_id(STDOUT_LESSON, "s1"));

    assert_eq!(finished.payload["kind"], "submit");
    assert_eq!(finished.payload["error"], Value::Null);
    assert_eq!(finished.payload["run"], Value::Null);
    assert_eq!(
        finished.submit(),
        &json!({
            "status": "passed",
            "passed": 2,
            "total": 2,
            "cases": [
                { "name": "正の数", "status": "pass", "detail": null },
                { "name": "負の数", "status": "pass", "detail": null },
            ],
        })
    );
    // 判定の進み具合は、段階が変わったときと、テストケースが 1 件終わるたびに届く。
    assert_eq!(
        finished.progress_steps(),
        [
            ("build".to_string(), 0, None),
            ("judge".to_string(), 0, Some(2)),
            ("judge".to_string(), 1, Some(2)),
            ("judge".to_string(), 2, Some(2)),
        ]
    );
    // 判定で動かしたプログラムの出力は送らない。
    assert_eq!(finished.text("stdout"), "");

    // run_finished の progress は、完了にした後の進捗。
    let finished_at = finished.payload["finished_at"].as_str().unwrap();
    let progress = &finished.payload["progress"];
    assert_eq!(progress["status"], "completed");
    assert_eq!(progress["completion_count"], 1);
    assert_eq!(progress["first_completed_at"], finished_at);
    assert!(fx.lesson_completed(STDOUT_LESSON));

    // run_finished を受け取った時点で、試行ログに passed_at と submit イベントが入っている。
    let log = finished.log.clone().expect("試行ログが無い");
    assert_eq!(log.passed_at.as_deref(), Some(finished_at));
    assert_eq!(
        progress["last_passed_session_id"],
        json!(log.session_id.clone())
    );
    let events = submit_events(Some(log));
    assert_eq!(events.len(), 1);
    assert_eq!(
        serde_json::to_value(&events[0]).unwrap(),
        json!({
            "at": finished_at,
            "files": { "src/main.rs": SUM },
            "result": {
                "status": "passed",
                "error_codes": [],
                "passed": 2,
                "total": 2,
                "failed_cases": [],
                "stderr": "",
            },
        })
    );
    // 提出の前に下書きを保存した。
    let draft = fx.data_dir().read_draft(STDOUT_LESSON).unwrap().unwrap();
    assert_eq!(draft.files["src/main.rs"], SUM);
}

#[test]
fn a_test_submit_failing_one_hidden_test_is_wrong_with_its_display_name() {
    let fx = Fixture::new();
    let wrong = "fn add(a: i32, b: i32) -> i32 {\n    (a + b).abs()\n}\n\nfn main() {}\n";
    fx.submit(TEST_LESSON, "s1", &[("src/main.rs", wrong)])
        .unwrap();
    let finished = fx.wait_finished(&request_id(TEST_LESSON, "s1"));

    let submit = finished.submit();
    assert_eq!(submit["status"], "wrong");
    assert_eq!(submit["passed"], 1);
    assert_eq!(submit["total"], 2);
    // テストケースは、隠しテストの名前を得た `--list` の出力の順（名前の順）に並ぶ。
    assert_eq!(case_names(submit), ["負の数の足し算", "正の数の足し算"]);
    assert_eq!(submit["cases"][0]["status"], "fail");
    assert_eq!(submit["cases"][1]["status"], "pass");
    let detail = submit["cases"][0]["detail"].as_str().unwrap();
    assert!(detail.contains("left: 3"), "{detail}");
    // test 方式では、隠しテストの名前を得るまで、全テストケースの数は決まらない。
    let steps = finished.progress_steps();
    assert_eq!(
        steps[..2],
        [
            ("build".to_string(), 0, None),
            ("judge".to_string(), 0, None)
        ]
    );
    assert_eq!(steps.last(), Some(&("judge".to_string(), 2, Some(2))));

    // 不正解は、レッスンを完了にしない。
    assert_eq!(finished.payload["progress"], Value::Null);
    assert!(!fx.lesson_completed(TEST_LESSON));
    let log = finished.log.clone().unwrap();
    assert_eq!(log.passed_at, None);
    let events = submit_events(Some(log));
    assert_eq!(events[0].result.status, SubmitStatus::Wrong);
    assert_eq!(events[0].result.failed_cases, ["負の数の足し算"]);
    // 記録するのは、隠しテストを組み込む前のエディタ上のファイル。
    assert_eq!(events[0].files["src/main.rs"], wrong);
}

#[test]
fn learner_tests_are_judged_as_the_last_case_when_min_learner_tests_is_set() {
    let fx = Fixture::new();
    let code = "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\nfn main() {}\n\n\
                #[cfg(test)]\nmod tests {\n    use super::*;\n\n    \
                #[test]\n    fn adds_zero() {\n        assert_eq!(add(0, 0), 0);\n    }\n}\n";
    fx.submit(LEARNER_TESTS_LESSON, "s1", &[("src/main.rs", code)])
        .unwrap();
    let finished = fx.wait_finished(&request_id(LEARNER_TESTS_LESSON, "s1"));

    let submit = finished.submit();
    assert_eq!(submit["status"], "passed", "{submit}");
    assert_eq!(
        case_names(submit),
        [
            "負の数の足し算",
            "正の数の足し算",
            "自分で書いたテスト（1 件以上）"
        ]
    );
    assert_eq!(
        finished.progress_steps().last(),
        Some(&("judge".to_string(), 3, Some(3)))
    );
    assert!(fx.lesson_completed(LEARNER_TESTS_LESSON));
}

#[test]
fn compile_errors_send_the_build_output_and_are_recorded() {
    let fx = Fixture::new();
    let code = "fn main() {\n    let s = String::from(\"a\");\n    let t = s;\n    println!(\"{s} {t}\");\n}\n";
    fx.submit(STDOUT_LESSON, "s1", &[("src/main.rs", code)])
        .unwrap();
    let finished = fx.wait_finished(&request_id(STDOUT_LESSON, "s1"));

    assert_eq!(
        finished.submit(),
        &json!({ "status": "compile_error", "passed": 0, "total": 0, "cases": [] })
    );
    let stderr = finished.text("stderr");
    assert!(stderr.contains("E0382"), "{stderr}");
    assert_eq!(finished.progress_steps(), [("build".to_string(), 0, None)]);
    assert_eq!(finished.payload["progress"], Value::Null);

    let events = submit_events(finished.log);
    assert_eq!(events[0].result.status, SubmitStatus::CompileError);
    assert_eq!(events[0].result.error_codes, ["E0382"]);
    assert_eq!(events[0].result.failed_cases, Vec::<String>::new());
    assert!(events[0].result.stderr.contains("E0382"));
    assert!(!events[0].result.stderr.contains('\u{1b}'));
}

#[test]
fn errors_on_the_hidden_test_side_also_send_the_hint_as_system() {
    let fx = Fixture::new();
    let code = "fn plus(a: i32, b: i32) -> i32 {\n    a + b\n}\n\nfn main() {}\n";
    fx.submit(TEST_LESSON, "s1", &[("src/main.rs", code)])
        .unwrap();
    let finished = fx.wait_finished(&request_id(TEST_LESSON, "s1"));

    assert_eq!(finished.submit()["status"], "compile_error");
    assert!(finished.text("stderr").contains("E0425"));
    assert_eq!(
        finished.text("system"),
        "問題文で指定された関数名・型・シグネチャを確認してください。"
    );
}

// ---- 止める ----

#[test]
fn stopping_while_judging_gives_stopped_without_a_record_or_completion() {
    let fx = Fixture::new();
    let id = request_id(STDOUT_LESSON, "s1");
    fx.submit(STDOUT_LESSON, "s1", &[("src/main.rs", SLOW_SUM)])
        .unwrap();
    fx.wait_judging(&id);

    assert_eq!(
        fx.invoke("stop_run", json!({ "request_id": id })),
        Ok(Value::Null)
    );
    let finished = fx.wait_finished(&id);
    assert_eq!(
        finished.submit(),
        &json!({ "status": "stopped", "passed": 0, "total": 0, "cases": [] })
    );
    assert_eq!(finished.payload["error"], Value::Null);
    assert_eq!(finished.payload["progress"], Value::Null);
    // 止めた提出は記録せず、レッスンを完了にしない。
    assert!(submit_events(finished.log).is_empty());
    assert!(fx.submit_events(STDOUT_LESSON).is_empty());
    assert!(!fx.lesson_completed(STDOUT_LESSON));
    wait_until("判定のプログラムが止まらない", || {
        fx.process_count() == 0
    });
    // 終わった提出の ID には、何もしない。
    assert_eq!(
        fx.invoke("stop_run", json!({ "request_id": id })),
        Ok(Value::Null)
    );
}

#[test]
fn stopping_while_hidden_tests_run_in_parallel_stops_them_all() {
    let fx = Fixture::new();
    let id = request_id(SLOW_TEST_LESSON, "s1");
    let code = "fn add(a: i32, b: i32) -> i32 {\n    a + b\n}\n\nfn main() {}\n";
    fx.submit(SLOW_TEST_LESSON, "s1", &[("src/main.rs", code)])
        .unwrap();
    fx.wait_judging(&id);
    // 隠しテストの名前を得て、テストのプロセスが動き出すまで待つ。
    thread::sleep(Duration::from_secs(1));

    let started = Instant::now();
    fx.invoke("stop_run", json!({ "request_id": id })).unwrap();
    let finished = fx.wait_finished(&id);
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "{:?}",
        started.elapsed()
    );
    assert_eq!(finished.submit()["status"], "stopped");
    assert!(submit_events(finished.log).is_empty());
    assert!(!fx.lesson_completed(SLOW_TEST_LESSON));
    wait_until("隠しテストのプロセスが止まらない", || {
        fx.process_count() == 0
    });
}

#[test]
fn stopping_during_the_build_gives_stopped_without_the_build_output() {
    let fx = Fixture::new();
    let id = request_id(STDOUT_LESSON, "s1");
    fx.submit(STDOUT_LESSON, "s1", &[("src/main.rs", ENDLESS_COMPILE)])
        .unwrap();
    wait_until("ビルドの rustc が起動しない", || {
        fx.process_count() > 0
    });

    fx.invoke("stop_run", json!({ "request_id": id })).unwrap();
    let finished = fx.wait_finished(&id);
    assert_eq!(finished.submit()["status"], "stopped");
    assert!(finished.outputs.is_empty(), "{:?}", finished.outputs);
    assert!(submit_events(finished.log).is_empty());
    wait_until("ビルドの rustc が止まらない", || {
        fx.process_count() == 0
    });
}

#[test]
fn quitting_the_app_stops_the_submit_without_run_finished_or_a_record() {
    let fx = Fixture::new();
    fx.submit(STDOUT_LESSON, "s1", &[("src/main.rs", SLOW_SUM)])
        .unwrap();
    fx.wait_judging(&request_id(STDOUT_LESSON, "s1"));

    stop_runs_on_exit(fx.app.handle());
    wait_until("判定のプログラムが止まらない", || {
        fx.process_count() == 0
    });
    fx.assert_no_finished_within(Duration::from_secs(2));
    assert!(fx.runs().current().is_none());
    assert!(fx.submit_events(STDOUT_LESSON).is_empty());
    assert!(!fx.lesson_completed(STDOUT_LESSON));
}

// ---- 同時に 1 つだけ ----

#[test]
fn a_running_submit_makes_runs_and_submits_busy_and_takes_no_stdin() {
    let fx = Fixture::new();
    let id = request_id(STDOUT_LESSON, "s1");
    fx.submit(STDOUT_LESSON, "s1", &[("src/main.rs", SLOW_SUM)])
        .unwrap();
    fx.wait_judging(&id);

    assert_eq!(
        error_code(fx.invoke(
            "run_code",
            json!({
                "request_id": "r1",
                "lesson_id": STDOUT_LESSON,
                "mode": "main",
                "files": { "src/main.rs": SUM },
            }),
        )),
        "busy"
    );
    assert_eq!(
        error_code(fx.submit(STDOUT_LESSON, "s2", &[("src/main.rs", SUM)])),
        "busy"
    );
    assert_eq!(
        error_code(fx.submit(STDOUT_LESSON, "s1", &[("src/main.rs", SUM)])),
        "invalid_argument"
    );
    assert_eq!(
        error_code(fx.invoke("send_stdin", json!({ "request_id": id, "text": "1\n" }))),
        "invalid_state"
    );
    // busy で断った実行・提出は、下書きを書き換えない。
    let draft = fx.data_dir().read_draft(STDOUT_LESSON).unwrap().unwrap();
    assert_eq!(draft.files["src/main.rs"], SLOW_SUM);

    // 止めずに待てば、判定を終えて正解になる。
    let finished = fx.wait_finished(&id);
    assert_eq!(finished.submit()["status"], "passed");
}

// ---- 受け付ける前に断るもの ----

#[test]
fn without_cargo_the_draft_is_saved_and_cargo_unavailable_is_returned() {
    let fx = Fixture::with(None);
    assert_eq!(
        error_code(fx.submit(STDOUT_LESSON, "s1", &[("src/main.rs", SUM)])),
        "cargo_unavailable"
    );
    let draft = fx.data_dir().read_draft(STDOUT_LESSON).unwrap().unwrap();
    assert_eq!(draft.files["src/main.rs"], SUM);
    assert!(fx.runs().current().is_none());
    fx.assert_no_finished_within(Duration::from_millis(200));
}
