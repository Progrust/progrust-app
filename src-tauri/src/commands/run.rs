//! 実行のコマンド `run_code`・`send_stdin`・`close_stdin`・`stop_run`（spec/architecture/commands/run.md）。
//!
//! 実行・テスト実行のタスクは [`crate::runner::run`]、同時に 1 つだけ行う状態は [`crate::runner::runs`] にある。
//! ここでは、受け付ける前に確かめること（ID、下書きの保存、`cargo`、初回準備の状態）と、イベントの送り先を扱う。

use std::collections::BTreeMap;
use std::sync::Arc;

use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use super::error::{CommandError, ErrorCode};
use crate::attempts::{RunMode, RunRecord, SessionKind};
use crate::content::{ContentDir, has_exercise};
use crate::environment::Environment;
use crate::runner::RunnerDir;
use crate::runner::run::{
    RUN_FINISHED, RUN_OUTPUT, RunEvents, RunFinished, RunJob, RunOutput, execute,
};
use crate::runner::runs::{ActiveRun, RunKind, Runs, not_running};
use crate::runner::workspace::WorkspaceDir;
use crate::storage::{AppDataDir, StorageError};

/// 演習画面の「実行」（`mode` が `main`）と「テスト」（`test`）。下書きを保存し、`cargo` と初回準備の状態を確かめてから、
/// 別のスレッドで実行を始めて返す。出力は `run_output`、結果は `run_finished` で送る。
///
/// 確かめる間に `cargo` の検出が終わるのを待つことがあるので、async のコマンドにして、別のスレッドで行う。
#[tauri::command(rename_all = "snake_case")]
pub async fn run_code<R: Runtime>(
    app: AppHandle<R>,
    request_id: String,
    lesson_id: String,
    mode: RunMode,
    files: BTreeMap<String, String>,
) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        start_run(&app, &request_id, &lesson_id, mode, files)
    })
    .await
    .map_err(|error| CommandError {
        code: ErrorCode::Internal,
        message: "実行を始められませんでした。".into(),
        detail: Some(error.to_string()),
    })?
}

/// 要求を始め、確かめることを確かめてから、実行のスレッドを起動する。確かめられなかったときは、要求を外して返す。
fn start_run<R: Runtime>(
    app: &AppHandle<R>,
    request_id: &str,
    lesson_id: &str,
    mode: RunMode,
    files: BTreeMap<String, String>,
) -> Result<(), CommandError> {
    let runs = app.state::<Runs>().inner().clone();
    let active = runs.begin(request_id, lesson_id, RunKind::Run)?;
    match prepare(app, &active, &runs, mode, files) {
        Ok(job) => {
            let events: Arc<dyn RunEvents> = Arc::new(AppEvents(app.clone()));
            std::thread::spawn(move || execute(job, events));
            Ok(())
        }
        Err(error) => {
            runs.release(&active);
            Err(error)
        }
    }
}

/// 下書きの保存 → `cargo` → 初回準備の状態の順に確かめ、実行のタスクを作る。
fn prepare<R: Runtime>(
    app: &AppHandle<R>,
    active: &Arc<ActiveRun>,
    runs: &Runs,
    mode: RunMode,
    files: BTreeMap<String, String>,
) -> Result<RunJob, CommandError> {
    let lesson_id = active.lesson_id();
    let content = app.state::<ContentDir>();
    let data_dir = app.state::<AppDataDir>();
    let environment = app.state::<Environment>();

    let kind = content.lesson_kind(lesson_id)?;
    let session_kind = SessionKind::for_lesson_kind(&kind)
        .filter(|_| has_exercise(&kind))
        .ok_or_else(|| CommandError {
            code: ErrorCode::InvalidArgument,
            message: "このレッスンには演習がありません。".into(),
            detail: Some(format!("レッスン {lesson_id} は演習の無いレッスン")),
        })?;
    // ファイルパスが条件を満たさないときは、ここで、何も書かずに invalid_argument になる。
    data_dir.save_draft(lesson_id, files.clone())?;
    let tools = environment.rust_tools()?;
    let dependencies = content.exercise_dependencies(lesson_id)?;
    if !dependencies.is_empty() {
        environment.ensure_crates_ready()?;
    }
    Ok(RunJob {
        active: Arc::clone(active),
        runs: runs.clone(),
        mode,
        session_kind,
        files,
        dependencies,
        cargo: tools.cargo,
        workspace: WorkspaceDir::in_app_data(&data_dir),
        runner: app.state::<RunnerDir>().inner().clone(),
    })
}

/// 実行中のプログラムの標準入力に `text`（入力欄の 1 行に改行を付けたもの）を書く。
#[tauri::command(rename_all = "snake_case")]
pub fn send_stdin(
    runs: State<'_, Runs>,
    request_id: String,
    text: String,
) -> Result<(), CommandError> {
    runs.find(&request_id)
        .ok_or_else(not_running)?
        .write_stdin(&text)
}

/// 実行中のプログラムの標準入力を閉じる（「入力終了」）。
#[tauri::command(rename_all = "snake_case")]
pub fn close_stdin(runs: State<'_, Runs>, request_id: String) -> Result<(), CommandError> {
    runs.find(&request_id)
        .ok_or_else(not_running)?
        .close_stdin()
}

/// 実行・テスト実行・提出を止める。動いていない ID のときは、何もしない。止めた結果は `run_finished` で送る。
#[tauri::command(rename_all = "snake_case")]
pub fn stop_run(runs: State<'_, Runs>, request_id: String) -> Result<(), CommandError> {
    runs.stop(&request_id);
    Ok(())
}

/// アプリを終了するときに、動いている実行・テスト実行・提出を止める。`run_finished` を送らず、記録しない。
pub fn stop_runs_on_exit<R: Runtime>(app: &AppHandle<R>) {
    if let Some(runs) = app.try_state::<Runs>() {
        runs.stop_for_app_exit();
    }
}

/// イベントを、アプリのウィンドウに `emit` で送る。試行ログは、アプリの状態の [`AppDataDir`] に書く。
struct AppEvents<R: Runtime>(AppHandle<R>);

impl<R: Runtime> RunEvents for AppEvents<R> {
    fn output(&self, output: RunOutput) {
        // 送れなくても（ウィンドウが無いなど）、実行は続ける。
        let _ = self.0.emit(RUN_OUTPUT, output);
    }

    fn finished(&self, finished: RunFinished) {
        let _ = self.0.emit(RUN_FINISHED, finished);
    }

    fn append_run(
        &self,
        lesson_id: &str,
        kind: SessionKind,
        record: RunRecord,
    ) -> Result<(), StorageError> {
        self.0
            .state::<AppDataDir>()
            .append_run(lesson_id, kind, record)
            .map(|_| ())
    }
}
