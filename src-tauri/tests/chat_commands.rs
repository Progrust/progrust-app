//! 質疑応答のコマンド `send_chat`・`cancel_chat` とイベント `chat_delta`・`chat_finished`
//! （spec/architecture/commands/chat.md、spec/qa-chat/attempt-log.md、spec/qa-chat/history.md）を、
//! モックの実行環境と偽の `claude` で確かめる。

#[path = "fake_tools/claude.rs"]
mod claude;
#[path = "fake_tools/claude_print.rs"]
mod claude_print;
mod common;

use std::{
    collections::BTreeMap,
    fs,
    os::unix::fs::MetadataExt,
    path::Path,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use claude::FakeClaude;
use claude_print::stream_json;
use common::{LOCAL_URL, app, invoke};
use progrust_lib::attempts::{
    AttemptLog, BuildOutput, ChatEvent, Event, SessionKind, SubmitRecord, SubmitStatus,
};
use progrust_lib::chat::PromptDir;
use progrust_lib::chat::answering::{CHAT_DELTA, CHAT_FINISHED, CHAT_TIME_LIMIT, Chats};
use progrust_lib::claude::ENV_VAR_NAMES;
use progrust_lib::commands::chat::{CHAT_SYSTEM_PROMPT, stop_chats_on_exit};
use progrust_lib::content::ContentDir;
use progrust_lib::environment::{ClaudeTool, DetectedTools, Environment};
use progrust_lib::storage::AppDataDir;
use serde_json::{Value, json};
use tauri::test::MockRuntime;
use tauri::{App, Listener, Manager, WebviewWindow};
use tempfile::TempDir;

/// イベントを待つ最長の時間。
const WAIT: Duration = Duration::from_secs(30);

/// 動画と演習のあるレッスン。
const LESSON: &str = "ch01-01";
/// 動画のみのレッスン。
const VIDEO_ONLY: &str = "ch01-02";
/// 動画の無い章末復習レッスン。
const REVIEW: &str = "ch01-review";

/// 学習者が正解したときのコードに入れる目印。
const LEARNER_CODE: &str = "fn main() {\n    println!(\"LEARNER_CODE_MARKER\");\n}\n";

fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn write_json(root: &Path, relative: &str, value: Value) {
    write(root, relative, &value.to_string());
}

fn lesson_json(id: &str, kind: &str, primary: &[&str]) -> Value {
    json!({
        "id": id, "chapter": "ch01", "kind": kind, "title": format!("レッスン{id}"),
        "summary": "概要", "primary_elements": primary, "secondary_elements": [],
        "estimated_minutes": 10
    })
}

fn script_json() -> Value {
    json!({
        "code_blocks": [],
        "scenes": [{ "id": "title", "name": "表紙" }, { "id": "move", "name": "ムーブ" }],
        "sentences": [
            { "id": "t1", "scene": "title", "text": "所有権のムーブを学びます。", "say": "よみ" },
            { "id": "m1", "scene": "move", "text": "SENTENCE_M1_MARKER の文です。", "say": "よみ" },
            { "id": "m2", "scene": "move", "text": "SENTENCE_M2_MARKER の文です。", "say": "よみ" }
        ]
    })
}

fn write_exercise(root: &Path, id: &str) {
    let dir = format!("lessons/{id}/exercise");
    write(
        root,
        &format!("{dir}/problem.md"),
        "# 問題\n\nPROBLEM_MARKER\n",
    );
    write(
        root,
        &format!("{dir}/explanation.md"),
        "# 解説\n\nEXPLANATION_MARKER\n",
    );
    write(
        root,
        &format!("{dir}/solution/src/main.rs"),
        "fn main() {\n    println!(\"SOLUTION_MARKER\");\n}\n",
    );
}

fn make_content() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write_json(
        root,
        "chapters.json",
        json!([{ "id": "ch01", "title": "第1章", "description": "",
                 "lessons": [LESSON, VIDEO_ONLY, REVIEW] }]),
    );
    write_json(
        root,
        "elements.json",
        json!([{ "id": "a1", "chapter": "ch01", "name": "要素a1", "description": "説明",
                 "related_errors": [] }]),
    );
    write_json(
        root,
        &format!("lessons/{LESSON}/lesson.json"),
        lesson_json(LESSON, "lesson", &["a1"]),
    );
    write_json(
        root,
        &format!("lessons/{LESSON}/video/script.json"),
        script_json(),
    );
    write_exercise(root, LESSON);
    write_json(
        root,
        &format!("lessons/{VIDEO_ONLY}/lesson.json"),
        lesson_json(VIDEO_ONLY, "video_only", &[]),
    );
    write_json(
        root,
        &format!("lessons/{VIDEO_ONLY}/video/script.json"),
        script_json(),
    );
    write_json(
        root,
        &format!("lessons/{REVIEW}/lesson.json"),
        lesson_json(REVIEW, "chapter_review", &["a1"]),
    );
    write_exercise(root, REVIEW);
    dir
}

/// 受け取ったイベント。
enum Received {
    Delta(Value),
    Finished(Value),
}

/// 1 つの質問について受け取ったイベント。
struct Answer {
    deltas: Vec<Value>,
    finished: Value,
}

struct Fixture {
    _content: TempDir,
    data: TempDir,
    /// `claude` の作業ディレクトリを作る場所。
    work: TempDir,
    fake: FakeClaude,
    app: App<MockRuntime>,
    window: WebviewWindow<MockRuntime>,
    received: mpsc::Receiver<Received>,
}

impl Fixture {
    fn new() -> Self {
        Self::with(true, CHAT_TIME_LIMIT)
    }

    fn with_claude(found: bool) -> Self {
        Self::with(found, CHAT_TIME_LIMIT)
    }

    fn with(found: bool, time_limit: Duration) -> Self {
        let content = make_content();
        let data = tempfile::tempdir().unwrap();
        let work = tempfile::tempdir().unwrap();
        let fake = FakeClaude::new();
        let (app, window) = app();
        app.manage(ContentDir::new(content.path()));
        app.manage(AppDataDir::new(data.path()));
        app.manage(PromptDir::new(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../prompts"),
        ));
        app.manage(Environment::detected(DetectedTools {
            cargo: None,
            claude: found.then(|| ClaudeTool {
                path: fake.path().to_path_buf(),
                version: "2.1.329".into(),
            }),
            clang: None,
        }));
        app.manage(Chats::new(time_limit, work.path().to_path_buf()));

        let (sender, received) = mpsc::channel();
        {
            let sender = sender.clone();
            app.listen(CHAT_DELTA, move |event| {
                let payload = serde_json::from_str(event.payload()).unwrap();
                let _ = sender.send(Received::Delta(payload));
            });
        }
        app.listen(CHAT_FINISHED, move |event| {
            let payload = serde_json::from_str(event.payload()).unwrap();
            let _ = sender.send(Received::Finished(payload));
        });
        Self {
            _content: content,
            data,
            work,
            fake,
            app,
            window,
            received,
        }
    }

    fn data_dir(&self) -> AppDataDir {
        AppDataDir::new(self.data.path())
    }

    /// 設定の質疑応答のモデルを `chat`、バックグラウンドのモデルを `background` にする。
    fn set_models(&self, chat: &str, background: &str) {
        write_json(
            self.data.path(),
            "settings.json",
            json!({
                "schema_version": 2,
                "theme": "system",
                "editor_font_size": 14,
                "video": { "volume": 1.0, "muted": false, "playback_rate": 1.0, "captions": false },
                "models": { "chat": chat, "background": background },
                "tool_paths": { "cargo": null, "claude": null, "clang": null },
                "first_launch_completed": true
            }),
        );
    }

    /// レッスンを開いたときのように、試行セッションを始める。
    fn open_session(&self, lesson_id: &str) -> AttemptLog {
        self.data_dir()
            .start_or_continue_session(lesson_id, SessionKind::Lesson)
            .unwrap()
    }

    /// 試行セッションを始め、正解した提出を記録する。
    fn open_passed_session(&self, lesson_id: &str) -> AttemptLog {
        self.open_session(lesson_id);
        self.data_dir()
            .append_submit(
                lesson_id,
                SessionKind::Lesson,
                SubmitRecord {
                    at: "2026-10-31T13:20:00+09:00".into(),
                    files: BTreeMap::from([("src/main.rs".to_string(), LEARNER_CODE.to_string())]),
                    status: SubmitStatus::Passed,
                    error_codes: Vec::new(),
                    passed: 1,
                    total: 1,
                    failed_cases: Vec::new(),
                    build: BuildOutput::default(),
                    program_stderr: String::new(),
                },
            )
            .unwrap();
        self.data_dir()
            .unfinished_session(lesson_id)
            .unwrap()
            .unwrap()
    }

    fn send(&self, args: Value) -> Result<Value, Value> {
        invoke(&self.window, LOCAL_URL, "send_chat", args)
    }

    fn cancel(&self, request_id: &str) -> Result<Value, Value> {
        invoke(
            &self.window,
            LOCAL_URL,
            "cancel_chat",
            json!({ "request_id": request_id }),
        )
    }

    /// 次の `chat_delta` を待つ。
    fn next_delta(&self) -> Value {
        match self
            .received
            .recv_timeout(WAIT)
            .expect("chat_delta が届かない")
        {
            Received::Delta(delta) => delta,
            Received::Finished(finished) => panic!("chat_delta の前に chat_finished: {finished}"),
        }
    }

    /// `chat_finished` まで、受け取ったイベントを集める。
    fn answer(&self) -> Answer {
        let mut deltas = Vec::new();
        loop {
            match self
                .received
                .recv_timeout(WAIT)
                .expect("chat_finished が届かない")
            {
                Received::Delta(delta) => deltas.push(delta),
                Received::Finished(finished) => return Answer { deltas, finished },
            }
        }
    }

    /// しばらく待っても、イベントが届かないことを確かめる。
    fn assert_no_more_events(&self) {
        if let Ok(event) = self.received.recv_timeout(Duration::from_millis(500)) {
            let payload = match event {
                Received::Delta(value) | Received::Finished(value) => value,
            };
            panic!("イベントが余分に届いた: {payload}");
        }
    }

    /// レッスン `lesson_id` の未完了のセッションの `chat` イベント。セッションが無いときは空。
    fn chat_events(&self, lesson_id: &str) -> Vec<ChatEvent> {
        self.data_dir()
            .unfinished_session(lesson_id)
            .unwrap()
            .map(|log| {
                log.events
                    .into_iter()
                    .filter_map(|event| match event {
                        Event::Chat(chat) => Some(chat),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// `claude` の作業ディレクトリを作る場所にあるもの（デバイスの番号と inode の番号）。
    fn work_dirs(&self) -> Vec<(u64, u64)> {
        fs::read_dir(self.work.path())
            .unwrap()
            .map(|entry| {
                let metadata = entry.unwrap().metadata().unwrap();
                (metadata.dev(), metadata.ino())
            })
            .collect()
    }

    /// 偽の `claude` が受け取った標準入力（`number` 回目の呼び出し。1 から数える）。
    fn stdin(&self, number: usize) -> String {
        self.fake.calls()[number - 1]
            .stdin
            .clone()
            .expect("標準入力が記録されていない")
    }
}

fn video_args(request_id: &str, lesson_id: &str, sentence_id: &str, question: &str) -> Value {
    json!({
        "request_id": request_id,
        "lesson_id": lesson_id,
        "context": "video",
        "sentence_id": sentence_id,
        "question": question,
        "history": [],
        "retry": false,
    })
}

fn explanation_args(request_id: &str, lesson_id: &str, question: &str) -> Value {
    json!({
        "request_id": request_id,
        "lesson_id": lesson_id,
        "context": "explanation",
        "sentence_id": null,
        "question": question,
        "history": [],
        "retry": false,
    })
}

fn is_alive(pid: i32) -> bool {
    // SAFETY: シグナル 0 は送らずに、プロセスがあるかだけを調べる。
    unsafe { libc::kill(pid, 0) == 0 }
}

/// `condition` が成り立つまで待つ。
fn wait_until(what: &str, condition: impl Fn() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !condition() {
        assert!(Instant::now() < deadline, "{what}まで待てなかった");
        thread::sleep(Duration::from_millis(20));
    }
}

/// 回答の断片が `chat_delta` で順に届き、つなぐと全文になり、最後に `chat_finished`（`completed`）が 1 回届く。
/// 質問は、再生中だった文の ID を `video:` に続けた `context` で試行ログに記録される。
#[test]
fn a_video_question_streams_the_answer_and_is_recorded() {
    let fixture = Fixture::new();
    let session = fixture.open_session(LESSON);
    let fragments = ["所有権は、", "値を持つ", "変数のことです。"];
    fixture.fake.set_stdout(&stream_json::answer(&fragments));

    let response = fixture.send(video_args("q1", LESSON, "m2", "所有者とは？"));
    assert_eq!(response, Ok(Value::Null));
    let answer = fixture.answer();
    assert_eq!(
        answer.finished,
        json!({ "request_id": "q1", "status": "completed", "error": null })
    );
    assert!(
        answer
            .deltas
            .iter()
            .all(|delta| delta["request_id"] == "q1")
    );
    let text: String = answer
        .deltas
        .iter()
        .map(|delta| delta["text"].as_str().unwrap())
        .collect();
    assert_eq!(text, fragments.concat());
    fixture.assert_no_more_events();

    let events = fixture.chat_events(LESSON);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].context, "video:m2");
    assert_eq!(events[0].question, "所有者とは？");
    assert_eq!(events[0].answer_summary, None);
    assert!(chrono::DateTime::parse_from_rfc3339(&events[0].at).is_ok());
    assert_eq!(
        fixture
            .data_dir()
            .unfinished_session(LESSON)
            .unwrap()
            .unwrap()
            .session_id,
        session.session_id
    );

    // 再生中だった文と質問が、動画の再生中の指示文で渡る。演習の模範解答は渡らない。
    let stdin = fixture.stdin(1);
    assert!(
        stdin.contains("[m2] SENTENCE_M2_MARKER の文です。"),
        "{stdin}"
    );
    assert!(stdin.contains("所有者とは？"));
    assert!(!stdin.contains("SOLUTION_MARKER"));
}

/// 質疑応答の呼び出しには、設定の `models.chat` が `--model` で渡り、質疑応答のフラグとシステムプロンプトが付く。
#[test]
fn the_chat_model_from_settings_is_passed_to_claude() {
    let fixture = Fixture::new();
    fixture.set_models("haiku", "opus");
    fixture.fake.set_stdout(&stream_json::answer(&["はい。"]));
    fixture
        .send(video_args("q1", VIDEO_ONLY, "t1", "質問"))
        .unwrap();
    assert_eq!(fixture.answer().finished["status"], "completed");

    let calls = fixture.fake.calls();
    assert_eq!(calls.len(), 1);
    let args = &calls[0].args;
    assert!(
        args.windows(2).any(|w| w == ["--model", "haiku"]),
        "{args:?}"
    );
    assert!(
        args.windows(2)
            .any(|w| w == ["--system-prompt", CHAT_SYSTEM_PROMPT]),
        "{args:?}"
    );
    assert!(args.iter().any(|arg| arg == "--include-partial-messages"));
    assert!(
        calls[0]
            .env
            .keys()
            .all(|name| ENV_VAR_NAMES.contains(&name.as_str()))
    );
}

/// 解説表示中の質問は `explanation` で記録され、正解したときのコードが入力に入る。
#[test]
fn an_explanation_question_is_recorded_as_explanation() {
    let fixture = Fixture::new();
    fixture.open_passed_session(LESSON);
    fixture
        .fake
        .set_stdout(&stream_json::answer(&["解説です。"]));
    fixture
        .send(explanation_args("q1", LESSON, "なぜこの解法？"))
        .unwrap();
    assert_eq!(fixture.answer().finished["status"], "completed");

    let events = fixture.chat_events(LESSON);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].context, "explanation");
    assert_eq!(events[0].question, "なぜこの解法？");
    let stdin = fixture.stdin(1);
    assert!(stdin.contains("LEARNER_CODE_MARKER"), "{stdin}");
    assert!(stdin.contains("SOLUTION_MARKER"));
}

/// 章末復習レッスンの解説表示中も質問できる。
#[test]
fn a_chapter_review_explanation_question_is_accepted() {
    let fixture = Fixture::new();
    fixture.open_passed_session(REVIEW);
    fixture.fake.set_stdout(&stream_json::answer(&["はい。"]));
    assert_eq!(
        fixture.send(explanation_args("q1", REVIEW, "質問")),
        Ok(Value::Null)
    );
    assert_eq!(fixture.answer().finished["status"], "completed");
    assert_eq!(fixture.chat_events(REVIEW).len(), 1);
}

/// 「再試行」（`retry` が `true`）は記録しない。回答は同じように届く。
#[test]
fn a_retry_is_not_recorded() {
    let fixture = Fixture::new();
    fixture.open_session(LESSON);
    fixture.fake.set_stdout(&stream_json::answer(&["はい。"]));
    let mut args = video_args("q1", LESSON, "m1", "質問");
    args["retry"] = json!(true);
    assert_eq!(fixture.send(args), Ok(Value::Null));
    assert_eq!(fixture.answer().finished["status"], "completed");
    assert_eq!(fixture.chat_events(LESSON), []);
    assert_eq!(fixture.fake.calls().len(), 1);
}

/// 動画のみのレッスンには試行セッションが無いので、記録しない（セッションも作らない）。
#[test]
fn a_question_in_a_video_only_lesson_is_not_recorded() {
    let fixture = Fixture::new();
    fixture.fake.set_stdout(&stream_json::answer(&["はい。"]));
    assert_eq!(
        fixture.send(video_args("q1", VIDEO_ONLY, "m1", "質問")),
        Ok(Value::Null)
    );
    assert_eq!(fixture.answer().finished["status"], "completed");
    assert!(!fixture.data.path().join("attempts").exists());
}

/// 引数・場面・`claude` の条件を満たさないときはエラーを返し、`claude` を呼ばず、記録しない。
#[test]
fn rejected_questions_return_errors_and_are_not_recorded() {
    let fixture = Fixture::new();
    fixture.open_session(LESSON);
    fixture.open_session(REVIEW);
    let over = "あ".repeat(2_001);
    let mut null_sentence = video_args("q", LESSON, "m1", "質問");
    null_sentence["sentence_id"] = Value::Null;
    let cases = [
        (video_args("q", LESSON, "m1", ""), "invalid_argument"),
        (video_args("q", LESSON, "m1", &over), "invalid_argument"),
        (null_sentence, "invalid_argument"),
        // 台本に無い文の ID。
        (video_args("q", LESSON, "x9", "質問"), "invalid_argument"),
        // 正解していない演習の解説表示中。
        (explanation_args("q", LESSON, "質問"), "invalid_state"),
        // 演習の無いレッスンの解説表示中。
        (explanation_args("q", VIDEO_ONLY, "質問"), "invalid_state"),
        // 動画の無いレッスンの動画の再生中。
        (video_args("q", REVIEW, "m1", "質問"), "invalid_state"),
        (video_args("q", "ch09-09", "m1", "質問"), "not_found"),
        (explanation_args("q", "ch09-09", "質問"), "not_found"),
    ];
    for (args, code) in cases {
        let error = fixture.send(args.clone()).unwrap_err();
        assert_eq!(error["code"], json!(code), "{args}");
    }
    assert!(fixture.fake.calls().is_empty());
    assert_eq!(fixture.chat_events(LESSON), []);
    assert_eq!(fixture.chat_events(REVIEW), []);
    fixture.assert_no_more_events();
}

/// 2,000 文字ちょうどの質問は送れる。
#[test]
fn a_question_of_2000_chars_is_accepted() {
    let fixture = Fixture::new();
    fixture.fake.set_stdout(&stream_json::answer(&["はい。"]));
    let question = "あ".repeat(2_000);
    assert_eq!(
        fixture.send(video_args("q1", VIDEO_ONLY, "m1", &question)),
        Ok(Value::Null)
    );
    assert_eq!(fixture.answer().finished["status"], "completed");
}

#[test]
fn without_claude_the_question_is_claude_unavailable() {
    let fixture = Fixture::with_claude(false);
    fixture.open_session(LESSON);
    let error = fixture
        .send(video_args("q1", LESSON, "m1", "質問"))
        .unwrap_err();
    assert_eq!(error["code"], json!("claude_unavailable"));
    assert_eq!(fixture.chat_events(LESSON), []);
    fixture.assert_no_more_events();
}

/// 回答中は、ほかの質問は `busy`、同じ ID の質問は `invalid_argument`。エラーの質問は記録しない。
/// 回答が終われば、次の質問を送れる。
#[test]
fn a_second_question_while_answering_is_busy() {
    let fixture = Fixture::new();
    fixture.open_session(LESSON);
    fixture.fake.set_stdout(&format!(
        "{}{}",
        stream_json::answer_start(),
        stream_json::fragment("途中")
    ));
    fixture
        .fake
        .set_stdout_rest(&stream_json::answer_end("途中まで"));
    fixture
        .send(video_args("q1", LESSON, "m1", "一つ目"))
        .unwrap();
    assert_eq!(fixture.next_delta()["text"], "途中");

    let busy = fixture
        .send(video_args("q2", LESSON, "m1", "二つ目"))
        .unwrap_err();
    assert_eq!(busy["code"], json!("busy"));
    let same = fixture
        .send(video_args("q1", LESSON, "m1", "一つ目"))
        .unwrap_err();
    assert_eq!(same["code"], json!("invalid_argument"));
    assert_eq!(fixture.fake.calls().len(), 1);

    fixture.fake.go();
    let answer = fixture.answer();
    assert_eq!(answer.finished["request_id"], "q1");
    assert_eq!(answer.finished["status"], "completed");

    // 終わった後は、次の質問を送れる。
    fs::remove_file(fixture.fake.path().with_file_name("stdout-rest")).unwrap();
    fixture
        .fake
        .set_stdout(&stream_json::answer(&["二つ目の回答"]));
    fixture
        .send(video_args("q2", LESSON, "m1", "二つ目"))
        .unwrap();
    assert_eq!(fixture.answer().finished["status"], "completed");

    let questions: Vec<String> = fixture
        .chat_events(LESSON)
        .into_iter()
        .map(|event| event.question)
        .collect();
    assert_eq!(questions, ["一つ目", "二つ目"]);
}

/// `claude` のエラーは、`chat_finished`（`error`）と分類で知らせる。
#[test]
fn a_claude_error_finishes_with_the_category() {
    let fixture = Fixture::new();
    fixture.fake.set_stdout(&stream_json::api_error(
        "authentication_failed",
        "Not logged in · Please run /login",
        None,
    ));
    fixture.fake.set_exit_code(1);
    fixture
        .send(video_args("q1", VIDEO_ONLY, "m1", "質問"))
        .unwrap();
    assert_eq!(
        fixture.answer().finished,
        json!({
            "request_id": "q1",
            "status": "error",
            "error": { "category": "not_logged_in", "detail": null }
        })
    );
}

/// `result` の行を出さずに終わったときは「その他」のエラーで、詳細に標準エラー出力の先頭が入る。
#[test]
fn a_claude_that_ends_without_a_result_is_other_error_with_stderr() {
    let fixture = Fixture::new();
    fixture.fake.set_stdout(&stream_json::answer_start());
    fixture.fake.set_stderr("STDERR_MARKER\n");
    fixture.fake.set_exit_code(1);
    fixture
        .send(video_args("q1", VIDEO_ONLY, "m1", "質問"))
        .unwrap();
    let finished = fixture.answer().finished;
    assert_eq!(finished["status"], "error");
    assert_eq!(finished["error"]["category"], "other");
    let detail = finished["error"]["detail"].as_str().unwrap();
    assert!(detail.contains("STDERR_MARKER"), "{detail}");
}

/// 設定のモデルの値が正しくないときは、`claude` を起動せず、「その他」のエラーで知らせる。
#[test]
fn an_invalid_model_finishes_as_other_error_without_starting_claude() {
    let fixture = Fixture::new();
    fixture.set_models("--bare", "opus");
    fixture
        .send(video_args("q1", VIDEO_ONLY, "m1", "質問"))
        .unwrap();
    let finished = fixture.answer().finished;
    assert_eq!(finished["status"], "error");
    assert_eq!(finished["error"]["category"], "other");
    assert!(fixture.fake.calls().is_empty());
}

/// `claude` を起動できなかったときは、分類を `other` にする。
#[test]
fn a_claude_that_cannot_be_started_finishes_as_other_error() {
    let fixture = Fixture::new();
    fs::remove_file(fixture.fake.path()).unwrap();
    fixture
        .send(video_args("q1", VIDEO_ONLY, "m1", "質問"))
        .unwrap();
    let finished = fixture.answer().finished;
    assert_eq!(finished["status"], "error");
    assert_eq!(finished["error"]["category"], "other");
    assert!(finished["error"]["detail"].is_string());
}

/// 制限時間を超えた回答は、`claude` を止めてから、`chat_finished`（`error`）のタイムアウトで知らせる。
#[test]
fn a_chat_that_times_out_finishes_as_timeout() {
    let fixture = Fixture::with(true, Duration::from_millis(300));
    fixture.fake.set_stdout(&format!(
        "{}{}",
        stream_json::answer_start(),
        stream_json::fragment("途中")
    ));
    fixture.fake.set_hang();
    fixture
        .send(video_args("q1", VIDEO_ONLY, "m1", "質問"))
        .unwrap();
    let answer = fixture.answer();
    assert_eq!(
        answer.finished,
        json!({
            "request_id": "q1",
            "status": "error",
            "error": { "category": "timeout", "detail": null }
        })
    );
    let record = fixture.fake.process_record(1).unwrap();
    assert!(!is_alive(record.pid));
    assert_eq!(fixture.work_dirs(), []);
}

/// `cancel_chat` で止めると、`chat_finished`（`cancelled`）が届き、`claude` のプロセスと、同じプロセスグループの
/// 子のプロセスが残らない。回答中でない ID の `cancel_chat` は、何もしない。
#[test]
fn cancel_chat_stops_claude_and_finishes_as_cancelled() {
    let fixture = Fixture::new();
    fixture.open_session(LESSON);
    fixture.fake.set_stdout(&format!(
        "{}{}",
        stream_json::answer_start(),
        stream_json::fragment("途中")
    ));
    fixture.fake.set_spawn_child();
    fixture.fake.set_hang();
    assert_eq!(fixture.cancel("unknown"), Ok(Value::Null));

    fixture
        .send(video_args("q1", LESSON, "m1", "質問"))
        .unwrap();
    assert_eq!(fixture.next_delta()["text"], "途中");
    let record = fixture.fake.process_record(1).unwrap();
    assert!(is_alive(record.pid));
    // 新しいプロセスグループで、呼び出しごとに作った空の作業ディレクトリで起動している。
    assert_eq!(record.pgid, record.pid);
    assert_eq!(record.cwd_entries, Vec::<String>::new());
    assert_eq!(fixture.work_dirs(), [record.cwd_id]);

    assert_eq!(fixture.cancel("other"), Ok(Value::Null));
    assert_eq!(fixture.cancel("q1"), Ok(Value::Null));
    let answer = fixture.answer();
    assert_eq!(
        answer.finished,
        json!({ "request_id": "q1", "status": "cancelled", "error": null })
    );
    assert!(answer.deltas.is_empty());
    // chat_finished は、claude のプロセスが終わってから届く。
    assert!(!is_alive(record.pid));
    let child = record.child_pid.unwrap();
    wait_until("claude の子が止まる", || !is_alive(child));
    // 作業ディレクトリは、終わったら消える。
    assert_eq!(fixture.work_dirs(), []);
    fixture.assert_no_more_events();

    // 止めた質問も、送信したときに記録してある。終わった質問の ID の cancel_chat は、何もしない。
    assert_eq!(fixture.chat_events(LESSON).len(), 1);
    assert_eq!(fixture.cancel("q1"), Ok(Value::Null));
    fixture.assert_no_more_events();
}

/// アプリの終了では、回答中の `claude`（SIGTERM を無視するものも）を止め、終わるまで待つ。`chat_finished` は送らない。
#[test]
fn app_exit_stops_the_answering_claude_without_events() {
    let fixture = Fixture::new();
    fixture.fake.set_stdout(&format!(
        "{}{}",
        stream_json::answer_start(),
        stream_json::fragment("途中")
    ));
    fixture.fake.set_ignore_sigterm();
    fixture.fake.set_hang();
    fixture
        .send(video_args("q1", VIDEO_ONLY, "m1", "質問"))
        .unwrap();
    assert_eq!(fixture.next_delta()["text"], "途中");
    let record = fixture.fake.process_record(1).unwrap();

    stop_chats_on_exit(fixture.app.handle());
    assert!(!is_alive(record.pid));
    fixture.assert_no_more_events();
    assert_eq!(fixture.app.state::<Chats>().current_request_id(), None);
}

/// 標準入力の指示文には、`history` の末尾の 10 往復までと、今回の質問が入る。止めた往復の途中までの回答も、
/// `history` に入っていれば渡る。
#[test]
fn the_prompt_has_the_last_10_turns_of_history_and_the_question() {
    let fixture = Fixture::new();
    fixture.fake.set_stdout(&stream_json::answer(&["はい。"]));
    let history: Vec<Value> = (1..=12)
        .map(|n| json!({ "question": format!("質問-{n:02}"), "answer": format!("回答-{n:02}") }))
        .collect();
    let mut args = video_args("q1", VIDEO_ONLY, "m1", "今回の質問です");
    args["history"] = Value::Array(history);
    fixture.send(args).unwrap();
    assert_eq!(fixture.answer().finished["status"], "completed");

    let stdin = fixture.stdin(1);
    for n in 1..=2 {
        assert!(!stdin.contains(&format!("質問-{n:02}")), "{n}");
        assert!(!stdin.contains(&format!("回答-{n:02}")), "{n}");
    }
    let mut last = 0;
    for n in 3..=12 {
        let question = stdin
            .find(&format!("質問-{n:02}"))
            .unwrap_or_else(|| panic!("質問-{n:02} が無い"));
        let answer = stdin
            .find(&format!("回答-{n:02}"))
            .unwrap_or_else(|| panic!("回答-{n:02} が無い"));
        assert!(last < question && question < answer, "{n}");
        last = answer;
    }
    let question = stdin.find("今回の質問です").unwrap();
    assert!(last < question);
}

/// 引数の形が違うとき（必須の項目が無い、`context` の値が違う）は `invalid_argument`。
#[test]
fn malformed_arguments_are_invalid_argument() {
    let fixture = Fixture::new();
    let error = fixture
        .send(json!({ "request_id": "q1", "lesson_id": LESSON }))
        .unwrap_err();
    assert_eq!(error["code"], json!("invalid_argument"));
    let mut bad_context = video_args("q1", LESSON, "m1", "質問");
    bad_context["context"] = json!("exercise");
    let error = fixture.send(bad_context).unwrap_err();
    assert_eq!(error["code"], json!("invalid_argument"));
    assert!(fixture.fake.calls().is_empty());
}
