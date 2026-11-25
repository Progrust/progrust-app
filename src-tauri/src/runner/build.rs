//! 組み立てた `project/` を cargo でビルドし、ビルドの出力・エラーコード・実行ファイルを得る部品
//! （spec/exercise-runner/run.md「手順」「テスト実行」「ビルドの出力の表示」「停止」、
//! spec/exercise-runner/submit.md「test 方式」、spec/exercise-runner/limits.md のビルド時間）。
//!
//! - [`start_build`] で、cargo を子プロセスの部品（[`super::process`]）で新しいプロセスグループに起動する。
//!   止める操作は [`BuildHandle`] で行い、結果は [`RunningBuild::wait`] で受け取る。
//! - ビルドの出力（コンパイラの診断と cargo 自身の標準エラー出力）は、ビルド中には渡さない。ビルドが終わってから、
//!   受け取った順に [`BuildOutcome::output`] で返す（ADR 0135、ADR 0139）。
//! - ビルド時間の制限で止めたときは、それまでの出力の後に、タイムアウトの旨を付ける。止める操作で止めたときは、
//!   出力を捨てる。
//! - 実行ファイルは、cargo の JSON の `compiler-artifact` から決める（ADR 0205、ADR 0215）。
//! - レベルが `error` の診断が指すファイルを残す。`test` 方式の判定で、隠しテストの側でエラーが出たかを見分けるのに使う
//!   （spec/exercise-runner/submit.md「test 方式」の 3）。
//! - レベルが `error` の診断の `code`・`message`・`note` の文と、cargo 自身の標準エラー出力から、ビルドの失敗が
//!   初回準備のやり直しが要る失敗に当てはまるかを判定する（[`BuildOutcome::needs_crate_preparation`]。判定そのものは
//!   [`super::lost_crates`]）。
//!
//! Tauri の実行時に頼らない（判定だけを行うコマンドラインのプログラムからも使う。ADR 0208）。

use std::{
    ffi::OsString,
    io,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard},
    time::Duration,
};

use serde::Deserialize;

use super::process::{
    self, Limits, OutputChunk, OutputLimit, OutputStream, ProcessHandle, ProcessSpec,
    RunningProcess, StdinMode, Termination, TimeoutOutput,
};
use crate::attempts::output::BuildOutput;

/// ビルド時間の制限（spec/exercise-runner/limits.md）。
pub const BUILD_TIME_LIMIT: Duration = Duration::from_secs(120);

/// cargo に渡す、出力の形の指定。
const MESSAGE_FORMAT: &str = "--message-format=json-diagnostic-rendered-ansi";

/// ビルドのしかた。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildCommand {
    /// 実行のビルド（`cargo build`）。
    Build,
    /// テスト実行と `test` 方式の判定のビルド（`cargo test --no-run`）。
    TestNoRun,
}

impl BuildCommand {
    /// cargo に渡す引数。
    pub fn args(self) -> &'static [&'static str] {
        match self {
            Self::Build => &["build", "--offline", "--quiet", MESSAGE_FORMAT],
            Self::TestNoRun => &["test", "--offline", "--quiet", "--no-run", MESSAGE_FORMAT],
        }
    }
}

/// ビルドするもの。
#[derive(Debug, Clone)]
pub struct BuildSpec {
    /// `cargo` の絶対パス。
    pub cargo: PathBuf,
    /// ビルドするプロジェクト（`project/`）。cargo の作業ディレクトリにする。
    pub project_dir: PathBuf,
    pub command: BuildCommand,
    /// アプリの環境変数に加えて設定する環境変数（[`super::project::build_env`]）。
    pub envs: Vec<(OsString, OsString)>,
    /// ビルド時間の制限。
    pub time_limit: Duration,
}

impl BuildSpec {
    /// ビルド時間の制限を [`BUILD_TIME_LIMIT`] にする。
    pub fn new<K, V>(
        cargo: impl Into<PathBuf>,
        project_dir: impl Into<PathBuf>,
        command: BuildCommand,
        envs: impl IntoIterator<Item = (K, V)>,
    ) -> Self
    where
        K: Into<OsString>,
        V: Into<OsString>,
    {
        Self {
            cargo: cargo.into(),
            project_dir: project_dir.into(),
            command,
            envs: envs
                .into_iter()
                .map(|(key, value)| (key.into(), value.into()))
                .collect(),
            time_limit: BUILD_TIME_LIMIT,
        }
    }
}

/// ビルドの出力の断片の種類。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildOutputKind {
    /// コンパイラの診断（`compiler-message` の `rendered`）。ANSI エスケープを含む。
    CompilerMessage,
    /// cargo 自身の標準エラー出力。
    CargoStderr,
    /// アプリが出す知らせ（タイムアウトの旨）。
    System,
}

/// ビルドの出力の断片。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutputPiece {
    pub kind: BuildOutputKind,
    pub text: String,
}

/// ビルドの終わり方。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildStatus {
    /// cargo が終了コード 0 で終わった。
    Succeeded,
    /// cargo が失敗した。`exit_code` は終了コード（シグナルで終わったときは `None`）。
    Failed { exit_code: Option<i32> },
    /// ビルド時間の制限で止めた。
    TimedOut,
    /// 止める操作（[`BuildHandle::stop`]）で止めた。
    Stopped,
}

/// cargo が作った、実行できる成果物（`compiler-artifact` のうち `executable` があるもの）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildArtifact {
    /// クレートの根のファイル（`target.src_path`）。
    pub src_path: PathBuf,
    /// 成果物の種類（`target.kind`。`bin`、`lib`、`test` など）。
    pub kinds: Vec<String>,
    /// テスト用の成果物か（`profile.test`）。
    pub test: bool,
    /// 実行ファイル（`executable`）。
    pub executable: PathBuf,
}

/// レベルが `error` の診断の、初回準備のやり直しの判定（[`super::lost_crates`]）に使う部分。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDiagnostic {
    /// 診断の `code`（`E0786` など）。無いときは `None`。
    pub code: Option<String>,
    /// 診断の `message`（`found invalid metadata files for crate ...` など）。ANSI エスケープを含まない。
    pub message: String,
    /// 子の診断のうち、レベルが `note` のものの `message`（`rendered` の `= note: ...` の行の文）。出た順に並ぶ。
    pub notes: Vec<String>,
}

/// ビルドの結果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildOutcome {
    pub status: BuildStatus,
    /// ビルドの出力。受け取った順に並ぶ。止める操作で止めたときは空。ビルド時間の制限で止めたときは、
    /// 最後に [`BuildOutputKind::System`] のタイムアウトの旨が付く。
    pub output: Vec<BuildOutputPiece>,
    /// レベルが `error` の診断の `code`。重複を除き、出現順に並ぶ（spec/exercise-runner/attempt-log.md）。
    pub error_codes: Vec<String>,
    /// レベルが `error` の診断が指すファイル（診断の `spans` の `file_name` と、マクロの展開をさかのぼった
    /// `expansion.span` の `file_name`。cargo の出したまま）。重複を除き、出現順に並ぶ。
    pub error_files: Vec<PathBuf>,
    /// レベルが `error` の診断。受け取った順に並ぶ。
    pub error_diagnostics: Vec<ErrorDiagnostic>,
    /// 実行できる成果物。受け取った順に並ぶ。
    pub artifacts: Vec<BuildArtifact>,
    project_dir: PathBuf,
}

impl BuildOutcome {
    /// `cargo build` が作った、`src/main.rs` をクレートの根にする実行ファイル（run.md「手順」の 3、ADR 0215）。
    /// `src/bin/` のファイルの実行ファイルは選ばない。
    pub fn main_executable(&self) -> Option<&Path> {
        self.find_executable(Path::new("src/main.rs"), false)
    }

    /// `cargo test --no-run` が作った、`src_path` をクレートの根にするテスト用の実行ファイル（ADR 0205）。
    /// `src_path` は、絶対パスか、プロジェクトのディレクトリからの相対パス（`src/lib.rs` など）。
    pub fn test_executable(&self, src_path: &Path) -> Option<&Path> {
        self.find_executable(src_path, true)
    }

    /// レベルが `error` の診断が、`file` を指しているか。`file` は、絶対パスか、プロジェクトのディレクトリからの
    /// 相対パス。診断のパス（`src/../progrust_judge_tests.rs` など）は、シンボリックリンクと `..` をたどって比べる。
    pub fn has_error_in(&self, file: &Path) -> bool {
        let wanted = canonical(&self.project_dir.join(file));
        self.error_files
            .iter()
            .any(|error_file| canonical(&self.project_dir.join(error_file)) == wanted)
    }

    fn find_executable(&self, src_path: &Path, test: bool) -> Option<&Path> {
        let wanted = canonical(&self.project_dir.join(src_path));
        self.artifacts
            .iter()
            .filter(|artifact| artifact.test == test)
            .filter(|artifact| test || artifact.kinds.iter().any(|kind| kind == "bin"))
            .find(|artifact| canonical(&artifact.src_path) == wanted)
            .map(|artifact| artifact.executable.as_path())
    }

    /// cargo 自身の標準エラー出力。受け取った順に連結する。
    pub fn cargo_stderr(&self) -> String {
        self.output
            .iter()
            .filter(|piece| piece.kind == BuildOutputKind::CargoStderr)
            .map(|piece| piece.text.as_str())
            .collect()
    }

    /// ビルドの失敗が、初回準備のやり直しが要る失敗（[`super::lost_crates::needs_crate_preparation`]）に当てはまるか。
    /// `target_dir` は、このビルドのビルド先（`CARGO_TARGET_DIR`）。cargo が失敗したときだけ当てはまりうる。
    pub fn needs_crate_preparation(&self, target_dir: &Path) -> bool {
        let BuildStatus::Failed { exit_code } = self.status else {
            return false;
        };
        super::lost_crates::needs_crate_preparation(
            exit_code,
            &self.cargo_stderr(),
            &self.error_diagnostics,
            target_dir,
        )
    }

    /// 試行ログの `stderr` に入れるビルドの出力（spec/exercise-runner/attempt-log.md）。
    /// コンパイラの診断と cargo 自身の標準エラー出力を、それぞれ受け取った順に連結する。アプリの知らせは入れない。
    pub fn attempt_output(&self) -> BuildOutput {
        let mut output = BuildOutput::default();
        for piece in &self.output {
            match piece.kind {
                BuildOutputKind::CompilerMessage => output.compiler_messages.push_str(&piece.text),
                BuildOutputKind::CargoStderr => output.cargo_stderr.push_str(&piece.text),
                BuildOutputKind::System => {}
            }
        }
        output
    }
}

/// ビルド時間の制限で止めたときの知らせ。
pub fn timeout_message(time_limit: Duration) -> String {
    format!(
        "ビルド時間の制限（{} 秒）を超えたため中断しました。",
        time_limit.as_secs()
    )
}

/// 動いているビルド。
pub struct RunningBuild {
    process: RunningProcess,
    collector: Arc<Mutex<Collector>>,
    project_dir: PathBuf,
    time_limit: Duration,
}

impl RunningBuild {
    /// 止める操作。ほかのスレッドに渡せる。
    pub fn handle(&self) -> BuildHandle {
        BuildHandle(self.process.handle())
    }

    /// ビルドが終わるまで待ち、結果を返す。
    pub fn wait(self) -> io::Result<BuildOutcome> {
        let termination = self.process.wait()?;
        let collected = std::mem::take(&mut *lock(&self.collector)).finish();
        let mut outcome = BuildOutcome {
            status: match termination {
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
            },
            output: collected.output,
            error_codes: collected.error_codes,
            error_files: collected.error_files,
            error_diagnostics: collected.error_diagnostics,
            artifacts: collected.artifacts,
            project_dir: self.project_dir,
        };
        match outcome.status {
            BuildStatus::Stopped => {
                // 止める操作で止めたときは、ビルドの出力を捨てる（run.md「停止」）。
                outcome.output.clear();
                outcome.error_codes.clear();
                outcome.error_files.clear();
                outcome.error_diagnostics.clear();
                outcome.artifacts.clear();
            }
            BuildStatus::TimedOut => outcome.output.push(BuildOutputPiece {
                kind: BuildOutputKind::System,
                text: timeout_message(self.time_limit),
            }),
            BuildStatus::Succeeded | BuildStatus::Failed { .. } => {}
        }
        Ok(outcome)
    }
}

/// 動いているビルドへの操作。
#[derive(Clone)]
pub struct BuildHandle(ProcessHandle);

impl BuildHandle {
    /// cargo のプロセスグループ全体（cargo が起動した rustc を含む）を止める。終わった後に呼んでも何もしない。
    pub fn stop(&self) {
        self.0.stop();
    }
}

/// `spec` のとおりに cargo を新しいプロセスグループで起動する。
pub fn start_build(spec: BuildSpec) -> io::Result<RunningBuild> {
    let mut process_spec = ProcessSpec::new(
        &spec.cargo,
        &spec.project_dir,
        Limits {
            time: spec.time_limit,
            // 制限値（spec/exercise-runner/limits.md）にあるビルドの制限は時間だけなので、出力量は数えない。
            output: OutputLimit::Total(usize::MAX),
        },
    );
    process_spec.args = spec.command.args().iter().map(OsString::from).collect();
    process_spec.envs = spec.envs;
    process_spec.stdin = StdinMode::Closed;
    // ビルド時間の制限で止めたときも、それまでの出力を返す（run.md「ビルドの出力の表示」）。
    process_spec.on_timeout = TimeoutOutput::Deliver;

    let collector = Arc::new(Mutex::new(Collector::default()));
    let process = {
        let collector = Arc::clone(&collector);
        process::spawn(process_spec, move |chunks| {
            lock(&collector).push_chunks(chunks);
        })?
    };
    Ok(RunningBuild {
        process,
        collector,
        project_dir: spec.project_dir,
        time_limit: spec.time_limit,
    })
}

/// `spec` のとおりにビルドし、終わるまで待って結果を返す。
pub fn build(spec: BuildSpec) -> io::Result<BuildOutcome> {
    start_build(spec)?.wait()
}

// ---- cargo の出力を読む ----

/// cargo の出力を、受け取った順に読んでためる。
#[derive(Debug, Default)]
struct Collector {
    /// 標準出力の、まだ改行が来ていない行。
    partial_line: String,
    output: Vec<BuildOutputPiece>,
    error_codes: Vec<String>,
    error_files: Vec<PathBuf>,
    error_diagnostics: Vec<ErrorDiagnostic>,
    artifacts: Vec<BuildArtifact>,
}

/// ためた結果。
struct Collected {
    output: Vec<BuildOutputPiece>,
    error_codes: Vec<String>,
    error_files: Vec<PathBuf>,
    error_diagnostics: Vec<ErrorDiagnostic>,
    artifacts: Vec<BuildArtifact>,
}

impl Collector {
    fn push_chunks(&mut self, chunks: Vec<OutputChunk>) {
        for chunk in chunks {
            match chunk.stream {
                OutputStream::Stdout => self.push_stdout(&chunk.text),
                OutputStream::Stderr => self.push_piece(BuildOutputKind::CargoStderr, &chunk.text),
            }
        }
    }

    /// 標準出力を、行ごとに JSON として読む。
    fn push_stdout(&mut self, text: &str) {
        self.partial_line.push_str(text);
        while let Some(end) = self.partial_line.find('\n') {
            let line: String = self.partial_line.drain(..=end).collect();
            self.read_line(&line);
        }
    }

    /// 最後に改行の無い行が残っていたら、それも読んでから、ためた結果を返す。
    fn finish(mut self) -> Collected {
        let rest = std::mem::take(&mut self.partial_line);
        self.read_line(&rest);
        Collected {
            output: self.output,
            error_codes: self.error_codes,
            error_files: self.error_files,
            error_diagnostics: self.error_diagnostics,
            artifacts: self.artifacts,
        }
    }

    /// cargo の JSON の 1 行を読む。JSON でない行と、知らない `reason` の行は無視する。
    fn read_line(&mut self, line: &str) {
        let line = line.trim();
        if line.is_empty() {
            return;
        }
        let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
            return;
        };
        match value.get("reason").and_then(serde_json::Value::as_str) {
            Some("compiler-message") => {
                if let Ok(message) = serde_json::from_value::<CompilerMessage>(value) {
                    self.push_diagnostic(message.message);
                }
            }
            Some("compiler-artifact") => {
                if let Ok(artifact) = serde_json::from_value::<CompilerArtifact>(value) {
                    self.push_artifact(artifact);
                }
            }
            _ => {}
        }
    }

    fn push_diagnostic(&mut self, diagnostic: Diagnostic) {
        if diagnostic.level == "error" {
            let code = diagnostic.code.map(|code| code.code);
            if let Some(code) = &code
                && !self.error_codes.contains(code)
            {
                self.error_codes.push(code.clone());
            }
            self.error_diagnostics.push(ErrorDiagnostic {
                code,
                message: diagnostic.message,
                notes: diagnostic
                    .children
                    .into_iter()
                    .filter(|child| child.level == "note")
                    .map(|child| child.message)
                    .collect(),
            });
            for span in diagnostic.spans {
                // マクロの中で出たエラー（`assert_eq!` の中の型の違いなど）の `file_name` はマクロを定義した
                // ファイルになるので、展開した場所（`expansion.span`）もさかのぼって残す。
                let mut span = Some(span);
                while let Some(DiagnosticSpan {
                    file_name,
                    expansion,
                }) = span
                {
                    let file = PathBuf::from(file_name);
                    if !self.error_files.contains(&file) {
                        self.error_files.push(file);
                    }
                    span = expansion.map(|expansion| expansion.span);
                }
            }
        }
        if let Some(rendered) = diagnostic.rendered {
            self.push_piece(BuildOutputKind::CompilerMessage, &rendered);
        }
    }

    fn push_artifact(&mut self, artifact: CompilerArtifact) {
        let Some(executable) = artifact.executable else {
            return;
        };
        self.artifacts.push(BuildArtifact {
            src_path: artifact.target.src_path,
            kinds: artifact.target.kind,
            test: artifact.profile.test,
            executable,
        });
    }

    /// 断片を足す。直前の断片と同じ種類なら、1 つにまとめる。
    fn push_piece(&mut self, kind: BuildOutputKind, text: &str) {
        if text.is_empty() {
            return;
        }
        match self.output.last_mut() {
            Some(last) if last.kind == kind => last.text.push_str(text),
            _ => self.output.push(BuildOutputPiece {
                kind,
                text: text.to_string(),
            }),
        }
    }
}

/// `reason` が `compiler-message` の行。
#[derive(Deserialize)]
struct CompilerMessage {
    message: Diagnostic,
}

#[derive(Deserialize)]
struct Diagnostic {
    level: String,
    #[serde(default)]
    message: String,
    #[serde(default)]
    code: Option<DiagnosticCode>,
    #[serde(default)]
    rendered: Option<String>,
    #[serde(default)]
    spans: Vec<DiagnosticSpan>,
    #[serde(default)]
    children: Vec<DiagnosticChild>,
}

/// 子の診断（`note`・`help` など）。
#[derive(Deserialize)]
struct DiagnosticChild {
    level: String,
    #[serde(default)]
    message: String,
}

#[derive(Deserialize)]
struct DiagnosticCode {
    code: String,
}

#[derive(Deserialize)]
struct DiagnosticSpan {
    file_name: String,
    #[serde(default)]
    expansion: Option<Box<SpanExpansion>>,
}

/// マクロの展開。`span` は、マクロを呼んだ場所。
#[derive(Deserialize)]
struct SpanExpansion {
    span: DiagnosticSpan,
}

/// `reason` が `compiler-artifact` の行。
#[derive(Deserialize)]
struct CompilerArtifact {
    target: ArtifactTarget,
    profile: ArtifactProfile,
    #[serde(default)]
    executable: Option<PathBuf>,
}

#[derive(Deserialize)]
struct ArtifactTarget {
    kind: Vec<String>,
    src_path: PathBuf,
}

#[derive(Deserialize)]
struct ArtifactProfile {
    test: bool,
}

/// シンボリックリンクをたどった絶対パス。たどれないときは、そのまま。
/// macOS の一時ディレクトリ（`/var` は `/private/var` へのリンク）などで、cargo が出すパスと形が違うことがあるため。
fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

/// ロックを取る。ほかのスレッドが panic してロックが壊れていても、中の値を使う。
fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[cfg(test)]
mod tests;
