//! C の演習の実行の手順（spec/exercise-runner/c-run.md「手順」「シグナルで終わったとき」「メモリの検査をしないこと」、
//! spec/exercise-runner/attempt-log.md「C の演習」）。
//!
//! - [`run_steps`] は、実行のタスク（[`super::run::execute`]）が、C の演習（[`RunToolchain::Clang`]）のときに呼ぶ。
//!   `c-project/` を組み立て（[`super::c_project`]）、検査なしでビルドし（[`super::c_build`]）、`c-project/main` を
//!   `c-project/` を作業ディレクトリとして動かす。プログラムの動かし方（出力の送り方・停止・対話的な標準入力）は、
//!   Rust の演習と同じ部品（[`super::run::run_program`]）を使う。
//! - メモリの検査はしない。検査なしのビルドの実行ファイルを、そのまま動かす。
//! - 学習者のプログラムが、自分の受けたシグナルで終わったときは、出力を送り終えた後に、シグナルの名前と意味の 1 行
//!   （[`signal_message`]）を `run_output` の `system` で送る。アプリが止めたとき（停止ボタン、実行時間の制限、
//!   出力量の上限、演習画面を離れたとき）は、終わり方がシグナルにならないので送らない。
//! - 試行ログには、`clang` の出力をビルドの出力として渡す。`error_codes` は空にし、シグナルの 1 行は入れない。
//!
//! [`RunToolchain::Clang`]: super::run::RunToolchain::Clang

use std::path::Path;
use std::sync::Arc;

use super::build::BuildStatus;
use super::c_build::{CBuildKind, CBuildOutcome, CBuildOutputKind, CBuildSpec, start_c_build};
use super::c_project::assemble_c_project;
use super::process::{ProcessSpec, StdinMode, Termination, TimeoutOutput};
use super::run::{
    Outcome, RunEvents, RunJob, RunOutputChunk, RunResultStatus, RunStream, app_failure,
    process_limits, program_end, run_program, send, send_notice,
};
use crate::attempts::BuildOutput;
use crate::commands::error::CommandError;

/// `c-project/` の組み立てから、プログラムの終わりまで。アプリの側の失敗は `Err` で返す。
pub(super) fn run_steps(
    job: &RunJob,
    clang: &Path,
    events: &Arc<dyn RunEvents>,
) -> Result<Outcome, CommandError> {
    let limits = job.runs.limits();
    // ファイルパスは run_code が確かめている。ここで満たさないときも invalid_argument にする。
    assemble_c_project(&job.workspace, &job.files)?;

    let project_dir = job.workspace.c_project();
    let mut build_spec = CBuildSpec::new(clang, &project_dir, CBuildKind::Unchecked);
    build_spec.time_limit = limits.build_time;
    let running = start_c_build(build_spec)
        .map_err(|error| app_failure("clang を起動できませんでした。", error))?;
    job.active.set_building(running.handle());
    let built = running
        .wait()
        .map_err(|error| app_failure("ビルドの結果を受け取れませんでした。", error))?;
    let build = attempt_build_output(&built);

    match built.status {
        // 止める操作で止めたときは、ビルドの出力を捨てている（run.md「停止」）。
        BuildStatus::Stopped => {
            return Ok(without_program(RunResultStatus::Stopped, build));
        }
        BuildStatus::Failed { .. } | BuildStatus::TimedOut => {
            send(job, events, build_output_chunks(&built));
            return Ok(without_program(RunResultStatus::CompileError, build));
        }
        // ビルドに成功したときも、ビルドの出力（コンパイラの警告など）を表示する。
        BuildStatus::Succeeded => send(job, events, build_output_chunks(&built)),
    }

    let mut spec = ProcessSpec::new(
        project_dir.join(CBuildKind::Unchecked.output_file()),
        &project_dir,
        process_limits(&limits),
    );
    spec.stdin = StdinMode::Interactive;
    spec.on_timeout = TimeoutOutput::Discard;
    // 環境変数は足さず、アプリが受け取ったものをそのまま渡す（`claude` のように絞らない）。

    // ビルドが終わってからプログラムを起動するまでに止める指示が来ていたら、起動しない。
    if job.active.stop_cause().is_some() {
        return Ok(without_program(RunResultStatus::Stopped, build));
    }

    let program = run_program(job, events, spec)?;
    let mut ended = program_end(&program.termination, &limits);
    // アプリが止めたときは、終わり方が Stopped・TimedOut・OutputLimitExceeded になり、ここには来ない。
    if let Termination::Signaled(signal) = program.termination {
        ended.notice = Some(signal_message(signal));
    }
    send_notice(job, events, ended.notice);
    Ok(Outcome {
        status: ended.status,
        exit_code: ended.exit_code,
        error_codes: Vec::new(),
        build,
        stdout: program.stdout,
        program_stderr: program.stderr,
    })
}

/// プログラムを起動していない結果。
fn without_program(status: RunResultStatus, build: BuildOutput) -> Outcome {
    Outcome {
        status,
        exit_code: None,
        error_codes: Vec::new(),
        build,
        stdout: String::new(),
        program_stderr: String::new(),
    }
}

/// 試行ログに渡すビルドの出力。`clang` の出力（標準出力と標準エラー出力を受け取った順に連結したもの）を、
/// コンパイラの診断として渡す。ANSI エスケープは、試行ログに書くときに除く。アプリの知らせ（タイムアウトの旨）は
/// 入れない。
fn attempt_build_output(built: &CBuildOutcome) -> BuildOutput {
    BuildOutput {
        compiler_messages: built.clang_output(),
        cargo_stderr: String::new(),
    }
}

/// ビルドの出力を、受け取った順に `run_output` の断片にする。`clang` の出力は `stderr`、アプリの知らせ
/// （タイムアウトの旨）は `system` にする（spec/architecture/commands/run.md「イベント」）。
fn build_output_chunks(built: &CBuildOutcome) -> Vec<RunOutputChunk> {
    let mut chunks: Vec<RunOutputChunk> = Vec::new();
    for piece in &built.output {
        let stream = match piece.kind {
            CBuildOutputKind::Clang => RunStream::Stderr,
            CBuildOutputKind::System => RunStream::System,
        };
        match chunks.last_mut() {
            Some(last) if last.stream == stream => last.text.push_str(&piece.text),
            _ => chunks.push(RunOutputChunk {
                stream,
                text: piece.text.clone(),
            }),
        }
    }
    chunks
}

/// 学習者のプログラムがシグナル `signal` で終わったときに表示する 1 行（c-run.md「シグナルで終わったとき」）。
pub fn signal_message(signal: i32) -> String {
    let meaning = match signal {
        libc::SIGSEGV => Some("不正なメモリアクセス"),
        libc::SIGBUS => Some("不正なアドレスへのアクセス"),
        libc::SIGABRT => Some("異常終了。abort の呼び出しなど"),
        libc::SIGFPE => Some("算術演算の例外。整数の 0 での除算など"),
        libc::SIGILL => Some("不正な命令"),
        _ => None,
    };
    // 名前の分からないシグナルは、名前の代わりに番号を表示する。
    let name = signal_name(signal).map_or_else(|| signal.to_string(), str::to_string);
    match meaning {
        Some(meaning) => format!("シグナル {name} で終了しました（{meaning}）"),
        None => format!("シグナル {name} で終了しました"),
    }
}

/// シグナルの名前。番号は OS で違うので、`libc` の定数で比べる。分からないときは `None`。
fn signal_name(signal: i32) -> Option<&'static str> {
    let names: &[(i32, &str)] = &[
        (libc::SIGHUP, "SIGHUP"),
        (libc::SIGINT, "SIGINT"),
        (libc::SIGQUIT, "SIGQUIT"),
        (libc::SIGILL, "SIGILL"),
        (libc::SIGTRAP, "SIGTRAP"),
        (libc::SIGABRT, "SIGABRT"),
        #[cfg(target_os = "macos")]
        (libc::SIGEMT, "SIGEMT"),
        (libc::SIGFPE, "SIGFPE"),
        (libc::SIGKILL, "SIGKILL"),
        (libc::SIGBUS, "SIGBUS"),
        (libc::SIGSEGV, "SIGSEGV"),
        (libc::SIGSYS, "SIGSYS"),
        (libc::SIGPIPE, "SIGPIPE"),
        (libc::SIGALRM, "SIGALRM"),
        (libc::SIGTERM, "SIGTERM"),
        (libc::SIGURG, "SIGURG"),
        (libc::SIGSTOP, "SIGSTOP"),
        (libc::SIGTSTP, "SIGTSTP"),
        (libc::SIGCONT, "SIGCONT"),
        (libc::SIGCHLD, "SIGCHLD"),
        (libc::SIGTTIN, "SIGTTIN"),
        (libc::SIGTTOU, "SIGTTOU"),
        (libc::SIGIO, "SIGIO"),
        (libc::SIGXCPU, "SIGXCPU"),
        (libc::SIGXFSZ, "SIGXFSZ"),
        (libc::SIGVTALRM, "SIGVTALRM"),
        (libc::SIGPROF, "SIGPROF"),
        (libc::SIGWINCH, "SIGWINCH"),
        #[cfg(target_os = "macos")]
        (libc::SIGINFO, "SIGINFO"),
        (libc::SIGUSR1, "SIGUSR1"),
        (libc::SIGUSR2, "SIGUSR2"),
    ];
    names
        .iter()
        .find(|(number, _)| *number == signal)
        .map(|(_, name)| *name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runner::build::BuildStatus;
    use crate::runner::c_build::CBuildOutputPiece;

    #[test]
    fn the_signal_lines_follow_the_spec() {
        for (signal, line) in [
            (
                libc::SIGSEGV,
                "シグナル SIGSEGV で終了しました（不正なメモリアクセス）",
            ),
            (
                libc::SIGBUS,
                "シグナル SIGBUS で終了しました（不正なアドレスへのアクセス）",
            ),
            (
                libc::SIGABRT,
                "シグナル SIGABRT で終了しました（異常終了。abort の呼び出しなど）",
            ),
            (
                libc::SIGFPE,
                "シグナル SIGFPE で終了しました（算術演算の例外。整数の 0 での除算など）",
            ),
            (libc::SIGILL, "シグナル SIGILL で終了しました（不正な命令）"),
            (libc::SIGTERM, "シグナル SIGTERM で終了しました"),
            (libc::SIGKILL, "シグナル SIGKILL で終了しました"),
            (libc::SIGUSR1, "シグナル SIGUSR1 で終了しました"),
        ] {
            assert_eq!(signal_message(signal), line, "{signal}");
        }
    }

    #[test]
    fn signals_without_a_known_name_show_the_number() {
        assert_eq!(signal_message(99), "シグナル 99 で終了しました");
    }

    #[test]
    fn clang_output_is_stderr_and_notices_are_system() {
        let built = CBuildOutcome {
            status: BuildStatus::TimedOut,
            output: vec![
                CBuildOutputPiece {
                    kind: CBuildOutputKind::Clang,
                    text: "\u{1b}[1mmain.c:1:1: \u{1b}[0mwarning: a\n".into(),
                },
                CBuildOutputPiece {
                    kind: CBuildOutputKind::System,
                    text: "ビルド時間の制限を超えた".into(),
                },
            ],
        };
        assert_eq!(
            build_output_chunks(&built),
            [
                RunOutputChunk {
                    stream: RunStream::Stderr,
                    text: "\u{1b}[1mmain.c:1:1: \u{1b}[0mwarning: a\n".into(),
                },
                RunOutputChunk {
                    stream: RunStream::System,
                    text: "ビルド時間の制限を超えた".into(),
                },
            ]
        );
        // 試行ログには、clang の出力だけを、コンパイラの診断として渡す。
        assert_eq!(
            attempt_build_output(&built),
            BuildOutput {
                compiler_messages: "\u{1b}[1mmain.c:1:1: \u{1b}[0mwarning: a\n".into(),
                cargo_stderr: String::new(),
            }
        );
    }
}
