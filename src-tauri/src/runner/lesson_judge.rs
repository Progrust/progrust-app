//! 判定だけを行うコマンドラインのプログラム（`src/bin/progrust-judge.rs`）の中身
//! （spec/content-guidelines/check-tool.md、ADR 0208）。
//!
//! - 教材のレッスンのディレクトリと、判定するファイル（`solution/` か `starter/`）を受け取り、提出
//!   （spec/exercise-runner/submit.md）と同じ手順・制限値（[`super::submit::judge_submission`]）で判定して、
//!   判定結果を JSON で標準出力に書く。Tauri の実行時を使わない。
//! - 作業領域は、既定では `src-tauri/target/content-check/`（[`DEFAULT_WORKSPACE`]）にする。アプリの学習者データは
//!   読み書きしない。同梱の許可クレートの定義は、リポジトリの `runner/` を使う。
//! - 許可クレートを使う演習（`dependencies` が空でない演習）は、作業領域の初回準備が要るので、まだ判定しない（ADR 0222）。
//! - プログラムは、Cargo の feature `content-check` を付けたときだけビルドする。アプリの `.app` には入れない（ADR 0222）。
//!
//! 引数:
//!
//! ```text
//! progrust-judge --lesson <レッスンのディレクトリ> --files <solution|starter> [--workspace <作業領域のディレクトリ>]
//! ```
//!
//! 終了コード: 判定結果を書いたときは 0（判定結果の `status` によらない）。判定できなかったときは 1、引数が誤っているときは 2。

use std::ffi::OsString;
use std::fmt;
use std::path::PathBuf;
use std::process::ExitCode;

use serde::Serialize;

use super::RunnerDir;
use super::build::BUILD_TIME_LIMIT;
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
    /// 許可クレートを使う演習。作業領域の初回準備が要るので、まだ判定しない。
    UsesAllowedCrates(Vec<String>),
    /// 教材を読めなかった。
    Content(ContentError),
    /// 判定の途中で、アプリの側の失敗が起きた。
    Judge(CommandError),
}

impl fmt::Display for LessonJudgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoExercise => write!(f, "演習の無いレッスンは判定できない"),
            Self::UsesAllowedCrates(names) => write!(
                f,
                "許可クレートを使う演習（dependencies: {}）は、まだ判定できない",
                names.join(", ")
            ),
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
    if !dependencies.is_empty() {
        return Err(LessonJudgeError::UsesAllowedCrates(dependencies));
    }
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
    let judged = judge_submission(&request, &NoHooks).map_err(LessonJudgeError::Judge)?;
    let (judgement, built) = match judged {
        Judged::Done { result, built } => (SubmitJudgement::judged(result), built),
        Judged::CompileError(built) => (
            SubmitJudgement::without_cases(SubmitResultStatus::CompileError),
            built,
        ),
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
/// `CARGO` を使い、無いときは、このプログラムをビルドした `cargo` を使う。
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
    Ok(LessonJudgeOptions {
        lesson_dir: lesson_dir.ok_or("--lesson がありません。")?,
        files: files.ok_or("--files がありません。")?,
        workspace: WorkspaceDir::new(workspace.unwrap_or_else(|| PathBuf::from(DEFAULT_WORKSPACE))),
        runner: RunnerDir::new(REPOSITORY_RUNNER),
        cargo,
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
