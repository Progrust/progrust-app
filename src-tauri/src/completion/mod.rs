//! 提出で正解したときのレッスンの完了（spec/lesson-flow/completion.md）と、正解した試行セッションの終了
//! （spec/learning-record-and-analysis/attempt-log.md「試行セッションのライフサイクル」）、前回の正解コードの取り出し
//! （spec/lesson-flow/drafts.md「前回の正解コードを見る」）。
//!
//! - 正解の記録は [`AppDataDir::record_pass`]。提出のコマンドが、`submit` イベントを追記した後に呼ぶ。
//! - セッションの終了は、画面の操作（`end_session`）では [`AppDataDir::end_passed_session`]、
//!   アプリの終了と起動時の救済では [`AppDataDir::end_all_passed_sessions`]。

use std::collections::BTreeMap;

use serde::Serialize;

use crate::attempts::AttemptLog;
use crate::progress::{LessonProgress, now_string};
use crate::storage::{AppDataDir, StorageError};

/// `get_passed_code` の戻り値。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PassedCode {
    /// 最後に正解した試行セッションの ID（`last_passed_session_id`）。
    pub session_id: String,
    /// 正解した `submit` イベントの `at`。
    pub submitted_at: String,
    /// 正解した `submit` イベントのファイル。
    pub files: BTreeMap<String, String>,
}

/// [`AppDataDir::end_all_passed_sessions`] の結果。
#[derive(Debug, Default)]
pub struct EndedSessions {
    /// 終えたセッションの ID。
    pub ended: Vec<String>,
    /// 終えられなかったときの失敗。1 つのセッションで失敗しても、残りのセッションは終える。
    pub errors: Vec<StorageError>,
}

impl AppDataDir {
    /// 提出で正解したときに呼ぶ。試行ログ `session_id` に `passed_at` を `at` で書き（既にあれば最初の日時を残す）、
    /// `progress.json` のレッスン `lesson_id` を完了にして、更新した後の進捗を返す。
    /// 試行ログが無いときは、どちらも書かずに `None` を返す。
    pub fn record_pass(
        &self,
        lesson_id: &str,
        session_id: &str,
        at: &str,
    ) -> Result<Option<LessonProgress>, StorageError> {
        if self.mark_session_passed(session_id, at)?.is_none() {
            return Ok(None);
        }
        let progress = self
            .update_progress(|progress| progress.record_pass(lesson_id, session_id, at).clone())?;
        Ok(Some(progress))
    }

    /// 画面の操作（`end_session`）で呼ぶ。レッスン `lesson_id` の未完了のセッションが正解済みなら終え
    /// （そのレッスンの下書きを消し、`ended_at` を書く）、終えたセッションの ID を返す。
    /// 正解していないときと、未完了のセッションが無いときは、何もせずに `None` を返す。
    pub fn end_passed_session(&self, lesson_id: &str) -> Result<Option<String>, StorageError> {
        let now = now_string();
        let ended =
            self.end_passed_session_of_lesson(lesson_id, &now, |log| self.before_ending(log))?;
        Ok(ended.map(after_ending))
    }

    /// アプリの終了のときと、アプリの起動のとき（異常終了の救済）に呼ぶ。正解済みで終わっていないセッションを、
    /// すべて終える（下書きを消し、`ended_at` を書く）。
    pub fn end_all_passed_sessions(&self) -> EndedSessions {
        let mut result = EndedSessions::default();
        let sessions = match self.unfinished_sessions() {
            Ok(sessions) => sessions,
            Err(error) => {
                result.errors.push(error);
                return result;
            }
        };
        let now = now_string();
        for session in sessions.iter().filter(|log| log.passed_at.is_some()) {
            match self
                .end_passed_session_by_id(&session.session_id, &now, |log| self.before_ending(log))
            {
                Ok(Some(log)) => result.ended.push(after_ending(log)),
                Ok(None) => {}
                Err(error) => result.errors.push(error),
            }
        }
        result
    }

    /// 前回の正解コード（`last_passed_session_id` の試行ログの、正解した `submit` イベント）を返す。
    /// `last_passed_session_id` が無いとき、その試行ログが無いとき、試行ログに正解した `submit` イベントが無いときは `None`。
    pub fn passed_code(&self, lesson_id: &str) -> Result<Option<PassedCode>, StorageError> {
        let progress = self.read_progress()?;
        let Some(session_id) = progress
            .lessons
            .get(lesson_id)
            .and_then(|entry| entry.last_passed_session_id.clone())
        else {
            return Ok(None);
        };
        let Some(log) = self.read_attempt_log(&session_id)? else {
            return Ok(None);
        };
        Ok(log.passed_submit().map(|submit| PassedCode {
            session_id: session_id.clone(),
            submitted_at: submit.at.clone(),
            files: submit.files.clone(),
        }))
    }

    /// セッションの `ended_at` を書く直前に行うこと。下書きを、`ended_at` より先に消す。
    /// 消した後、`ended_at` を書く前にアプリが止まっても、次の起動時の救済で、もう一度終える。
    /// 逆の順では、`ended_at` を書いた後に止まると、下書きが残り続ける。
    fn before_ending(&self, log: &AttemptLog) -> Result<(), StorageError> {
        self.delete_draft(&log.lesson_id)
    }
}

/// セッションの `ended_at` を書いた後に行うこと。終えたセッションの ID を返す。
fn after_ending(log: AttemptLog) -> String {
    // M5: ここで、終えたセッション `log` の記録ジョブを登録する（spec/learning-record-and-analysis/attempt-log.md
    // 「試行セッションのライフサイクル」）。`ended_at` を書く前に登録するか後に登録するかで、途中でアプリが止まったときに
    // 登録が重なるか抜けるかが変わるので、M5 で決める。M5 より前に終えたセッションには、記録ジョブが登録されていない。
    log.session_id
}
