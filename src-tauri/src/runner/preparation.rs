//! 許可クレートの初回準備の手順（spec/exercise-runner/crates-and-workspace.md「初回準備」の「手順」、
//! ADR 0011、ADR 0097、ADR 0098、ADR 0242）を 1 回動かす部品。
//!
//! - [`prepare`] が、手順の 1〜5 を順に行う。`allcrates/`・`warmup/` を作り、`warmup/` を `cargo build` と
//!   `cargo test --no-run` でビルドし（ビルド先は `target/`）、求められたときは `target/` を `target-validate/` に
//!   APFS のクローン（`cp -c`）で複製して、結果を `prepared.json` に書く。
//! - 何を使うか（同梱の許可クレートの一覧と `Cargo.lock`、作業領域のルート、`cargo`・`rustc` のパス、手順の 4 を
//!   するかどうか）は [`PreparationSpec`] で渡す。教材チェックツールの判定の作業領域では、手順の 4 をしない（ADR 0242）。
//! - `cargo` と `cp` は、子プロセスの部品（[`super::process`]）の [`Stopper`] で起動する。[`Stopper::stop`] で止めると、
//!   `prepared.json` を書かずに [`PreparationOutcome::Stopped`] を返す（ADR 0236）。
//! - 初回準備の状態をメモリに持つこと、始める条件の判定（[`super::prepared::needs_preparation`]）、いつ始めるか、
//!   いつ止めるかは、呼び出し側が決める。
//!
//! Tauri の実行時に頼らない（判定だけを行うコマンドラインのプログラムからも使う。ADR 0208、ADR 0242）。

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use super::RunnerDir;
use super::prepared::{PreparedRecord, crates_hash_of, rustc_verbose_version, write_prepared};
use super::process::{Limits, OutputLimit, ProcessSpec, Stopper, Termination};
use super::project::{ALLOWED_CRATES_PACKAGE, PACKAGE_NAME};
use super::workspace::WorkspaceDir;
use crate::progress::now_string;
use crate::storage::StorageError;

/// 初回準備用のプロジェクト（`warmup/`）のパッケージ名。
pub const WARMUP_PACKAGE: &str = "progrust_warmup";

/// APFS のクローンで複製する `cp`（macOS の `cp -c`）。
const CP: &str = "/bin/cp";

/// 初回準備の `cargo` と `cp` には、時間の制限を設けない。初回準備の時間はネットワークの速さで大きく変わり、
/// 仕様に制限が無い（spec/exercise-runner/limits.md）。止めるのは、呼び出し側の止める操作だけである。
const NO_TIME_LIMIT: Duration = Duration::MAX;

/// `prepared.json` の `error` に入れる、cargo などの標準エラー出力の末尾の行の数と、文字の数の上限。
/// 学習者に見せられる長さにする。
const ERROR_TAIL_LINES: usize = 20;
const ERROR_TAIL_CHARS: usize = 2000;

/// 初回準備に使うもの。
#[derive(Debug, Clone)]
pub struct PreparationSpec {
    /// 同梱の許可クレートの一覧（`allowed-crates.toml`）と `Cargo.lock` のあるディレクトリ。
    pub runner: RunnerDir,
    /// 作業領域のルート。
    pub workspace: WorkspaceDir,
    /// `cargo` の絶対パス。
    pub cargo: PathBuf,
    /// `rustc` の絶対パス。`prepared.json` の `rustc_version`（`rustc -vV` の出力）を得るのに使う。
    pub rustc: PathBuf,
    /// 手順の 4（`target/` を `target-validate/` に複製する）をするか。アプリでは `true`、教材チェックツールの
    /// 判定の作業領域では `false`（ADR 0242）。
    pub copy_validate_target: bool,
}

/// 初回準備の終わり方。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PreparationOutcome {
    /// 最後まで動き、`prepared.json` にこの記録を書いた。`status` は `ready` か `failed`。
    Finished(PreparedRecord),
    /// 止める操作で止めた。`prepared.json` は書いていない。
    Stopped,
}

/// 初回準備の手順の 1〜5 を 1 回動かす。終わるまで待つ（呼び出し側は、別のスレッドで呼ぶ）。
///
/// - 成功したら `status: ready` と `crates_hash`・`rustc_version`・`prepared_at` を、失敗したら `status: failed` と
///   `error`（どの段で失敗したかと、cargo の標準エラー出力の末尾など）を、`prepared.json` に書く。
/// - `stopper` で止めたときは、`prepared.json` を書かずに [`PreparationOutcome::Stopped`] を返す。
/// - `prepared.json` を書けなかったときは、[`StorageError`] を返す。
pub fn prepare(
    spec: &PreparationSpec,
    stopper: &Stopper,
) -> Result<PreparationOutcome, StorageError> {
    let mut crates_hash = String::new();
    let mut rustc_version = String::new();
    let result = run_steps(spec, stopper, &mut crates_hash, &mut rustc_version);
    // 手順の最後の段の後に止められたときも、書かない（アプリの終了で止めるとき。ADR 0236）。
    if stopper.is_stopped() {
        return Ok(PreparationOutcome::Stopped);
    }
    let record = match result {
        Ok(()) => PreparedRecord::ready(crates_hash, rustc_version, now_string()),
        Err(StepError::Failed(error)) => {
            PreparedRecord::failed(crates_hash, rustc_version, now_string(), error)
        }
        Err(StepError::Stopped) => return Ok(PreparationOutcome::Stopped),
    };
    write_prepared(&spec.workspace, &record)?;
    Ok(PreparationOutcome::Finished(record))
}

/// 手順の段が進めなかった理由。
enum StepError {
    /// 失敗した。値は `prepared.json` の `error` に書く文。
    Failed(String),
    /// 止める操作で止めた。
    Stopped,
}

/// 手順の 1〜4。`crates_hash` と `rustc_version` には、求められたところで値を入れる（失敗の記録にも使う）。
fn run_steps(
    spec: &PreparationSpec,
    stopper: &Stopper,
    crates_hash: &mut String,
    rustc_version: &mut String,
) -> Result<(), StepError> {
    if stopper.is_stopped() {
        return Err(StepError::Stopped);
    }
    let allowed_crates = read_bundled(&spec.runner.allowed_crates())?;
    let cargo_lock = read_bundled(&spec.runner.cargo_lock())?;
    *crates_hash = crates_hash_of(&allowed_crates, &cargo_lock);
    *rustc_version = rustc_verbose_version(&spec.rustc).ok_or_else(|| {
        StepError::Failed(format!(
            "`rustc -vV` を実行できませんでした（{}）。",
            spec.rustc.display()
        ))
    })?;
    let allowed_crates = String::from_utf8(allowed_crates).map_err(|_| {
        StepError::Failed(format!(
            "同梱の許可クレートの一覧が UTF-8 ではありません（{}）。",
            spec.runner.allowed_crates().display()
        ))
    })?;

    // 1. allcrates/
    let allcrates = spec.workspace.allcrates();
    recreate_dir(&allcrates)?;
    write_file(
        &allcrates.join("Cargo.toml"),
        allcrates_manifest(&allowed_crates).as_bytes(),
    )?;
    write_file(&allcrates.join("src/lib.rs"), b"")?;

    // 2. warmup/
    let warmup = spec.workspace.warmup();
    recreate_dir(&warmup)?;
    write_file(&warmup.join("Cargo.toml"), warmup_manifest().as_bytes())?;
    write_file(&warmup.join("Cargo.lock"), &cargo_lock)?;
    write_file(&warmup.join("src/main.rs"), b"fn main() {}\n")?;

    // 3. cargo build と cargo test --no-run（ビルド先は target/）。--locked・--frozen・--offline は付けない。
    run_cargo(spec, stopper, &["build", "--quiet"], "cargo build")?;
    run_cargo(
        spec,
        stopper,
        &["test", "--no-run", "--quiet"],
        "cargo test --no-run",
    )?;

    // 4. target/ を target-validate/ に複製する。
    if spec.copy_validate_target {
        copy_validate_target(&spec.workspace, stopper)?;
    }
    Ok(())
}

/// `allcrates/` の `Cargo.toml`。`[dependencies]` は `allowed-crates.toml` の内容そのまま。
/// 同梱の `Cargo.lock` を作るときの形（`tools/runner-lock/lock.ts` の `allcratesManifest`）と同じにする。
fn allcrates_manifest(allowed_crates: &str) -> String {
    format!(
        "[package]\nname = \"{ALLOWED_CRATES_PACKAGE}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n{allowed_crates}"
    )
}

/// `warmup/` の `Cargo.toml`。`[dependencies]` は `allcrates/` への path 依存 1 行だけ。
fn warmup_manifest() -> String {
    format!(
        "[package]\nname = \"{WARMUP_PACKAGE}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n\
         [dependencies]\n{ALLOWED_CRATES_PACKAGE} = {{ path = \"../allcrates\" }}\n"
    )
}

/// `warmup/` で cargo を `args` で動かす。`label` は、失敗の文に書くコマンドの名前。
fn run_cargo(
    spec: &PreparationSpec,
    stopper: &Stopper,
    args: &[&str],
    label: &str,
) -> Result<(), StepError> {
    let mut process = ProcessSpec::new(&spec.cargo, spec.workspace.warmup(), limits());
    process.args = args.iter().map(OsString::from).collect();
    process.envs = vec![
        (
            OsString::from("CARGO_TARGET_DIR"),
            spec.workspace.target().into_os_string(),
        ),
        // `error` に書く出力に、色の指定（ANSI エスケープ）を入れない。
        (OsString::from("CARGO_TERM_COLOR"), OsString::from("never")),
    ];
    let output = stopper.collect_output(process).map_err(|error| {
        StepError::Failed(format!(
            "cargo を起動できませんでした（{}）: {error}",
            spec.cargo.display()
        ))
    })?;
    check_termination(output.termination, &output.stderr, &format!("`{label}`"))
}

/// 手順の 4。`target-validate/` を消し、`target/` を APFS のクローンで複製してから、学習者の `project/` の
/// ビルド結果を消す（ADR 0098）。
fn copy_validate_target(workspace: &WorkspaceDir, stopper: &Stopper) -> Result<(), StepError> {
    let target_validate = workspace.target_validate();
    remove_path(&target_validate)?;
    let mut process = ProcessSpec::new(CP, workspace.root(), limits());
    process.args = vec![
        OsString::from("-c"),
        OsString::from("-R"),
        OsString::from("-p"),
        workspace.target().into_os_string(),
        target_validate.clone().into_os_string(),
    ];
    let output = stopper.collect_output(process).map_err(|error| {
        StepError::Failed(format!(
            "target/ を target-validate/ に複製できませんでした（{CP} を起動できない）: {error}"
        ))
    })?;
    check_termination(
        output.termination,
        &output.stderr,
        "target/ から target-validate/ への複製（`cp -c`）",
    )?;
    remove_exercise_builds(&target_validate).map_err(|(path, error)| {
        StepError::Failed(format!(
            "target-validate/ から学習者のビルド結果を消せませんでした（{}）: {error}",
            path.display()
        ))
    })
}

/// 学習者の `project/`（パッケージ [`PACKAGE_NAME`]）のビルド結果を、ビルド先 `target_dir` から消す。
///
/// ビルド先の中の配置は cargo の版で変わるので、両方の配置のものを消す（ADR 0244）。ビルド先の下のディレクトリ
/// （プロファイルの `debug/` など）ごとに、次のものを消す。
///
/// - どの配置でも: 実行ファイルとその付属のファイル（`progrust_exercise`、`progrust_exercise.d` など）、ライブラリ
///   （`libprogrust_exercise.*`）、`incremental/` の `progrust_exercise-*`。
/// - cargo 1.100.0 の配置: `build/progrust_exercise/`（途中の成果物と fingerprint）。
/// - cargo 1.99.1 の配置: `deps/` の `progrust_exercise-*`・`libprogrust_exercise-*`、`.fingerprint/` と `build/` の
///   `progrust_exercise-*`。
pub fn remove_exercise_builds(target_dir: &Path) -> Result<(), (PathBuf, io::Error)> {
    let exact = PACKAGE_NAME.to_string();
    let with_extension = format!("{PACKAGE_NAME}.");
    let library = format!("lib{PACKAGE_NAME}.");
    let hashed = format!("{PACKAGE_NAME}-");
    let hashed_library = format!("lib{PACKAGE_NAME}-");

    for profile in child_dirs(target_dir)? {
        remove_entries(&profile, |name| {
            name == exact || name.starts_with(&with_extension) || name.starts_with(&library)
        })?;
        remove_entries(&profile.join("build"), |name| {
            name == exact || name.starts_with(&hashed)
        })?;
        remove_entries(&profile.join("deps"), |name| {
            name.starts_with(&hashed) || name.starts_with(&hashed_library)
        })?;
        remove_entries(&profile.join(".fingerprint"), |name| {
            name.starts_with(&hashed)
        })?;
        remove_entries(&profile.join("incremental"), |name| {
            name.starts_with(&hashed)
        })?;
    }
    Ok(())
}

/// `dir` の下のディレクトリ。`dir` が無いときは空。
fn child_dirs(dir: &Path) -> Result<Vec<PathBuf>, (PathBuf, io::Error)> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err((dir.to_path_buf(), error)),
    };
    let mut dirs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| (dir.to_path_buf(), error))?;
        let file_type = entry.file_type().map_err(|error| (entry.path(), error))?;
        if file_type.is_dir() {
            dirs.push(entry.path());
        }
    }
    Ok(dirs)
}

/// `dir` の中の、名前が `matches` に当てはまる項目を消す。`dir` が無いときは何もしない。
fn remove_entries(dir: &Path, matches: impl Fn(&str) -> bool) -> Result<(), (PathBuf, io::Error)> {
    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err((dir.to_path_buf(), error)),
    };
    for entry in entries {
        let entry = entry.map_err(|error| (dir.to_path_buf(), error))?;
        if entry.file_name().to_str().is_some_and(&matches) {
            let path = entry.path();
            remove_path_io(&path).map_err(|error| (path, error))?;
        }
    }
    Ok(())
}

/// 子プロセスの終わり方を、手順の結果にする。`what` は、失敗の文に書く段の名前。
fn check_termination(termination: Termination, stderr: &str, what: &str) -> Result<(), StepError> {
    let failure = match termination {
        Termination::Exited(0) => return Ok(()),
        Termination::Stopped => return Err(StepError::Stopped),
        Termination::Exited(code) => format!("{what}が失敗しました（終了コード {code}）。"),
        Termination::Signaled(signal) => {
            format!("{what}が失敗しました（シグナル {signal} で終了）。")
        }
        // 時間と出力量の制限は設けていないので、ふつうは起きない。
        Termination::TimedOut | Termination::OutputLimitExceeded => {
            format!("{what}が途中で止まりました。")
        }
    };
    let tail = output_tail(stderr);
    Err(StepError::Failed(if tail.is_empty() {
        failure
    } else {
        format!("{failure}\n{tail}")
    }))
}

/// 標準エラー出力の末尾（[`ERROR_TAIL_LINES`] 行・[`ERROR_TAIL_CHARS`] 文字まで）。省いたときは、先頭に `…` を付ける。
fn output_tail(output: &str) -> String {
    let output = output.trim_end();
    let lines: Vec<&str> = output.lines().collect();
    let start = lines.len().saturating_sub(ERROR_TAIL_LINES);
    let tail = lines[start..].join("\n");
    let chars = tail.chars().count();
    if chars > ERROR_TAIL_CHARS {
        let kept: String = tail.chars().skip(chars - ERROR_TAIL_CHARS).collect();
        format!("…{kept}")
    } else if start > 0 {
        format!("…\n{tail}")
    } else {
        tail
    }
}

fn limits() -> Limits {
    Limits {
        time: NO_TIME_LIMIT,
        output: OutputLimit::Total(usize::MAX),
    }
}

fn read_bundled(path: &Path) -> Result<Vec<u8>, StepError> {
    fs::read(path).map_err(|error| {
        StepError::Failed(format!(
            "同梱の許可クレートのファイルを読めませんでした（{}）: {error}",
            path.display()
        ))
    })
}

/// `dir` を空のディレクトリにする。無いときは作る。
fn recreate_dir(dir: &Path) -> Result<(), StepError> {
    remove_path(dir)?;
    fs::create_dir_all(dir).map_err(|error| write_failed(dir, &error))
}

/// `path` に `contents` を書く。親のディレクトリが無ければ作る。
fn write_file(path: &Path, contents: &[u8]) -> Result<(), StepError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| write_failed(parent, &error))?;
    }
    fs::write(path, contents).map_err(|error| write_failed(path, &error))
}

/// `path` を消す。ディレクトリなら中身ごと消す。無いときは何もしない。
fn remove_path(path: &Path) -> Result<(), StepError> {
    remove_path_io(path).map_err(|error| write_failed(path, &error))
}

fn remove_path_io(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => fs::remove_dir_all(path),
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn write_failed(path: &Path, error: &io::Error) -> StepError {
    StepError::Failed(format!(
        "作業領域のファイルを書き換えられませんでした（{}）: {error}",
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_allcrates_manifest_has_the_allowed_crates_as_they_are() {
        assert_eq!(
            allcrates_manifest("[dependencies]\nrand = \"=0.9.1\"\n"),
            "[package]\n\
             name = \"progrust_allowed_crates\"\n\
             version = \"0.1.0\"\n\
             edition = \"2024\"\n\
             \n\
             [dependencies]\n\
             rand = \"=0.9.1\"\n"
        );
    }

    #[test]
    fn the_warmup_manifest_depends_only_on_allcrates() {
        let manifest: toml::Table = warmup_manifest().parse().unwrap();
        assert_eq!(
            manifest["package"]["name"].as_str(),
            Some("progrust_warmup")
        );
        assert_eq!(manifest["package"]["edition"].as_str(), Some("2024"));
        let expected: toml::Table = "progrust_allowed_crates = { path = \"../allcrates\" }"
            .parse()
            .unwrap();
        assert_eq!(manifest["dependencies"].as_table(), Some(&expected));
    }

    #[test]
    fn a_short_output_is_kept_whole() {
        assert_eq!(output_tail("error: a\nerror: b\n\n"), "error: a\nerror: b");
        assert_eq!(output_tail(""), "");
    }

    #[test]
    fn a_long_output_keeps_only_the_last_lines() {
        let output: String = (1..=50).map(|index| format!("line {index}\n")).collect();
        let tail = output_tail(&output);
        let expected: Vec<String> = (31..=50).map(|index| format!("line {index}")).collect();
        assert_eq!(tail, format!("…\n{}", expected.join("\n")));
    }

    #[test]
    fn a_long_line_keeps_only_the_last_characters() {
        let output = format!("{}末尾", "あ".repeat(5000));
        let tail = output_tail(&output);
        assert_eq!(tail.chars().count(), ERROR_TAIL_CHARS + 1);
        assert!(tail.starts_with('…'), "{tail}");
        assert!(tail.ends_with("末尾"), "{tail}");
    }

    #[test]
    fn a_failed_cargo_names_the_step_and_shows_the_output() {
        let Err(StepError::Failed(error)) = check_termination(
            Termination::Exited(101),
            "    Updating crates.io index\nerror: failed to get `rand`\n",
            "`cargo build`",
        ) else {
            panic!("失敗にならない");
        };
        assert_eq!(
            error,
            "`cargo build`が失敗しました（終了コード 101）。\n\
             \x20   Updating crates.io index\n\
             error: failed to get `rand`"
        );
        assert!(matches!(
            check_termination(Termination::Stopped, "", "`cargo build`"),
            Err(StepError::Stopped)
        ));
        assert!(check_termination(Termination::Exited(0), "warning", "`cargo build`").is_ok());
    }

    /// 学習者の `project/` のビルド結果だけを消し、ほかのパッケージのビルド結果は残す。
    #[test]
    fn removes_only_the_exercise_builds() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path();
        let files = [
            ("debug/progrust_exercise", true),
            ("debug/progrust_exercise.d", true),
            ("debug/libprogrust_exercise.rlib", true),
            ("debug/libprogrust_exercise.d", true),
            ("debug/progrust_warmup", false),
            ("debug/progrust_warmup.d", false),
            ("debug/deps/progrust_exercise-0123abcd", true),
            ("debug/deps/progrust_exercise-0123abcd.d", true),
            ("debug/deps/libprogrust_exercise-0123abcd.rlib", true),
            ("debug/deps/progrust_warmup-89ab", false),
            ("debug/deps/librand-4567.rlib", false),
            ("debug/deps/libprogrust_allowed_crates-cdef.rlib", false),
            (
                "debug/.fingerprint/progrust_exercise-0123abcd/bin-progrust_exercise",
                true,
            ),
            (
                "debug/.fingerprint/progrust_warmup-89ab/bin-progrust_warmup",
                false,
            ),
            ("debug/.fingerprint/rand-4567/lib-rand", false),
            (
                "debug/incremental/progrust_exercise-1x2y/s-abc/dep-graph.bin",
                true,
            ),
            (
                "debug/incremental/progrust_warmup-3z4w/s-def/dep-graph.bin",
                false,
            ),
            // cargo 1.100.0 の配置（build/{パッケージ}/{ハッシュ}/）。
            (
                "debug/build/progrust_exercise/0123abcd/fingerprint/bin-progrust_exercise",
                true,
            ),
            (
                "debug/build/progrust_exercise/0123abcd/out/progrust_exercise-0123abcd",
                true,
            ),
            (
                "debug/build/progrust_warmup/89ab/fingerprint/bin-progrust_warmup",
                false,
            ),
            (
                "debug/build/progrust_allowed_crates/cdef/out/libprogrust_allowed_crates-cdef.rlib",
                false,
            ),
            ("debug/build/rand/4567/out/librand-4567.rlib", false),
            (".rustc_info.json", false),
            ("CACHEDIR.TAG", false),
        ];
        for (path, _) in files {
            let path = target.join(path);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(&path, "").unwrap();
        }
        fs::create_dir_all(target.join("debug/progrust_exercise.dSYM/Contents")).unwrap();

        remove_exercise_builds(target).unwrap();

        for (path, removed) in files {
            assert_eq!(!target.join(path).exists(), removed, "{path}");
        }
        assert!(!target.join("debug/progrust_exercise.dSYM").exists());
        assert!(
            !target
                .join("debug/.fingerprint/progrust_exercise-0123abcd")
                .exists()
        );
        assert!(
            !target
                .join("debug/incremental/progrust_exercise-1x2y")
                .exists()
        );
        assert!(!target.join("debug/build/progrust_exercise").exists());
    }

    #[test]
    fn removing_from_a_missing_target_does_nothing() {
        let temp = tempfile::tempdir().unwrap();
        remove_exercise_builds(&temp.path().join("missing")).unwrap();
    }

    /// 始める前に止めたときは、何も作らず、`prepared.json` も書かない。
    #[test]
    fn stopping_before_the_start_writes_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let workspace = WorkspaceDir::new(temp.path().join("workspace"));
        let stopper = Stopper::new();
        stopper.stop();
        let spec = PreparationSpec {
            runner: RunnerDir::new(concat!(env!("CARGO_MANIFEST_DIR"), "/../runner")),
            workspace: workspace.clone(),
            cargo: PathBuf::from(env!("CARGO")),
            rustc: PathBuf::from(env!("CARGO")).with_file_name("rustc"),
            copy_validate_target: false,
        };
        assert_eq!(
            prepare(&spec, &stopper).unwrap(),
            PreparationOutcome::Stopped
        );
        assert!(!workspace.root().exists());
    }
}
