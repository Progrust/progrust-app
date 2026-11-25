//! 判定だけを行うコマンドラインのプログラム（`src/bin/progrust-judge.rs`）の中身
//! （spec/content-guidelines/check-tool.md、ADR 0208）。
//!
//! - 教材のレッスンのディレクトリと、判定するファイル（`solution/` か `starter/`）を受け取り、提出
//!   （spec/exercise-runner/submit.md）と同じ手順・制限値（[`super::submit::judge_submission`]）で判定して、
//!   判定結果を JSON で標準出力に書く。Tauri の実行時を使わない。
//! - 作業領域は、既定では `src-tauri/target/content-check/`（[`DEFAULT_WORKSPACE`]）にする。アプリの学習者データは
//!   読み書きしない。同梱の許可クレートの定義は、リポジトリの `runner/` を使う。
//! - 許可クレートを使う演習（`dependencies` が空でない演習）は、判定の前に、作業領域で初回準備の「実行する条件」を
//!   判定し、当てはまれば初回準備をする（[`prepare_crates`]。ADR 0242）。初回準備では、`target-validate/` への複製
//!   （手順の 4）をしない。初回準備に失敗したら、判定せずに失敗する。
//! - 判定のビルドが、初回準備のやり直しが要る失敗になったときは、`prepared.json` の `status` を `failed` にし、
//!   初回準備をやり直してから、1 回だけ判定し直す。また同じ失敗になったら、判定せずに失敗する（ADR 0245）。
//! - プログラムは、Cargo の feature `content-check` を付けたときだけビルドする。アプリの `.app` には入れない（ADR 0222）。
//!
//! 引数:
//!
//! ```text
//! progrust-judge --lesson <レッスンのディレクトリ> --files <solution|starter> [--workspace <作業領域のディレクトリ>]
//! ```
//!
//! 終了コード: 判定結果を書いたときは 0（判定結果の `status` によらない）。判定できなかったとき（初回準備に
//! 失敗したときを含む）は 1、引数が誤っているときは 2。

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

use serde::Serialize;

use super::RunnerDir;
use super::build::BUILD_TIME_LIMIT;
use super::preparation::{PreparationOutcome, PreparationSpec, prepare};
use super::prepared::{LOST_CRATES_ERROR, PreparedStatus, mark_prepared_failed};
use super::process::Stopper;
use super::submit::{
    JudgeRequest, Judged, NoHooks, SubmitJudgement, SubmitResultStatus, judge_submission,
};
use super::workspace::WorkspaceDir;
use crate::attempts::output::attempt_stderr;
use crate::commands::error::{CommandError, ErrorCode};
use crate::content::{
    ContentError, has_exercise, read_exercise_dependencies, read_exercise_files,
    read_exercise_judge, read_lesson_kind,
};
use crate::environment::preparation::{Judgment, judge};

/// 既定の作業領域。`src-tauri/target/` の下の、教材チェックツール専用のディレクトリ。
pub const DEFAULT_WORKSPACE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/target/content-check");
/// リポジトリの `runner/`（同梱の許可クレートの定義）。
pub const REPOSITORY_RUNNER: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../runner");

const USAGE: &str = "使い方: progrust-judge --lesson <レッスンのディレクトリ> --files <solution|starter> [--workspace <作業領域のディレクトリ>]";

/// 判定するファイル。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LessonFiles {
    /// 模範解答（`exercise/solution/`）。
    Solution,
    /// 初期コード（`exercise/starter/`）。
    Starter,
}

impl LessonFiles {
    /// `exercise/` の下のディレクトリの名前。
    pub fn dir_name(self) -> &'static str {
        match self {
            Self::Solution => "solution",
            Self::Starter => "starter",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "solution" => Some(Self::Solution),
            "starter" => Some(Self::Starter),
            _ => None,
        }
    }
}

/// 判定するものと、使うもの。
#[derive(Debug, Clone)]
pub struct LessonJudgeOptions {
    /// レッスンのディレクトリ（`content/lessons/{lesson_id}/`）。
    pub lesson_dir: PathBuf,
    pub files: LessonFiles,
    pub workspace: WorkspaceDir,
    pub runner: RunnerDir,
    /// `cargo` のパス。
    pub cargo: PathBuf,
    /// `rustc` のパス。初回準備の「実行する条件」の判定と、`prepared.json` の `rustc_version` に使う。
    pub rustc: PathBuf,
}

/// 標準出力に書く判定結果。提出の判定結果（`run_finished` の `submit`）と同じ項目に、ビルドの出力を足す。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LessonJudgement {
    #[serde(flatten)]
    pub judgement: SubmitJudgement,
    /// ビルドの出力（コンパイラの診断と cargo 自身の標準エラー出力。ANSI エスケープを除く）。
    /// `compile_error` の理由を、教材の作者に見せるために使う。
    pub build_output: String,
}

/// 判定できなかった理由。
#[derive(Debug)]
pub enum LessonJudgeError {
    /// 演習の無いレッスン（動画のみのレッスン）。
    NoExercise,
    /// 許可クレートを使う演習で、作業領域の初回準備に失敗した。値は失敗の内容。
    Preparation(String),
    /// 初回準備をやり直しても、判定のビルドが、初回準備のやり直しが要る失敗になった。値はビルドの出力。
    /// `prepared.json` の `status` は `failed` にしてある。
    CratesUnavailable(String),
    /// 教材を読めなかった。
    Content(ContentError),
    /// 判定の途中で、アプリの側の失敗が起きた。
    Judge(CommandError),
}

impl fmt::Display for LessonJudgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoExercise => write!(f, "演習の無いレッスンは判定できない"),
            Self::Preparation(error) => {
                write!(
                    f,
                    "許可クレートの初回準備に失敗したので、判定できない: {error}"
                )
            }
            Self::CratesUnavailable(build_output) => {
                write!(
                    f,
                    "初回準備をやり直しても、許可クレートのソースかビルド済みの依存が失われていたので、\
                     判定できない。prepared.json の status を failed にした。"
                )?;
                let build_output = build_output.trim_end();
                if !build_output.is_empty() {
                    write!(f, "\nビルドの出力:\n{build_output}")?;
                }
                Ok(())
            }
            Self::Content(error) => write!(f, "教材を読めない: {error}"),
            Self::Judge(error) => {
                write!(f, "判定できなかった: {}", error.message)?;
                if let Some(detail) = &error.detail {
                    write!(f, "（{detail}）")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for LessonJudgeError {}

impl From<ContentError> for LessonJudgeError {
    fn from(error: ContentError) -> Self {
        Self::Content(error)
    }
}

/// レッスンの `solution/` か `starter/` を、提出と同じ手順・制限値で判定する。
pub fn judge_lesson(options: &LessonJudgeOptions) -> Result<LessonJudgement, LessonJudgeError> {
    let lesson_dir = &options.lesson_dir;
    if !has_exercise(&read_lesson_kind(lesson_dir)?) {
        return Err(LessonJudgeError::NoExercise);
    }
    let dependencies = read_exercise_dependencies(lesson_dir)?;
    let judge = read_exercise_judge(lesson_dir)?;
    let files = read_exercise_files(&lesson_dir.join("exercise").join(options.files.dir_name()))?;
    let request = JudgeRequest {
        cargo: &options.cargo,
        workspace: &options.workspace,
        runner: &options.runner,
        dependencies: &dependencies,
        files: &files,
        judge: &judge,
        build_time: BUILD_TIME_LIMIT,
    };
    if !dependencies.is_empty() {
        prepare_crates(options)?;
    }
    let mut judged = judge_submission(&request, &NoHooks).map_err(LessonJudgeError::Judge)?;
    if let Judged::CratesUnavailable(_) = judged {
        // アプリと同じく `prepared.json` の `status` を `failed` にし、初回準備をやり直してから、1 回だけ判定し直す
        // （ADR 0245）。
        mark_failed(&options.workspace);
        prepare_crates(options)?;
        judged = judge_submission(&request, &NoHooks).map_err(LessonJudgeError::Judge)?;
    }
    let (judgement, built) = match judged {
        Judged::Done { result, built } => (SubmitJudgement::judged(result), built),
        Judged::CompileError(built) => (
            SubmitJudgement::without_cases(SubmitResultStatus::CompileError),
            built,
        ),
        // 初回準備をやり直しても、許可クレートが失われていた。判定結果（`crates_unavailable`）は返さない。返すと、
        // 「初期コード」の検査が、判定せずに通ってしまう（ADR 0245）。
        Judged::CratesUnavailable(built) => {
            mark_failed(&options.workspace);
            return Err(LessonJudgeError::CratesUnavailable(attempt_stderr(
                &built.attempt_output(),
                "",
            )));
        }
        // 止める操作が無いので、ここには来ない。
        Judged::Stopped => {
            return Err(LessonJudgeError::Judge(CommandError {
                code: ErrorCode::Internal,
                message: "判定が途中で止まりました。".into(),
                detail: None,
            }));
        }
    };
    Ok(LessonJudgement {
        judgement,
        build_output: attempt_stderr(&built.attempt_output(), ""),
    })
}

/// 作業領域の `prepared.json` の `status` を `failed` にする。書けなかったときは、標準エラー出力に書いて続ける。
fn mark_failed(workspace: &WorkspaceDir) {
    if let Err(error) = mark_prepared_failed(workspace, LOST_CRATES_ERROR) {
        eprintln!("prepared.json の status を failed にできなかった: {error}");
    }
}

/// 作業領域で、初回準備の「実行する条件」（spec/exercise-runner/crates-and-workspace.md「初回準備」）を判定し、
/// 当てはまれば初回準備をする。判定と手順は、アプリと同じ部品（[`judge`]、[`prepare`]）を使う。手順の 4
/// （`target-validate/` への複製）はしない（ADR 0242）。
///
/// 初回準備が失敗したとき（`prepared.json` に `status: failed` を書いたとき、`prepared.json` を書けなかったとき）は、
/// [`LessonJudgeError::Preparation`] を返す。
fn prepare_crates(options: &LessonJudgeOptions) -> Result<(), LessonJudgeError> {
    if let Judgment::Ready { .. } = judge(&options.rustc, &options.runner, &options.workspace) {
        return Ok(());
    }
    let spec = PreparationSpec {
        runner: options.runner.clone(),
        workspace: options.workspace.clone(),
        cargo: options.cargo.clone(),
        rustc: options.rustc.clone(),
        copy_validate_target: false,
    };
    // 止める操作は無い（判定のプログラムは、初回準備が終わるまで待つ）。
    match prepare(&spec, &Stopper::new()) {
        Ok(PreparationOutcome::Finished(record)) => match record.status {
            PreparedStatus::Ready => Ok(()),
            PreparedStatus::Failed => Err(LessonJudgeError::Preparation(
                record.error.unwrap_or_default(),
            )),
        },
        Ok(PreparationOutcome::Stopped) => Err(LessonJudgeError::Preparation(
            "初回準備が途中で止まりました。".into(),
        )),
        Err(error) => Err(LessonJudgeError::Preparation(format!(
            "初回準備の結果を prepared.json に書けませんでした: {error}"
        ))),
    }
}

/// 判定だけを行うコマンドラインのプログラムの本体。`args` はプログラムの名前を除いた引数。
pub fn main(args: impl IntoIterator<Item = OsString>) -> ExitCode {
    let options = match parse_args(args) {
        Ok(options) => options,
        Err(message) => {
            eprintln!("{message}\n{USAGE}");
            return ExitCode::from(2);
        }
    };
    match judge_lesson(&options) {
        Ok(judgement) => match serde_json::to_string(&judgement) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("判定結果を JSON にできなかった: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("{}: {error}", options.lesson_dir.display());
            ExitCode::FAILURE
        }
    }
}

/// 引数を読む。`--workspace` が無いときは [`DEFAULT_WORKSPACE`] にする。`cargo` は、`cargo run` が渡す環境変数
/// `CARGO` を使い、無いときは、このプログラムをビルドした `cargo` を使う。`rustc` は、アプリの検出と同じく、
/// `cargo` と同じディレクトリのものを使う。
fn parse_args(args: impl IntoIterator<Item = OsString>) -> Result<LessonJudgeOptions, String> {
    let mut lesson_dir = None;
    let mut files = None;
    let mut workspace = None;
    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        let name = arg.to_string_lossy().into_owned();
        let value = match name.as_str() {
            "--lesson" | "--files" | "--workspace" => args
                .next()
                .ok_or_else(|| format!("{name} の値がありません。"))?,
            _ => return Err(format!("知らない引数です: {name}")),
        };
        match name.as_str() {
            "--lesson" => lesson_dir = Some(PathBuf::from(value)),
            "--workspace" => workspace = Some(PathBuf::from(value)),
            _ => {
                let text = value.to_string_lossy();
                files = Some(LessonFiles::parse(&text).ok_or_else(|| {
                    format!("--files は solution か starter にしてください: {text}")
                })?);
            }
        }
    }
    let cargo = std::env::var_os("CARGO")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env!("CARGO")));
    let rustc = cargo.with_file_name("rustc");
    Ok(LessonJudgeOptions {
        lesson_dir: lesson_dir.ok_or("--lesson がありません。")?,
        files: files.ok_or("--files がありません。")?,
        workspace: WorkspaceDir::new(workspace.unwrap_or_else(|| PathBuf::from(DEFAULT_WORKSPACE))),
        runner: RunnerDir::new(REPOSITORY_RUNNER),
        cargo,
        rustc,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::path::Path;

    fn args(list: &[&str]) -> Vec<OsString> {
        list.iter().map(OsString::from).collect()
    }

    #[test]
    fn parses_lesson_files_and_workspace() {
        let options = parse_args(args(&[
            "--lesson",
            "/c/lessons/ch01-01",
            "--files",
            "starter",
            "--workspace",
            "/w",
        ]))
        .unwrap();
        assert_eq!(options.lesson_dir, Path::new("/c/lessons/ch01-01"));
        assert_eq!(options.files, LessonFiles::Starter);
        assert_eq!(options.workspace.root(), Path::new("/w"));
        assert_eq!(options.runner.root(), Path::new(REPOSITORY_RUNNER));
        // rustc は、cargo と同じディレクトリのもの。
        assert_eq!(options.rustc, options.cargo.with_file_name("rustc"));
    }

    #[test]
    fn workspace_defaults_to_the_content_check_directory_under_target() {
        let options = parse_args(args(&["--files", "solution", "--lesson", "l"])).unwrap();
        assert_eq!(options.files, LessonFiles::Solution);
        assert_eq!(
            options.workspace.root(),
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/content-check")
        );
        // 作業領域のビルド先の分け方は、アプリの作業領域と同じ。
        assert_eq!(
            options.workspace.project_target(false),
            Path::new(env!("CARGO_MANIFEST_DIR")).join("target/content-check/target-std")
        );
    }

    #[test]
    fn rejects_missing_or_unknown_arguments() {
        for list in [
            &["--files", "solution"][..],
            &["--lesson", "l"],
            &["--lesson", "l", "--files", "answer"],
            &["--lesson", "l", "--files"],
            &["--lesson", "l", "--files", "solution", "--verbose"],
        ] {
            assert!(parse_args(args(list)).is_err(), "{list:?}");
        }
    }

    #[test]
    fn judgement_serializes_with_the_build_output() {
        let judgement = LessonJudgement {
            judgement: SubmitJudgement::without_cases(SubmitResultStatus::CompileError),
            build_output: "error[E0382]: borrow of moved value\n".into(),
        };
        assert_eq!(
            serde_json::to_value(&judgement).unwrap(),
            serde_json::json!({
                "status": "compile_error",
                "passed": 0,
                "total": 0,
                "cases": [],
                "build_output": "error[E0382]: borrow of moved value\n",
            })
        );
    }
}
