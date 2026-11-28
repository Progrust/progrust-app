//! 組み立てた `c-project/` を `clang` でビルドする部品（spec/exercise-runner/c-project-assembly.md「ビルド」
//! 「ビルドの出力の表示」、spec/exercise-runner/limits.md「C の演習」、spec/exercise-runner/run.md「停止」）。
//!
//! - ビルドは、検査なし・検査つき・メモリリークの検査のライブラリの 3 種類（[`CBuildKind`]）。コマンドと
//!   オプションは、仕様の表のとおりにする。
//! - [`start_c_build`] で、検出した `clang` を、`c-project/` を作業ディレクトリとして、子プロセスの部品
//!   （[`super::process`]）で新しいプロセスグループに起動する。止める操作は [`CBuildHandle`] で行い、結果は
//!   [`RunningCBuild::wait`] で受け取る。
//! - ビルド時間の制限は、`clang` の起動 1 回ごとに当てる。超えたときと止めるときは、プロセスグループ全体に
//!   SIGKILL を送る。
//! - ビルドの出力（`clang` の標準出力と標準エラー出力）は、ビルドが終わってから、受け取った順に
//!   [`CBuildOutcome::output`] で返す。送るかどうかは呼び出し側が決める（run.md「ビルドの出力の表示」）。
//! - 終了コード 0 を成功とする。
//! - `clang` には、アプリが受け取った環境変数をそのまま渡す。`claude` のように 6 つに絞らない
//!   （spec/claude-integration/invocation.md「環境変数」）。
//!
//! Tauri の実行時に頼らない（教材チェックツールの判定からも使う。ADR 0273）。

use std::{
    ffi::OsString,
    io,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use super::build::{BUILD_TIME_LIMIT, BuildStatus, timeout_message};
use super::process::{
    self, Limits, OutputChunk, OutputLimit, ProcessHandle, ProcessSpec, RunningProcess, StdinMode,
    Termination, TimeoutOutput,
};

/// 検査なし・検査つきのビルドに共通のオプション。
const COMMON_FLAGS: [&str; 6] = [
    "-std=c17",
    "-g",
    "-O0",
    "-Wall",
    "-Wextra",
    "-fcolor-diagnostics",
];
/// 検査つきのビルドに足す、メモリの検査のオプション。
const CHECK_FLAGS: [&str; 3] = [
    "-fsanitize=address,undefined",
    "-fno-sanitize-recover=all",
    "-fno-omit-frame-pointer",
];

/// ビルドの種類（c-project-assembly.md「ビルド」の表）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CBuildKind {
    /// 検査なし（`main`）。実行と、提出のメモリリークの検査で使う。
    Unchecked,
    /// 検査つき（`main-checked`）。提出のテストケースの実行で使う。
    Checked,
    /// メモリリークの検査のライブラリ（`leakcheck/libprogrust_leakcheck.dylib`）。提出のメモリリークの検査で使う。
    LeakCheckLibrary,
}

impl CBuildKind {
    /// `clang` に渡す引数。パスは `c-project/` からの相対パスで書く。
    pub fn args(self) -> Vec<&'static str> {
        match self {
            Self::Unchecked => [&COMMON_FLAGS[..], &["-o", "main", "main.c"]].concat(),
            Self::Checked => [
                &COMMON_FLAGS[..],
                &CHECK_FLAGS[..],
                &["-o", "main-checked", "main.c"],
            ]
            .concat(),
            Self::LeakCheckLibrary => vec![
                "-dynamiclib",
                "-O2",
                "-o",
                "leakcheck/libprogrust_leakcheck.dylib",
                "leakcheck/leakcheck.c",
            ],
        }
    }

    /// ビルドでできるもの（`c-project/` からの相対パス）。
    pub fn output_file(self) -> &'static str {
        match self {
            Self::Unchecked => "main",
            Self::Checked => "main-checked",
            Self::LeakCheckLibrary => "leakcheck/libprogrust_leakcheck.dylib",
        }
    }
}

/// ビルドするもの。
#[derive(Debug, Clone)]
pub struct CBuildSpec {
    /// 検出した `clang` の絶対パス（[`crate::environment::ClangTool`]）。
    pub clang: PathBuf,
    /// `c-project/`。`clang` の作業ディレクトリにする。
    pub project_dir: PathBuf,
    pub kind: CBuildKind,
    /// ビルド時間の制限。この `clang` の起動 1 回に当てる。
    pub time_limit: Duration,
}

impl CBuildSpec {
    /// ビルド時間の制限を [`BUILD_TIME_LIMIT`] にする。
    pub fn new(
        clang: impl Into<PathBuf>,
        project_dir: impl Into<PathBuf>,
        kind: CBuildKind,
    ) -> Self {
        Self {
            clang: clang.into(),
            project_dir: project_dir.into(),
            kind,
            time_limit: BUILD_TIME_LIMIT,
        }
    }
}

/// ビルドの出力の断片の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CBuildOutputKind {
    /// `clang` の標準出力か標準エラー出力（コンパイラの診断）。ANSI エスケープを含む。
    Clang,
    /// アプリが出す知らせ（タイムアウトの旨）。
    System,
}

/// ビルドの出力の断片。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CBuildOutputPiece {
    pub kind: CBuildOutputKind,
    pub text: String,
}

/// ビルドの結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CBuildOutcome {
    /// 終わり方。`clang` が終了コード 0 で終わったときだけ [`BuildStatus::Succeeded`]。
    pub status: BuildStatus,
    /// ビルドの出力。`clang` の標準出力と標準エラー出力を、受け取った順に並べる。止める操作で止めたときは空。
    /// ビルド時間の制限で止めたときは、最後に [`CBuildOutputKind::System`] のタイムアウトの旨が付く。
    pub output: Vec<CBuildOutputPiece>,
}

impl CBuildOutcome {
    /// `clang` の出力（標準出力と標準エラー出力）を、受け取った順に連結する。アプリの知らせは入れない。
    pub fn clang_output(&self) -> String {
        self.output
            .iter()
            .filter(|piece| piece.kind == CBuildOutputKind::Clang)
            .map(|piece| piece.text.as_str())
            .collect()
    }
}

/// 動いているビルド。
pub struct RunningCBuild {
    process: RunningProcess,
    output: Arc<Mutex<Vec<CBuildOutputPiece>>>,
    time_limit: Duration,
}

impl RunningCBuild {
    /// 止める操作。ほかのスレッドに渡せる。
    pub fn handle(&self) -> CBuildHandle {
        CBuildHandle(self.process.handle())
    }

    /// ビルドが終わるまで待ち、結果を返す。
    pub fn wait(self) -> io::Result<CBuildOutcome> {
        let termination = self.process.wait()?;
        let mut output = std::mem::take(&mut *lock(&self.output));
        let status = match termination {
            Termination::Exited(0) => BuildStatus::Succeeded,
            Termination::Exited(code) => BuildStatus::Failed {
                exit_code: Some(code),
            },
            // ビルドに出力量の上限は無い（上限は数えきれない大きさにしている）。
            Termination::Signaled(_) | Termination::OutputLimitExceeded => {
                BuildStatus::Failed { exit_code: None }
            }
            Termination::TimedOut => BuildStatus::TimedOut,
            Termination::Stopped => BuildStatus::Stopped,
        };
        match status {
            // 止める操作で止めたときは、ビルドの出力を捨てる（run.md「停止」）。
            BuildStatus::Stopped => output.clear(),
            BuildStatus::TimedOut => output.push(CBuildOutputPiece {
                kind: CBuildOutputKind::System,
                text: timeout_message(self.time_limit),
            }),
            BuildStatus::Succeeded | BuildStatus::Failed { .. } => {}
        }
        Ok(CBuildOutcome { status, output })
    }
}

/// 動いているビルドへの操作。
#[derive(Clone)]
pub struct CBuildHandle(ProcessHandle);

impl CBuildHandle {
    /// `clang` のプロセスグループ全体（`clang` が起動したリンカなどを含む）を止める。終わった後に呼んでも何もしない。
    pub fn stop(&self) {
        self.0.stop();
    }
}

/// `spec` のとおりに `clang` を新しいプロセスグループで起動する。
pub fn start_c_build(spec: CBuildSpec) -> io::Result<RunningCBuild> {
    let mut process_spec = ProcessSpec::new(
        &spec.clang,
        &spec.project_dir,
        Limits {
            time: spec.time_limit,
            // 制限値（spec/exercise-runner/limits.md）にあるビルドの制限は時間だけなので、出力量は数えない。
            output: OutputLimit::Total(usize::MAX),
        },
    );
    process_spec.args = spec.kind.args().into_iter().map(OsString::from).collect();
    // 環境変数は足さず、アプリが受け取ったものをそのまま渡す（invocation.md「環境変数」）。
    process_spec.envs = Vec::new();
    process_spec.stdin = StdinMode::Closed;
    // ビルド時間の制限で止めたときも、それまでの出力を返す（run.md「ビルドの出力の表示」）。
    process_spec.on_timeout = TimeoutOutput::Deliver;

    let output = Arc::new(Mutex::new(Vec::new()));
    let process = {
        let output = Arc::clone(&output);
        process::spawn(process_spec, move |chunks| {
            push_chunks(&mut lock(&output), chunks);
        })?
    };
    Ok(RunningCBuild {
        process,
        output,
        time_limit: spec.time_limit,
    })
}

/// `spec` のとおりにビルドし、終わるまで待って結果を返す。
pub fn c_build(spec: CBuildSpec) -> io::Result<CBuildOutcome> {
    start_c_build(spec)?.wait()
}

/// 受け取った出力を、受け取った順に足す。標準出力と標準エラー出力を区別せず、続く断片は 1 つにまとめる。
fn push_chunks(output: &mut Vec<CBuildOutputPiece>, chunks: Vec<OutputChunk>) {
    for chunk in chunks {
        if chunk.text.is_empty() {
            continue;
        }
        match output.last_mut() {
            Some(last) if last.kind == CBuildOutputKind::Clang => last.text.push_str(&chunk.text),
            _ => output.push(CBuildOutputPiece {
                kind: CBuildOutputKind::Clang,
                text: chunk.text,
            }),
        }
    }
}

/// ロックを取る。ほかのスレッドが panic してロックが壊れていても、中の値を使う。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
