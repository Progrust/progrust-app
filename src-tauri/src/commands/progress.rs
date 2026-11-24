//! 進捗のコマンド（spec/architecture/commands/progress.md）。

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;
use tauri::{AppHandle, Manager, Runtime, State};

use super::error::{CommandError, ErrorCode};
use crate::attempts::SessionKind;
use crate::completion::PassedCode;
use crate::content::{ContentDir, has_exercise};
use crate::drafts::Draft;
use crate::progress::{LessonProgress, ProgressSummary, now_string};
use crate::runner::runs::Runs;
use crate::storage::AppDataDir;

/// 進捗と、推奨レッスンを返す。`progress.json` が無いときは、何も進んでいない状態として扱う。
/// `chapters.json` に無いレッスンの進捗は含めない。
#[tauri::command(rename_all = "snake_case")]
pub fn get_progress(
    data_dir: State<'_, AppDataDir>,
    content: State<'_, ContentDir>,
) -> Result<ProgressSummary, CommandError> {
    let chapters = content.list_chapters()?;
    let progress = data_dir.read_progress()?;
    Ok(ProgressSummary::new(&chapters, progress))
}

/// `open_lesson` の戻り値。生成問題は M6 で作るので、`generated` はいまは `null` を返す。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpenedLesson {
    /// 開始または継続した試行セッションの ID。動画のみのレッスンでは `None`。
    pub session_id: Option<String>,
    pub progress: Option<LessonProgress>,
    pub generated: Option<Value>,
    /// 下書き。無いときと、演習の無いレッスンでは `None`。
    pub draft: Option<Draft>,
}

/// レッスンを開いたことを進捗に記録し、試行セッションを始めるか続けて、更新した後の進捗と、セッションの ID と、
/// 下書きを返す。動画のみのレッスンでは、試行セッションを作らない。
#[tauri::command(rename_all = "snake_case")]
pub fn open_lesson(
    data_dir: State<'_, AppDataDir>,
    content: State<'_, ContentDir>,
    lesson_id: String,
) -> Result<OpenedLesson, CommandError> {
    let kind = content.lesson_kind(&lesson_id)?;
    let now = now_string();
    let progress =
        data_dir.update_progress(|progress| progress.open_lesson(&lesson_id, &now).clone())?;
    let session_id = match SessionKind::for_lesson_kind(&kind) {
        Some(session_kind) => Some(
            data_dir
                .start_or_continue_session(&lesson_id, session_kind)?
                .session_id,
        ),
        None => None,
    };
    let draft = if has_exercise(&kind) {
        data_dir.read_draft(&lesson_id)?
    } else {
        None
    };
    Ok(OpenedLesson {
        session_id,
        progress: Some(progress),
        generated: None,
        draft,
    })
}

/// 画面の操作（「レッスン一覧へ」「次のレッスンへ」、解説表示中の演習画面から別の画面へ移った）で、試行セッションを終える。
/// そのレッスンの実行・テスト実行が動いていれば、止めて、`stopped` の `run` イベントを試行ログに追記し終えるまで待つ。
/// その後、そのレッスンの未完了のセッションが正解済みのときだけ、下書きを消し、`ended_at` を書く。それ以外のときは何もしない。
///
/// 実行が止まって追記し終えるのを待つので、async のコマンドにして、別のスレッドで行う（画面のスレッドを止めない）。
#[tauri::command(rename_all = "snake_case")]
pub async fn end_session<R: Runtime>(
    app: AppHandle<R>,
    lesson_id: String,
) -> Result<(), CommandError> {
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<ContentDir>().lesson_kind(&lesson_id)?;
        // フロントエンドが先に stop_run を呼んでいても、止めた実行は、終える前のセッションに追記される
        // （spec/architecture/commands/progress.md「end_session」）。
        app.state::<Runs>().stop_lesson_run_and_wait(&lesson_id);
        app.state::<AppDataDir>().end_passed_session(&lesson_id)?;
        Ok(())
    })
    .await
    .map_err(|error| CommandError {
        code: ErrorCode::Internal,
        message: "試行セッションを終えられませんでした。".into(),
        detail: Some(error.to_string()),
    })?
}

/// 前回の正解コード（`last_passed_session_id` の試行ログの、正解した `submit` イベントの日時とファイル）を返す。
/// `last_passed_session_id` が無いときと、その試行ログ（正解した `submit` イベント）が無いときは `not_found`。
#[tauri::command(rename_all = "snake_case")]
pub fn get_passed_code(
    data_dir: State<'_, AppDataDir>,
    content: State<'_, ContentDir>,
    lesson_id: String,
) -> Result<PassedCode, CommandError> {
    content.lesson_kind(&lesson_id)?;
    data_dir
        .passed_code(&lesson_id)?
        .ok_or_else(|| CommandError {
            code: ErrorCode::NotFound,
            message: "前回の正解コードが見つかりません。".into(),
            detail: Some(format!("レッスン {lesson_id} の正解した提出の記録が無い")),
        })
}

/// 演習の無いレッスン（動画のみのレッスン）の下書きを扱おうとしたときのエラー。
fn no_exercise(lesson_id: &str) -> CommandError {
    CommandError {
        code: ErrorCode::InvalidArgument,
        message: "このレッスンには演習がありません。".into(),
        detail: Some(format!("レッスン {lesson_id} は演習の無いレッスン")),
    }
}

/// エディタ上の全ファイルを下書きに保存する。ファイルパスが条件を満たさないときは、何も書かずに
/// `invalid_argument` を返す。演習の無いレッスンでも `invalid_argument` を返す。
#[tauri::command(rename_all = "snake_case")]
pub fn save_draft(
    data_dir: State<'_, AppDataDir>,
    content: State<'_, ContentDir>,
    lesson_id: String,
    files: BTreeMap<String, String>,
) -> Result<(), CommandError> {
    if !has_exercise(&content.lesson_kind(&lesson_id)?) {
        return Err(no_exercise(&lesson_id));
    }
    data_dir.save_draft(&lesson_id, files)?;
    Ok(())
}

/// 下書きを教材の初期コードで上書きし、上書きした後の下書きを返す。演習の無いレッスンでは `invalid_argument` を返す。
#[tauri::command(rename_all = "snake_case")]
pub fn reset_draft(
    data_dir: State<'_, AppDataDir>,
    content: State<'_, ContentDir>,
    lesson_id: String,
) -> Result<Draft, CommandError> {
    let Some(starter_files) = content.starter_files(&lesson_id)? else {
        return Err(no_exercise(&lesson_id));
    };
    Ok(data_dir.reset_draft(&lesson_id, starter_files)?)
}

/// 動画の再生位置と、最後まで再生したかを保存する。動画のみのレッスンで初めて最後まで再生したときは、完了にする。
/// 動画の無いレッスン（章末復習レッスン）では `invalid_argument` を返す。
#[tauri::command(rename_all = "snake_case")]
pub fn save_video_position(
    data_dir: State<'_, AppDataDir>,
    content: State<'_, ContentDir>,
    lesson_id: String,
    last_sentence_id: String,
    video_watched: bool,
) -> Result<LessonProgress, CommandError> {
    let kind = content.lesson_kind(&lesson_id)?;
    if kind == "chapter_review" {
        return Err(CommandError {
            code: ErrorCode::InvalidArgument,
            message: "このレッスンには動画がありません。".into(),
            detail: Some(format!("レッスン {lesson_id} は動画の無いレッスン")),
        });
    }
    let now = now_string();
    Ok(data_dir.update_progress(|progress| {
        progress
            .save_video_position(
                &lesson_id,
                &last_sentence_id,
                video_watched,
                kind == "video_only",
                &now,
            )
            .clone()
    })?)
}
