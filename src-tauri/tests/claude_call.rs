//! `claude -p` を 1 回呼ぶ部品（src/claude/call.rs）を、偽の `claude` で確かめる
//! （spec/claude-integration/invocation.md、spec/claude-integration/errors.md）。

#[path = "fake_tools/claude.rs"]
mod claude;
#[path = "fake_tools/claude_print.rs"]
mod claude_print;

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
use progrust_lib::claude::{
    ClaudeCall, ClaudeEnv, ClaudeError, ClaudeOutcome, ClaudeRequest, ErrorCategory,
    call::STOP_GRACE,
};
use tempfile::TempDir;

/// アプリが受け取った環境変数に見立てる。6 つのほかに、渡してはいけない変数も入れる。
const APP_ENV_VARS: [(&str, &str); 10] = [
    ("HOME", "/Users/learner"),
    ("ANTHROPIC_API_KEY", "secret"),
    ("USER", "learner"),
    ("LOGNAME", "learner"),
    ("CLAUDE_CONFIG_DIR", "/tmp/claude-config"),
    ("PATH", "/usr/bin:/bin:/usr/sbin:/sbin"),
    ("LANG", "ja_JP.UTF-8"),
    ("SHELL", "/bin/zsh"),
    ("TMPDIR", "/var/folders/xx/progrust-test/T/"),
    ("HTTPS_PROXY", "http://proxy.example"),
];

/// `claude` に届くはずの環境変数（invocation.md「環境変数」）。
fn six_env_vars() -> BTreeMap<String, String> {
    [
        ("HOME", "/Users/learner"),
        ("USER", "learner"),
        ("LOGNAME", "learner"),
        ("PATH", "/usr/bin:/bin:/usr/sbin:/sbin"),
        ("SHELL", "/bin/zsh"),
        ("TMPDIR", "/var/folders/xx/progrust-test/T/"),
    ]
    .into_iter()
    .map(|(name, value)| (name.to_string(), value.to_string()))
    .collect()
}

const SYSTEM_PROMPT: &str = "あなたは Rust の先生です。\n完成したコードは返さないでください。";

/// 日本語と改行を含む、パイプの容量より大きい指示文。
fn prompt() -> String {
    "## 質問\n所有権とは何ですか？\n".repeat(4_000)
}

/// 偽の `claude` と、作業ディレクトリを作る場所。
struct Setup {
    fake: FakeClaude,
    work_parent: TempDir,
}

impl Setup {
    fn new() -> Self {
        Self {
            fake: FakeClaude::new(),
            work_parent: tempfile::tempdir().unwrap(),
        }
    }

    fn request(&self, model: &str, time_limit: Duration) -> ClaudeRequest {
        ClaudeRequest {
            claude: self.fake.path().to_path_buf(),
            env: ClaudeEnv::from_vars(APP_ENV_VARS),
            model: model.into(),
            system_prompt: SYSTEM_PROMPT.into(),
            prompt: prompt(),
            time_limit,
            work_dir_parent: self.work_parent.path().to_path_buf(),
        }
    }

    fn chat_request(&self) -> ClaudeRequest {
        self.request("haiku", Duration::from_secs(60))
    }

    /// 作業ディレクトリを作る場所にあるものの名前。
    fn work_entries(&self) -> Vec<String> {
        entries(self.work_parent.path())
    }

    /// 呼び出しの後に、作業ディレクトリが消えていることを確かめる。
    fn assert_work_dir_removed(&self) {
        assert_eq!(self.work_entries(), Vec::<String>::new());
    }
}

fn entries(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = fs::read_dir(dir)
        .unwrap()
        .map(|entry| entry.unwrap().file_name().into_string().unwrap())
        .collect();
    names.sort();
    names
}

/// 回答の断片を集めながら呼ぶ（質疑応答）。
fn run_chat(call: ClaudeCall) -> (ClaudeOutcome, Vec<String>) {
    let mut fragments = Vec::new();
    let outcome = call.run(Some(&mut |fragment: &str| {
        fragments.push(fragment.to_string())
    }));
    (outcome, fragments)
}

fn completed(text: &str) -> ClaudeOutcome {
    ClaudeOutcome::Completed { text: text.into() }
}

fn failed(category: ErrorCategory, detail: Option<&str>) -> ClaudeOutcome {
    ClaudeOutcome::Failed(ClaudeError {
        category,
        detail: detail.map(String::from),
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

/// 質疑応答の起動のフラグ（invocation.md「起動のフラグ」「出力形式」）。
fn chat_args(model: &str) -> Vec<String> {
    let mut args = job_args(model);
    args.push("--include-partial-messages".into());
    args
}

/// ジョブの起動のフラグ。
fn job_args(model: &str) -> Vec<String> {
    [
        "-p",
        "--model",
        model,
        "--safe-mode",
        "--setting-sources",
        "",
        "--strict-mcp-config",
        "--tools",
        "",
        "--disable-slash-commands",
        "--no-session-persistence",
        "--system-prompt",
        SYSTEM_PROMPT,
        "--output-format",
        "stream-json",
        "--verbose",
    ]
    .map(String::from)
    .to_vec()
}

/// 隔離のフラグ・6 つの環境変数・空の一時ディレクトリで起動し、指示文が標準入力で届き、断片が
/// 終わる前から順に届き、`result` の行で完了する。作業ディレクトリは終わった後に消える。
#[test]
fn a_chat_call_is_isolated_and_streams_fragments() {
    let setup = Setup::new();
    let first = format!(
        "{}{}",
        stream_json::answer_start(),
        stream_json::fragment("所有権は、")
    );
    let rest = format!(
        "{}{}{}",
        stream_json::fragment("値の持ち主が"),
        stream_json::fragment("1 つだけという決まりです。"),
        stream_json::answer_end("所有権は、値の持ち主が1 つだけという決まりです。")
    );
    setup.fake.set_stdout(&first);
    setup.fake.set_stdout_rest(&rest);

    let call = ClaudeCall::new(setup.chat_request());
    let mut fragments = Vec::new();
    let mut work_dir_during_call = None;
    let outcome = call.run(Some(&mut |fragment: &str| {
        if fragments.is_empty() {
            // 最初の断片は、claude が残りを書く前に届く。このときの作業ディレクトリを調べる。
            let names = setup.work_entries();
            assert_eq!(names.len(), 1, "作業ディレクトリは 1 つ: {names:?}");
            assert!(names[0].starts_with("progrust-claude-"), "{names:?}");
            let metadata = fs::metadata(setup.work_parent.path().join(&names[0])).unwrap();
            work_dir_during_call = Some((metadata.dev(), metadata.ino()));
            setup.fake.go();
        }
        fragments.push(fragment.to_string());
    }));

    assert_eq!(
        outcome,
        completed("所有権は、値の持ち主が1 つだけという決まりです。")
    );
    assert_eq!(
        fragments,
        ["所有権は、", "値の持ち主が", "1 つだけという決まりです。"]
    );

    let calls = setup.fake.calls();
    assert_eq!(calls.len(), 1, "{calls:?}");
    assert_eq!(calls[0].args, chat_args("haiku"));
    // AI にファイルを変えさせない（ツールを使わせない）。
    let tools = calls[0]
        .args
        .iter()
        .position(|arg| arg == "--tools")
        .unwrap();
    assert_eq!(calls[0].args[tools + 1], "");
    assert_eq!(calls[0].env, six_env_vars());
    assert_eq!(calls[0].stdin.as_deref(), Some(prompt().as_str()));

    let record = setup.fake.process_record(1).unwrap();
    assert_eq!(
        record.cwd_entries,
        Vec::<String>::new(),
        "空の作業ディレクトリ"
    );
    assert_eq!(Some(record.cwd_id), work_dir_during_call);
    assert_eq!(record.pgid, record.pid, "新しいプロセスグループで起動した");
    // SAFETY: 引数を取らない関数で、失敗しない。
    assert_ne!(record.pgid, unsafe { libc::getpgrp() });
    assert!(!is_alive(record.pid), "終わってから返る");
    setup.assert_work_dir_removed();
}

/// 断片を受け取らない呼び出し（ジョブ）は、`--include-partial-messages` を付けず、`result` の文を返す。
/// `--model` に渡したモデルが届く。
#[test]
fn a_job_call_returns_the_result_text() {
    let setup = Setup::new();
    // パスに空白を含む場所に置いた claude も起動できる。
    let fake = FakeClaude::place(&setup.work_parent.path().join("claude を置く 場所"));
    fake.set_stdout(&stream_json::answer(&["---\n", "summary: ok\n---\n"]));
    let mut request = setup.request("claude-opus-4-1", Duration::from_secs(60));
    request.claude = fake.path().to_path_buf();

    let outcome = ClaudeCall::new(request).run(None);

    assert_eq!(outcome, completed("---\nsummary: ok\n---\n"));
    let calls = fake.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].args, job_args("claude-opus-4-1"));
    assert_eq!(calls[0].env, six_env_vars());
    assert_eq!(calls[0].stdin.as_deref(), Some(prompt().as_str()));
    assert_eq!(setup.work_entries(), ["claude を置く 場所"]);
}

/// エラーの分類（errors.md「分類」）。`subtype` と終了コードは使わない。どの終わり方でも、
/// 作業ディレクトリは消える。
#[test]
fn errors_are_classified_from_the_stream() {
    let not_logged_in = stream_json::api_error(
        "authentication_failed",
        "Not logged in · Please run /login",
        None,
    );
    let model_message = "There's an issue with the selected model (claude-nonexistent-model-9).";
    let cases: [(String, &str, i32, ClaudeOutcome); 7] = [
        (
            not_logged_in.clone(),
            "Warning: this sign-in ends soon.\n",
            1,
            failed(ErrorCategory::NotLoggedIn, None),
        ),
        // 終了コードが 0 でも、`is_error` が `true` ならエラーになる。
        (
            not_logged_in,
            "",
            0,
            failed(ErrorCategory::NotLoggedIn, None),
        ),
        (
            stream_json::api_error("rate_limit", "Rate limited", Some(429)),
            "",
            1,
            failed(ErrorCategory::RateLimited, None),
        ),
        (
            stream_json::api_error("billing_error", "Billing error", Some(402)),
            "",
            1,
            failed(ErrorCategory::RateLimited, None),
        ),
        (
            stream_json::api_error("model_not_found", model_message, Some(404)),
            "[claude-code:unrecognized_model] {\"model\":\"claude-nonexistent-model-9\"}\n",
            1,
            failed(
                ErrorCategory::Other,
                Some(&format!(
                    "{model_message}\n\n標準エラー出力:\n[claude-code:unrecognized_model] {{\"model\":\"claude-nonexistent-model-9\"}}"
                )),
            ),
        ),
        // 終了コードが 1 でも、`is_error` が `false` なら完了になる。
        (stream_json::answer(&["はい。"]), "", 1, completed("はい。")),
        // `result` の行が無いまま終わった。
        (
            format!(
                "{}{}",
                stream_json::answer_start(),
                stream_json::fragment("途中")
            ),
            "Error: something went wrong\n",
            3,
            failed(
                ErrorCategory::Other,
                Some(
                    "claude が result の行を出さずに終わりました（終了コード 3）。\n\n標準エラー出力:\nError: something went wrong",
                ),
            ),
        ),
    ];
    for (stdout, stderr, exit_code, expected) in cases {
        let setup = Setup::new();
        setup.fake.set_stdout(&stdout);
        setup.fake.set_stderr(stderr);
        setup.fake.set_exit_code(exit_code);
        let (outcome, _) = run_chat(ClaudeCall::new(setup.chat_request()));
        assert_eq!(outcome, expected, "{stdout}");
        setup.assert_work_dir_removed();
    }
}

/// claude が自分で終わった後に、同じプロセスグループに残った子（出力のパイプを持ったまま）は止まり、
/// 呼び出しは制限時間を待たずに完了する（ADR 0260）。
#[test]
fn a_child_left_after_claude_exits_is_killed() {
    let setup = Setup::new();
    setup.fake.set_stdout(&stream_json::answer(&["はい。"]));
    setup.fake.set_spawn_child();

    let started = Instant::now();
    let (outcome, fragments) = run_chat(ClaudeCall::new(setup.chat_request()));

    assert_eq!(outcome, completed("はい。"));
    assert_eq!(fragments, ["はい。"]);
    assert!(started.elapsed() < Duration::from_secs(30));
    let child = setup.fake.process_record(1).unwrap().child_pid.unwrap();
    wait_until("claude の子が止まる", || !is_alive(child));
    setup.assert_work_dir_removed();
}

/// 制限時間を超えるとタイムアウトになり、SIGTERM でプロセスグループ（claude と、その子）が止まる。
#[test]
fn the_time_limit_stops_the_process_group() {
    let setup = Setup::new();
    setup.fake.set_stdout(&stream_json::answer_start());
    setup.fake.set_spawn_child();
    setup.fake.set_hang();
    let time_limit = Duration::from_secs(1);

    let started = Instant::now();
    let (outcome, _) = run_chat(ClaudeCall::new(setup.request("haiku", time_limit)));
    let elapsed = started.elapsed();

    assert_eq!(outcome, failed(ErrorCategory::Timeout, None));
    assert!(elapsed >= time_limit, "{elapsed:?}");
    assert!(
        elapsed < time_limit + STOP_GRACE,
        "SIGTERM で止まるはず: {elapsed:?}"
    );
    let record = setup.fake.process_record(1).unwrap();
    assert!(!is_alive(record.pid), "claude は終わってから返る");
    let child = record.child_pid.unwrap();
    wait_until("claude の子が止まる", || !is_alive(child));
    setup.assert_work_dir_removed();
}

/// SIGTERM を無視する claude とその子も、3 秒後の SIGKILL で止まる。
#[test]
fn a_claude_that_ignores_sigterm_is_killed() {
    let setup = Setup::new();
    setup.fake.set_ignore_sigterm();
    setup.fake.set_spawn_child();
    setup.fake.set_hang();
    let time_limit = Duration::from_secs(1);

    let started = Instant::now();
    let (outcome, _) = run_chat(ClaudeCall::new(setup.request("haiku", time_limit)));
    let elapsed = started.elapsed();

    assert_eq!(outcome, failed(ErrorCategory::Timeout, None));
    assert!(elapsed >= time_limit + STOP_GRACE, "{elapsed:?}");
    let record = setup.fake.process_record(1).unwrap();
    assert!(!is_alive(record.pid), "claude は終わってから返る");
    let child = record.child_pid.unwrap();
    wait_until("claude の子が止まる", || !is_alive(child));
    setup.assert_work_dir_removed();
}

/// 中断すると、claude が終わってから中断として返る。中断した後の断片は渡さない。
#[test]
fn cancelling_returns_after_the_process_ends() {
    let setup = Setup::new();
    setup.fake.set_stdout(&format!(
        "{}{}{}",
        stream_json::answer_start(),
        stream_json::fragment("1 つ目"),
        stream_json::fragment("2 つ目"),
    ));
    setup.fake.set_hang();
    let call = ClaudeCall::new(setup.chat_request());
    let canceller = call.canceller();

    let mut fragments = Vec::new();
    let outcome = call.run(Some(&mut |fragment: &str| {
        fragments.push(fragment.to_string());
        canceller.cancel();
    }));

    assert_eq!(outcome, ClaudeOutcome::Cancelled);
    assert_eq!(fragments, ["1 つ目"]);
    let record = setup.fake.process_record(1).unwrap();
    assert!(!is_alive(record.pid), "claude は終わってから返る");
    setup.assert_work_dir_removed();
}

/// 別のスレッドから中断できる。SIGTERM を無視する claude は、SIGKILL で止まってから中断として返る。
/// `result` の行を読んだ後でも、止めたら中断になる。
#[test]
fn cancelling_from_another_thread_waits_for_the_kill() {
    let setup = Setup::new();
    setup.fake.set_stdout(&stream_json::answer(&["回答"]));
    setup.fake.set_ignore_sigterm();
    setup.fake.set_spawn_child();
    setup.fake.set_hang();
    let call = ClaudeCall::new(setup.chat_request());
    let canceller = call.canceller();

    let (sender, received) = mpsc::channel();
    let caller = thread::spawn(move || {
        let (outcome, _) = run_chat(call);
        sender.send(Instant::now()).unwrap();
        outcome
    });
    wait_until("claude が出力を書き終える", || {
        setup
            .fake
            .calls()
            .first()
            .is_some_and(|call| call.stdin.is_some())
    });
    thread::sleep(Duration::from_millis(200));
    let cancelled_at = Instant::now();
    canceller.cancel();
    let outcome = caller.join().unwrap();
    let returned_at = received.recv().unwrap();

    assert_eq!(outcome, ClaudeOutcome::Cancelled);
    assert!(
        returned_at - cancelled_at >= STOP_GRACE,
        "{:?}",
        returned_at - cancelled_at
    );
    let record = setup.fake.process_record(1).unwrap();
    assert!(!is_alive(record.pid));
    let child = record.child_pid.unwrap();
    wait_until("claude の子が止まる", || !is_alive(child));
    setup.assert_work_dir_removed();
}

/// 起動する前に中断したら、起動しない。終わった後の中断は、何もしない。
#[test]
fn cancelling_before_or_after_the_call() {
    let setup = Setup::new();
    let call = ClaudeCall::new(setup.chat_request());
    call.canceller().cancel();
    assert_eq!(run_chat(call).0, ClaudeOutcome::Cancelled);
    assert!(setup.fake.calls().is_empty());
    setup.assert_work_dir_removed();

    setup.fake.set_stdout(&stream_json::answer(&["はい。"]));
    let call = ClaudeCall::new(setup.chat_request());
    let canceller = call.canceller();
    assert_eq!(run_chat(call).0, completed("はい。"));
    canceller.cancel();
    setup.assert_work_dir_removed();
}

/// モデルの値が条件（`-` で始まらない、空白を含まない、など）を満たさないときは、起動せずにその他のエラーになる。
#[test]
fn an_invalid_model_does_not_start_claude() {
    let setup = Setup::new();
    for model in ["--bare", "-p", "claude opus", "", "opus\n"] {
        let outcome = ClaudeCall::new(setup.request(model, Duration::from_secs(60))).run(None);
        let ClaudeOutcome::Failed(error) = outcome else {
            panic!("{model:?} で起動した: {outcome:?}");
        };
        assert_eq!(error.category, ErrorCategory::Other, "{model:?}");
        assert!(error.detail.is_some());
    }
    assert!(setup.fake.calls().is_empty(), "claude を起動しない");
    setup.assert_work_dir_removed();
}

/// claude を起動できないときは、その他のエラーになり、作業ディレクトリは消える。
#[test]
fn a_claude_that_cannot_start_is_other() {
    let setup = Setup::new();
    let mut request = setup.chat_request();
    request.claude = setup.work_parent.path().join("no-such-claude");
    let ClaudeOutcome::Failed(error) = ClaudeCall::new(request).run(None) else {
        panic!("起動できないはず");
    };
    assert_eq!(error.category, ErrorCategory::Other);
    assert!(
        error
            .detail
            .as_deref()
            .unwrap()
            .starts_with("claude を起動できませんでした"),
        "{error:?}"
    );
    setup.assert_work_dir_removed();
}
