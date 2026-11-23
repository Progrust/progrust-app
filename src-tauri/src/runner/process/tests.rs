//! 子プロセスの部品のテスト。学習者のプログラムの代わりに `/bin/sh` のスクリプトを動かす。

use std::{
    path::Path,
    sync::mpsc,
    time::{Duration, Instant},
};

use super::*;

/// テストで使う制限値。止まらないプログラムでも、テストが止まらなくならないようにする。
const LIMITS: Limits = Limits {
    time: Duration::from_secs(20),
    output: OutputLimit::Total(1024 * 1024),
};
/// 状態が変わるのを待つ最長の時間。
const WAIT: Duration = Duration::from_secs(10);

fn sh(script: &str, dir: &Path) -> ProcessSpec {
    let mut spec = ProcessSpec::new("/bin/sh", dir, LIMITS);
    spec.args = vec!["-c".into(), script.into()];
    spec
}

/// 出力の断片を、受け取ったそばから読めるようにして起動する。
fn spawn_with_channel(spec: ProcessSpec) -> (RunningProcess, mpsc::Receiver<Vec<OutputChunk>>) {
    let (sender, receiver) = mpsc::channel();
    let process = spawn(spec, move |chunks| {
        let _ = sender.send(chunks);
    })
    .unwrap();
    (process, receiver)
}

/// 標準出力に `expected` が届くまで待ち、それまでに届いた標準出力を返す。
fn wait_for_stdout(
    received: &mpsc::Receiver<Vec<OutputChunk>>,
    stdout: &mut String,
    expected: &str,
) {
    let deadline = Instant::now() + WAIT;
    while !stdout.contains(expected) {
        let chunks = received
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap_or_else(|_| panic!("{expected:?} が届かない（届いた標準出力: {stdout:?}）"));
        for chunk in chunks {
            if chunk.stream == OutputStream::Stdout {
                stdout.push_str(&chunk.text);
            }
        }
    }
}

/// 残りの断片の標準出力を足す。
fn drain_stdout(received: &mpsc::Receiver<Vec<OutputChunk>>, stdout: &mut String) {
    for chunk in received.try_iter().flatten() {
        if chunk.stream == OutputStream::Stdout {
            stdout.push_str(&chunk.text);
        }
    }
}

/// ファイルができるまで待って、中身を返す。
fn wait_for_file(path: &Path) -> String {
    let deadline = Instant::now() + WAIT;
    loop {
        if let Ok(text) = std::fs::read_to_string(path) {
            return text;
        }
        assert!(Instant::now() < deadline, "{} ができない", path.display());
        thread::sleep(Duration::from_millis(20));
    }
}

/// 空白で区切った pid を読む。
fn read_pids(path: &Path) -> Vec<libc::pid_t> {
    wait_for_file(path)
        .split_whitespace()
        .map(|pid| pid.parse().unwrap())
        .collect()
}

fn is_alive(pid: libc::pid_t) -> bool {
    // SAFETY: シグナル 0 は、送らずに、プロセスがあるかどうかだけを確かめる。
    unsafe { libc::kill(pid, 0) == 0 }
}

/// プロセスが無くなるのを待つ。親が止まった子は init などに引き取られてから消えるので、少し待つ。
fn assert_terminates(pid: libc::pid_t) {
    let deadline = Instant::now() + WAIT;
    while is_alive(pid) {
        assert!(Instant::now() < deadline, "pid {pid} が止まらない");
        thread::sleep(Duration::from_millis(20));
    }
}

// ---- 出力量の上限 ----

#[test]
fn infinite_output_stops_at_the_limit_and_only_the_output_up_to_the_limit_is_delivered() {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = sh("while :; do echo abcdefghi; done", dir.path());
    spec.limits.output = OutputLimit::Total(10_000);
    let collected = collect_output(spec).unwrap();
    assert_eq!(collected.termination, Termination::OutputLimitExceeded);
    assert_eq!(collected.stdout.len(), 10_000);
    assert_eq!(collected.stdout, "abcdefghi\n".repeat(1_000));
    assert_eq!(collected.stderr, "");
}

#[test]
fn output_over_the_limit_just_before_exiting_is_returned_as_over_the_limit() {
    let dir = tempfile::tempdir().unwrap();
    // 上限を少し超える量を一度に書いてすぐ終わる。終わったことが、超えた分を読むより先に分かることがある。
    std::fs::write(dir.path().join("big"), "abcdefghi\n".repeat(1_100)).unwrap();
    for _ in 0..30 {
        let mut spec = sh("exec cat big", dir.path());
        spec.limits.output = OutputLimit::Total(10_000);
        let collected = collect_output(spec).unwrap();
        assert_eq!(collected.termination, Termination::OutputLimitExceeded);
        assert_eq!(collected.stdout, "abcdefghi\n".repeat(1_000));
    }
}

/// 起動したプロセスが終わった後のプロセスグループ。pgid 0 には、シグナルを送らない。
fn exited_group() -> Group {
    Group {
        pgid: 0,
        on_timeout: TimeoutOutput::Discard,
        state: Mutex::new(GroupState {
            stop: None,
            exited: true,
        }),
    }
}

/// 終了コード 0 で終わったことを知らせてから、`group` で終わり方を決める。
fn dispatch_exit_zero(group: &Group) -> Termination {
    let (events, received) = mpsc::sync_channel(1);
    events
        .send(Event::Exited(Ok(ExitStatus::from_raw(0))))
        .unwrap();
    drop(events);
    dispatch(received, group, 0, |_| {}).unwrap()
}

#[test]
fn going_over_the_limit_after_the_program_exited_is_over_the_limit() {
    let group = exited_group();
    group.request_stop(StopReason::OutputLimit);
    assert_eq!(dispatch_exit_zero(&group), Termination::OutputLimitExceeded);
}

#[test]
fn stopping_or_timing_out_after_the_program_exited_keeps_the_exit_code() {
    let group = exited_group();
    group.request_stop(StopReason::User);
    group.request_stop(StopReason::Timeout);
    assert_eq!(dispatch_exit_zero(&group), Termination::Exited(0));
}

#[test]
fn output_exactly_at_the_limit_is_not_over_the_limit() {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = sh(
        "i=0; while [ $i -lt 1000 ]; do echo abcdefghi; i=$((i+1)); done",
        dir.path(),
    );
    spec.limits.output = OutputLimit::Total(10_000);
    let collected = collect_output(spec).unwrap();
    assert_eq!(collected.termination, Termination::Exited(0));
    assert_eq!(collected.stdout.len(), 10_000);
}

#[test]
fn the_output_limit_counts_stdout_and_stderr_together() {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = sh("while :; do echo out; echo err >&2; done", dir.path());
    spec.limits.output = OutputLimit::Total(5_000);
    let collected = collect_output(spec).unwrap();
    assert_eq!(collected.termination, Termination::OutputLimitExceeded);
    assert_eq!(collected.stdout.len() + collected.stderr.len(), 5_000);
    assert!(!collected.stdout.is_empty() && !collected.stderr.is_empty());
}

#[test]
fn a_character_cut_at_the_output_limit_is_not_delivered() {
    let dir = tempfile::tempdir().unwrap();
    // 「あい」（UTF-8 で 1 文字 3 バイト）を出し続ける。上限は 3 の倍数ではない。
    let mut spec = sh(
        r"while :; do printf '\343\201\202\343\201\204'; done",
        dir.path(),
    );
    spec.limits.output = OutputLimit::Total(10_000);
    let collected = collect_output(spec).unwrap();
    assert_eq!(collected.termination, Termination::OutputLimitExceeded);
    assert_eq!(collected.stdout.len(), 9_999);
    assert!(!collected.stdout.contains(char::REPLACEMENT_CHARACTER));
    assert_eq!(collected.stdout, "あい".repeat(3_333 / 2) + "あ");
}

// ---- 出力量の上限（stdout 方式の判定） ----

/// 標準出力だけで数える上限で、`script` を動かす。止まって待ち続けたときに、テストが長くならないようにする。
fn sh_counting_stdout_only(script: &str, dir: &Path, limit: usize) -> ProcessSpec {
    let mut spec = sh(script, dir);
    spec.limits = Limits {
        time: Duration::from_secs(5),
        output: OutputLimit::Stdout(limit),
    };
    spec
}

/// 標準エラー出力に 1 MB（パイプに入りきらない量）を書くスクリプト。書ききれなかったら（読む側がパイプを
/// 閉じたなど）、終了コード 9 で終わる。
const MUCH_STDERR: &str = "yes e | head -c 1048576 >&2 || exit 9";

/// [`MUCH_STDERR`] のうち、渡される先頭の部分。
fn head_of_much_stderr() -> String {
    "e\n".repeat(STDERR_HEAD_BYTES / 2)
}

#[test]
fn stderr_is_not_counted_and_only_its_head_is_delivered_without_stopping() {
    let dir = tempfile::tempdir().unwrap();
    // 標準出力は上限ちょうどで、標準エラー出力は上限よりずっと多い。
    let script =
        format!("{MUCH_STDERR}; i=0; while [ $i -lt 1000 ]; do echo abcdefghi; i=$((i+1)); done");
    let started = Instant::now();
    let collected = collect_output(sh_counting_stdout_only(&script, dir.path(), 10_000)).unwrap();
    assert_eq!(collected.termination, Termination::Exited(0));
    assert_eq!(collected.stdout, "abcdefghi\n".repeat(1_000));
    assert_eq!(collected.stderr, head_of_much_stderr());
    // 先頭より後の標準エラー出力も読んで捨てるので、プログラムは書くのを待たされない。
    assert!(started.elapsed() < Duration::from_secs(3));
}

#[test]
fn stdout_over_the_limit_is_stopped_even_when_stderr_is_not_counted() {
    let dir = tempfile::tempdir().unwrap();
    let collected = collect_output(sh_counting_stdout_only(
        "echo err >&2; while :; do echo abcdefghi; done",
        dir.path(),
        10_000,
    ))
    .unwrap();
    assert_eq!(collected.termination, Termination::OutputLimitExceeded);
    assert_eq!(collected.stdout, "abcdefghi\n".repeat(1_000));
    assert_eq!(collected.stderr, "err\n");
}

#[test]
fn much_stderr_keeps_the_exit_code_and_the_head_of_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let script = format!("{MUCH_STDERR}; exit 3");
    let collected = collect_output(sh_counting_stdout_only(&script, dir.path(), 10_000)).unwrap();
    assert_eq!(collected.termination, Termination::Exited(3));
    assert_eq!(collected.stdout, "");
    assert_eq!(collected.stderr, head_of_much_stderr());
}

#[test]
fn a_character_cut_at_the_end_of_the_stderr_head_is_not_delivered() {
    let dir = tempfile::tempdir().unwrap();
    // 「あ」（UTF-8 で 3 バイト）を、先頭の量より多く書く。先頭の量は 3 の倍数ではない。
    assert_ne!(STDERR_HEAD_BYTES % 3, 0);
    let characters = STDERR_HEAD_BYTES / 3 + 100;
    let script = format!(
        r"i=0; while [ $i -lt {characters} ]; do printf '\343\201\202' >&2; i=$((i+1)); done"
    );
    let collected = collect_output(sh_counting_stdout_only(&script, dir.path(), 10_000)).unwrap();
    assert_eq!(collected.termination, Termination::Exited(0));
    assert_eq!(collected.stderr, "あ".repeat(STDERR_HEAD_BYTES / 3));
}

// ---- 止める ----

#[test]
fn stopping_kills_the_children_in_the_same_process_group() {
    let dir = tempfile::tempdir().unwrap();
    let pids = dir.path().join("pids");
    let (process, _received) = spawn_with_channel(sh(
        "sleep 60 & a=$!; sleep 60 & b=$!; echo \"$a $b\" > pids.tmp; mv pids.tmp pids; wait",
        dir.path(),
    ));
    let children = read_pids(&pids);
    assert_eq!(children.len(), 2);
    assert!(children.iter().all(|&pid| is_alive(pid)));

    process.handle().stop();
    assert_eq!(process.wait().unwrap(), Termination::Stopped);
    for pid in children {
        assert_terminates(pid);
    }
}

#[test]
fn stopping_discards_output_not_yet_delivered_even_while_the_receiver_is_slow() {
    let dir = tempfile::tempdir().unwrap();
    // 1 回目に渡されたところで、受け取る側を止めておく。その間も、プログラムは出力し続ける。
    let (started, wait_started) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let calls = Arc::new(AtomicUsize::new(0));
    let process = {
        let calls = Arc::clone(&calls);
        spawn(sh("while :; do echo x; done", dir.path()), move |_chunks| {
            if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                let _ = started.send(());
                let _ = released.recv();
            }
        })
        .unwrap()
    };
    wait_started.recv_timeout(WAIT).expect("出力が届かない");

    process.handle().stop();
    release.send(()).unwrap();
    let after_release = Instant::now();
    assert_eq!(process.wait().unwrap(), Termination::Stopped);
    assert!(after_release.elapsed() < Duration::from_secs(2));
    // 受け取る側が止まっていた間にたまった出力は、渡さない。
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

#[test]
fn the_remaining_processes_in_the_group_are_killed_when_the_program_exits() {
    let dir = tempfile::tempdir().unwrap();
    // 子の sleep は標準出力のパイプを持ったまま動き続ける。
    let started = Instant::now();
    let collected = collect_output(sh(
        "sleep 60 & echo $! > pid; echo done; exit 0",
        dir.path(),
    ))
    .unwrap();
    assert!(started.elapsed() < Duration::from_secs(5));
    assert_eq!(collected.termination, Termination::Exited(0));
    assert_eq!(collected.stdout, "done\n");
    for pid in read_pids(&dir.path().join("pid")) {
        assert_terminates(pid);
    }
}

#[test]
fn the_end_of_pipe_is_not_awaited_when_a_child_escaped_the_group_holds_the_pipe() {
    let dir = tempfile::tempdir().unwrap();
    // perl の子は、自分で別のプロセスグループに移り、標準出力のパイプを持ったまま 60 秒動く。
    let script = "perl -e 'setpgrp(0, 0); open(my $f, \">\", \"escaped\"); close($f); sleep 60' & \
                  echo $! > pid; \
                  while [ ! -f escaped ]; do sleep 0.05; done; \
                  echo done";
    let started = Instant::now();
    let collected = collect_output(sh(script, dir.path())).unwrap();
    let elapsed = started.elapsed();
    let escaped = read_pids(&dir.path().join("pid"));

    // 抜けた子は止めない（spec/exercise-runner/run.md「停止」）。テストの後始末として止める。
    let still_alive = escaped.iter().all(|&pid| is_alive(pid));
    for &pid in &escaped {
        // SAFETY: テストが起動した perl の pid にだけ送る。
        unsafe { libc::kill(pid, libc::SIGKILL) };
    }

    assert_eq!(collected.termination, Termination::Exited(0));
    assert_eq!(collected.stdout, "done\n");
    assert!(
        elapsed < Duration::from_secs(10),
        "EOF を待った: {elapsed:?}"
    );
    assert!(still_alive, "抜けた子まで止めた");
}

// ---- 制限時間 ----

#[test]
fn a_program_over_the_time_limit_is_stopped_as_timed_out() {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = sh("sleep 30", dir.path());
    spec.limits.time = Duration::from_millis(300);
    let started = Instant::now();
    let collected = collect_output(spec).unwrap();
    assert_eq!(collected.termination, Termination::TimedOut);
    assert!(started.elapsed() >= Duration::from_millis(300));
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn the_time_limit_includes_waiting_for_stdin() {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = sh("read line", dir.path());
    spec.stdin = StdinMode::Interactive;
    spec.limits.time = Duration::from_millis(300);
    let (process, _received) = spawn_with_channel(spec);
    assert_eq!(process.wait().unwrap(), Termination::TimedOut);
}

/// 制限時間で止めたときに、まだ渡していない出力を、`on_timeout` のとおりに捨てるか渡す。
/// 最初の出力を渡したところで受け取る側を止めておき、その間に 2 つ目の出力を書かせ、制限時間で止まるのを待つ。
fn output_after_a_slow_delivery_and_timeout(on_timeout: TimeoutOutput) -> String {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = sh(
        "echo $$ > pid; printf first; \
         while [ ! -e go ]; do sleep 0.01; done; \
         printf second; touch printed; sleep 30",
        dir.path(),
    );
    spec.limits.time = Duration::from_secs(2);
    spec.on_timeout = on_timeout;
    let (started, wait_started) = mpsc::channel();
    let (release, released) = mpsc::channel::<()>();
    let delivered = Arc::new(Mutex::new(String::new()));
    let process = {
        let delivered = Arc::clone(&delivered);
        spawn(spec, move |chunks| {
            let first = {
                let mut delivered = lock(&delivered);
                let first = delivered.is_empty();
                for chunk in chunks {
                    delivered.push_str(&chunk.text);
                }
                first
            };
            if first {
                let _ = started.send(());
                let _ = released.recv();
            }
        })
        .unwrap()
    };
    wait_started.recv_timeout(WAIT).expect("出力が届かない");
    assert_eq!(*lock(&delivered), "first");

    std::fs::write(dir.path().join("go"), "").unwrap();
    wait_for_file(&dir.path().join("printed"));
    // 制限時間で止まるまで、受け取る側を止めたままにする。
    for pid in read_pids(&dir.path().join("pid")) {
        assert_terminates(pid);
    }
    release.send(()).unwrap();
    assert_eq!(process.wait().unwrap(), Termination::TimedOut);
    std::mem::take(&mut *lock(&delivered))
}

#[test]
fn output_not_yet_delivered_is_discarded_on_timeout_by_default() {
    assert_eq!(
        output_after_a_slow_delivery_and_timeout(TimeoutOutput::Discard),
        "first"
    );
}

#[test]
fn output_not_yet_delivered_is_delivered_on_timeout_when_asked() {
    assert_eq!(
        output_after_a_slow_delivery_and_timeout(TimeoutOutput::Deliver),
        "firstsecond"
    );
}

// ---- 標準入力 ----

#[test]
fn lines_written_to_stdin_are_received_and_closing_gives_eof() {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = sh(
        "while read line; do echo \"got:$line\"; done; echo eof",
        dir.path(),
    );
    spec.stdin = StdinMode::Interactive;
    let (process, received) = spawn_with_channel(spec);
    let handle = process.handle();
    let mut stdout = String::new();

    handle.write_stdin_line("a").unwrap();
    wait_for_stdout(&received, &mut stdout, "got:a\n");
    handle.write_stdin_line("日本語").unwrap();
    wait_for_stdout(&received, &mut stdout, "got:日本語\n");
    handle.close_stdin();
    assert_eq!(handle.write_stdin_line("b"), Err(StdinClosed));

    assert_eq!(process.wait().unwrap(), Termination::Exited(0));
    drain_stdout(&received, &mut stdout);
    assert_eq!(stdout, "got:a\ngot:日本語\neof\n");
}

#[test]
fn stdin_closed_right_after_start_gives_eof() {
    let dir = tempfile::tempdir().unwrap();
    let collected = collect_output(sh(
        "if read line; then echo \"got:$line\"; else echo eof; fi",
        dir.path(),
    ))
    .unwrap();
    assert_eq!(collected.termination, Termination::Exited(0));
    assert_eq!(collected.stdout, "eof\n");
}

#[test]
fn writing_to_stdin_that_is_not_interactive_fails() {
    let dir = tempfile::tempdir().unwrap();
    let (process, _received) = spawn_with_channel(sh("sleep 30", dir.path()));
    let handle = process.handle();
    assert_eq!(handle.write_stdin_line("a"), Err(StdinClosed));
    handle.stop();
    assert_eq!(process.wait().unwrap(), Termination::Stopped);
}

#[test]
fn the_given_input_is_written_to_stdin_and_closed() {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = sh("cat; echo err >&2", dir.path());
    spec.stdin = StdinMode::Input(b"1\n2\n".to_vec());
    let collected = collect_output(spec).unwrap();
    assert_eq!(
        collected,
        CollectedOutput {
            termination: Termination::Exited(0),
            stdout: "1\n2\n".into(),
            stderr: "err\n".into(),
        }
    );
}

// ---- 出力の送り方 ----

#[test]
fn output_without_a_trailing_newline_is_delivered_before_the_program_ends() {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = sh("printf 'name? '; read name; echo \"hi $name\"", dir.path());
    spec.stdin = StdinMode::Interactive;
    let (process, received) = spawn_with_channel(spec);
    let mut stdout = String::new();

    // プログラムは標準入力を待っているので、まだ終わっていない。
    wait_for_stdout(&received, &mut stdout, "name? ");
    process.handle().write_stdin_line("bob").unwrap();

    assert_eq!(process.wait().unwrap(), Termination::Exited(0));
    drain_stdout(&received, &mut stdout);
    assert_eq!(stdout, "name? hi bob\n");
}

#[test]
fn stdout_and_stderr_are_delivered_as_separate_kinds() {
    let dir = tempfile::tempdir().unwrap();
    let (process, received) =
        spawn_with_channel(sh("echo out; sleep 0.2; echo err >&2", dir.path()));
    assert_eq!(process.wait().unwrap(), Termination::Exited(0));
    let chunks: Vec<OutputChunk> = received.try_iter().flatten().collect();
    assert_eq!(
        chunks,
        [
            OutputChunk {
                stream: OutputStream::Stdout,
                text: "out\n".into(),
            },
            OutputChunk {
                stream: OutputStream::Stderr,
                text: "err\n".into(),
            },
        ]
    );
}

#[test]
fn many_small_writes_are_delivered_in_a_few_batches() {
    let dir = tempfile::tempdir().unwrap();
    let (process, received) = spawn_with_channel(sh(
        "i=0; while [ $i -lt 2000 ]; do echo line; i=$((i+1)); done",
        dir.path(),
    ));
    assert_eq!(process.wait().unwrap(), Termination::Exited(0));
    let batches: Vec<Vec<OutputChunk>> = received.try_iter().collect();
    let stdout: String = batches
        .iter()
        .flatten()
        .map(|chunk| chunk.text.as_str())
        .collect();
    assert_eq!(stdout, "line\n".repeat(2000));
    assert!(batches.len() < 200, "{} 回に分けて渡した", batches.len());
    // 同じ種類の断片が続く分は、1 つにまとめる。
    assert!(batches.iter().all(|batch| batch.len() == 1));
}

// ---- 終わり方 ----

#[test]
fn the_exit_code_is_returned() {
    let dir = tempfile::tempdir().unwrap();
    let collected = collect_output(sh("exit 3", dir.path())).unwrap();
    assert_eq!(collected.termination, Termination::Exited(3));
}

#[test]
fn the_signal_is_returned() {
    let dir = tempfile::tempdir().unwrap();
    let collected = collect_output(sh("kill -ABRT $$", dir.path())).unwrap();
    assert_eq!(collected.termination, Termination::Signaled(libc::SIGABRT));
}

#[test]
fn the_working_directory_arguments_and_environment_variables_are_given() {
    let dir = tempfile::tempdir().unwrap();
    let mut spec = sh("pwd -P; echo \"$0 $1 $PROGRUST_TEST\"", dir.path());
    spec.args.extend(["name".into(), "first".into()]);
    spec.envs = vec![("PROGRUST_TEST".into(), "value".into())];
    let collected = collect_output(spec).unwrap();
    let expected_dir = dir.path().canonicalize().unwrap();
    assert_eq!(
        collected.stdout,
        format!("{}\nname first value\n", expected_dir.display())
    );
}

#[test]
fn a_program_that_cannot_be_started_is_an_error() {
    let dir = tempfile::tempdir().unwrap();
    let spec = ProcessSpec::new(dir.path().join("missing"), dir.path(), LIMITS);
    assert!(spawn(spec, |_| {}).is_err());
}

// ---- まとめて止める操作 ----

#[test]
fn a_stopper_stops_all_running_processes_and_starts_no_more() {
    let dir = tempfile::tempdir().unwrap();
    let stopper = Stopper::new();
    let started = Instant::now();
    let workers: Vec<_> = (0..2)
        .map(|_| {
            let stopper = stopper.clone();
            let spec = sh("sleep 30", dir.path());
            std::thread::spawn(move || stopper.collect_output(spec).unwrap())
        })
        .collect();
    // 2 つとも起動して登録されるまで待つ。
    let deadline = Instant::now() + WAIT;
    while lock(&stopper.state).running.len() < 2 {
        assert!(Instant::now() < deadline, "プロセスが登録されない");
        std::thread::sleep(Duration::from_millis(10));
    }

    stopper.stop();
    for worker in workers {
        assert_eq!(worker.join().unwrap().termination, Termination::Stopped);
    }
    assert!(started.elapsed() < WAIT);
    assert!(stopper.is_stopped());
    assert!(lock(&stopper.state).running.is_empty());

    // 止めた後は、起動しない。
    let marker = dir.path().join("started");
    let collected = stopper
        .collect_output(sh(&format!("touch '{}'", marker.display()), dir.path()))
        .unwrap();
    assert_eq!(collected.termination, Termination::Stopped);
    assert!(!marker.exists());
}

#[test]
fn processes_that_finished_are_no_longer_registered() {
    let dir = tempfile::tempdir().unwrap();
    let stopper = Stopper::new();
    let collected = stopper.collect_output(sh("echo done", dir.path())).unwrap();
    assert_eq!(collected.termination, Termination::Exited(0));
    assert_eq!(collected.stdout, "done\n");
    assert!(lock(&stopper.state).running.is_empty());
    assert!(!stopper.is_stopped());
}

// ---- UTF-8 ----

#[test]
fn bytes_cut_in_the_middle_of_a_character_are_carried_over() {
    let mut decoder = Utf8Decoder::default();
    let bytes = "あい".as_bytes();
    assert_eq!(decoder.decode(&bytes[..1]), "");
    assert_eq!(decoder.decode(&bytes[1..4]), "あ");
    assert_eq!(decoder.decode(&bytes[4..]), "い");
    assert_eq!(decoder.finish(), "");
}

#[test]
fn invalid_bytes_become_replacement_characters() {
    let mut decoder = Utf8Decoder::default();
    assert_eq!(decoder.decode(b"a\xffb"), "a\u{fffd}b");
    assert_eq!(decoder.decode(&"あ".as_bytes()[..2]), "");
    assert_eq!(decoder.finish(), "\u{fffd}");
}
