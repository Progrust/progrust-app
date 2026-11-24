//! 実行・提出を同時に 1 つだけ行う状態のテスト。プログラムは `sh` で動かす。

use std::path::Path;
use std::sync::mpsc;
use std::time::Duration;

use super::*;
use crate::runner::process::{
    self, Limits, OutputLimit, OutputStream, ProcessSpec, RunningProcess, StdinMode, Termination,
};

fn sh(script: &str, dir: &Path, stdin: StdinMode) -> ProcessSpec {
    let mut spec = ProcessSpec::new(
        "/bin/sh",
        dir,
        Limits {
            time: Duration::from_secs(30),
            output: OutputLimit::Total(1024 * 1024),
        },
    );
    spec.args = vec!["-c".into(), script.into()];
    spec.stdin = stdin;
    spec
}

/// 起動し、標準出力を受け取る通り道を返す。
fn spawn(spec: ProcessSpec) -> (RunningProcess, mpsc::Receiver<String>) {
    let (sender, receiver) = mpsc::channel();
    let process = process::spawn(spec, move |chunks| {
        for chunk in chunks {
            if chunk.stream == OutputStream::Stdout {
                let _ = sender.send(chunk.text);
            }
        }
    })
    .unwrap();
    (process, receiver)
}

/// 受け取った標準出力に `expected` が現れるまで待つ。
fn wait_for(receiver: &mpsc::Receiver<String>, stdout: &mut String, expected: &str) {
    while !stdout.contains(expected) {
        let text = receiver
            .recv_timeout(Duration::from_secs(10))
            .unwrap_or_else(|_| panic!("{expected:?} が届かない（{stdout:?}）"));
        stdout.push_str(&text);
    }
}

fn code(result: Result<(), CommandError>) -> ErrorCode {
    result.unwrap_err().code
}

#[test]
fn only_one_request_runs_at_a_time() {
    let runs = Runs::default();
    let first = runs.begin("a", "ch01-01", RunKind::Run).unwrap();

    let same = runs.begin("a", "ch01-01", RunKind::Run).err().unwrap();
    assert_eq!(same.code, ErrorCode::InvalidArgument);
    let other = runs.begin("b", "ch01-02", RunKind::Submit).err().unwrap();
    assert_eq!(other.code, ErrorCode::Busy);
    assert_eq!(
        other.message,
        "ほかの実行・提出が終わるまで待ってください。"
    );

    assert!(runs.find("a").is_some());
    assert!(runs.find("b").is_none());
    runs.release(&first);
    assert!(runs.find("a").is_none());
    // 終わった後は、同じ ID でも、別の ID でも始められる。
    let second = runs.begin("b", "ch01-02", RunKind::Submit).unwrap();
    assert_eq!(second.kind(), RunKind::Submit);
    // 外した要求をもう一度外しても、動いている要求は外れない。
    runs.release(&first);
    assert!(runs.find("b").is_some());
}

#[test]
fn stdin_cannot_be_used_before_the_program_starts() {
    let runs = Runs::default();
    let run = runs.begin("a", "l", RunKind::Run).unwrap();
    assert_eq!(code(run.write_stdin("x\n")), ErrorCode::InvalidState);
    assert_eq!(code(run.close_stdin()), ErrorCode::InvalidState);
}

#[test]
fn stdin_is_written_as_is_and_recorded_until_closed() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::default();
    let run = runs.begin("a", "l", RunKind::Run).unwrap();
    let (process, received) = spawn(sh(
        "while read line; do echo \"got:$line\"; done; echo eof",
        dir.path(),
        StdinMode::Interactive,
    ));
    run.set_running(process.handle(), StdinState::Open);
    let mut stdout = String::new();

    run.write_stdin("a\n").unwrap();
    wait_for(&received, &mut stdout, "got:a\n");
    run.write_stdin("日本語\n").unwrap();
    wait_for(&received, &mut stdout, "got:日本語\n");
    run.close_stdin().unwrap();
    wait_for(&received, &mut stdout, "eof\n");
    // 改行を足さずに書く（入力欄の 1 行に改行を付けたものが届く）。
    assert_eq!(stdout, "got:a\ngot:日本語\neof\n");

    assert_eq!(code(run.write_stdin("b\n")), ErrorCode::InvalidState);
    assert_eq!(code(run.close_stdin()), ErrorCode::InvalidState);
    assert_eq!(run.stdin_log(), "a\n日本語\n");

    assert_eq!(process.wait().unwrap(), Termination::Exited(0));
    run.set_finished();
    assert_eq!(code(run.write_stdin("c\n")), ErrorCode::InvalidState);
}

#[test]
fn stdin_of_a_program_started_with_closed_stdin_is_invalid_state() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::default();
    let run = runs.begin("a", "l", RunKind::Run).unwrap();
    let (process, _received) = spawn(sh("sleep 30", dir.path(), StdinMode::Closed));
    run.set_running(process.handle(), StdinState::Closed);
    assert_eq!(code(run.write_stdin("x\n")), ErrorCode::InvalidState);
    assert_eq!(code(run.close_stdin()), ErrorCode::InvalidState);
    runs.stop("a");
    assert_eq!(process.wait().unwrap(), Termination::Stopped);
}

#[test]
fn a_stop_before_the_program_starts_stops_it_when_it_starts() {
    let dir = tempfile::tempdir().unwrap();
    let runs = Runs::default();
    let run = runs.begin("a", "l", RunKind::Run).unwrap();
    runs.stop("a");
    assert_eq!(run.stop_cause(), Some(StopCause::User));

    let (process, _received) = spawn(sh("sleep 30", dir.path(), StdinMode::Closed));
    run.set_running(process.handle(), StdinState::Closed);
    assert_eq!(process.wait().unwrap(), Termination::Stopped);
}

#[test]
fn stopping_an_id_that_is_not_running_does_nothing() {
    let runs = Runs::default();
    let run = runs.begin("a", "l", RunKind::Run).unwrap();
    runs.stop("other");
    assert_eq!(run.stop_cause(), None);
    runs.release(&run);
    runs.stop("a");
    assert_eq!(run.stop_cause(), None);
}

#[test]
fn app_exit_overrides_a_user_stop() {
    let runs = Runs::default();
    let run = runs.begin("a", "l", RunKind::Run).unwrap();
    runs.stop("a");
    assert!(!run.stopped_for_app_exit());
    runs.stop_for_app_exit();
    assert!(run.stopped_for_app_exit());
    // 後から止める操作が来ても、アプリの終了のまま。
    runs.stop("a");
    assert!(run.stopped_for_app_exit());
}

#[test]
fn stopping_a_submit_while_judging_stops_the_judge_processes() {
    let runs = Runs::default();
    let submit = runs.begin("s", "l", RunKind::Submit).unwrap();
    let stopper = Stopper::new();
    submit.set_judging(stopper.clone());
    assert!(!stopper.is_stopped());
    runs.stop("s");
    assert!(stopper.is_stopped());
    assert_eq!(submit.stop_cause(), Some(StopCause::User));
}

#[test]
fn a_stop_before_judging_stops_the_judge_when_it_starts() {
    let runs = Runs::default();
    let submit = runs.begin("s", "l", RunKind::Submit).unwrap();
    runs.stop_for_app_exit();
    let stopper = Stopper::new();
    submit.set_judging(stopper.clone());
    assert!(stopper.is_stopped());
    assert!(submit.stopped_for_app_exit());
}

#[test]
fn a_submit_does_not_take_stdin() {
    let runs = Runs::default();
    let submit = runs.begin("s", "l", RunKind::Submit).unwrap();
    assert_eq!(code(submit.write_stdin("x\n")), ErrorCode::InvalidState);
    submit.set_judging(Stopper::new());
    assert_eq!(code(submit.write_stdin("x\n")), ErrorCode::InvalidState);
    assert_eq!(code(submit.close_stdin()), ErrorCode::InvalidState);
}

/// 別のスレッドで [`Runs::stop_lesson_run_and_wait`] を呼び、返ったら知らせる通り道を返す。
fn stop_and_wait_in_thread(runs: &Runs, lesson_id: &str) -> mpsc::Receiver<()> {
    let (sender, receiver) = mpsc::channel();
    let runs = runs.clone();
    let lesson_id = lesson_id.to_string();
    std::thread::spawn(move || {
        runs.stop_lesson_run_and_wait(&lesson_id);
        let _ = sender.send(());
    });
    receiver
}

#[test]
fn stop_lesson_run_and_wait_stops_the_run_and_returns_after_it_is_released() {
    let runs = Runs::default();
    let run = runs.begin("a", "l", RunKind::Run).unwrap();
    let returned = stop_and_wait_in_thread(&runs, "l");

    // 止める指示を出し、要求が外れるまで返らない。
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while run.stop_cause().is_none() {
        assert!(std::time::Instant::now() < deadline, "止める指示が来ない");
        std::thread::sleep(Duration::from_millis(10));
    }
    assert_eq!(run.stop_cause(), Some(StopCause::User));
    assert!(
        returned.recv_timeout(Duration::from_millis(300)).is_err(),
        "要求が外れる前に返った"
    );

    runs.release(&run);
    returned
        .recv_timeout(Duration::from_secs(10))
        .expect("要求が外れても返らない");
}

#[test]
fn stop_lesson_run_and_wait_after_stop_run_still_waits_for_the_release() {
    let runs = Runs::default();
    let run = runs.begin("a", "l", RunKind::Run).unwrap();
    runs.stop("a");
    let returned = stop_and_wait_in_thread(&runs, "l");
    assert!(
        returned.recv_timeout(Duration::from_millis(300)).is_err(),
        "要求が外れる前に返った"
    );
    runs.release(&run);
    returned
        .recv_timeout(Duration::from_secs(10))
        .expect("要求が外れても返らない");
}

#[test]
fn stop_lesson_run_and_wait_ignores_other_lessons_submits_and_no_run() {
    let runs = Runs::default();
    // 動いていないときは、すぐ返る。
    runs.stop_lesson_run_and_wait("l");

    // ほかのレッスンの実行は止めない。
    let other = runs.begin("a", "other", RunKind::Run).unwrap();
    runs.stop_lesson_run_and_wait("l");
    assert_eq!(other.stop_cause(), None);
    runs.release(&other);

    // 提出は止めない（止めた提出は試行ログに追記しない）。
    let submit = runs.begin("s", "l", RunKind::Submit).unwrap();
    runs.stop_lesson_run_and_wait("l");
    assert_eq!(submit.stop_cause(), None);
    assert!(runs.find("s").is_some());
}
