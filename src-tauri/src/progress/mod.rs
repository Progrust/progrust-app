//! 進捗（`progress.json`。spec/lesson-flow/progress.md）の型と読み書き、推奨レッスン
//! （spec/lesson-flow/order.md）の計算。

use std::collections::BTreeMap;
use std::sync::{Mutex, PoisonError};

use chrono::{DateTime, FixedOffset, Local, SecondsFormat};
use serde::{Deserialize, Serialize};

use crate::content::ChapterList;
use crate::storage::{AppDataDir, StorageError};

/// `progress.json` の、`{app_data_dir}` からの相対パス。
const PROGRESS_FILE: &str = "progress.json";

/// `progress.json` の `schema_version`。
pub const SCHEMA_VERSION: u64 = 1;

/// レッスンの状態。一度も開いていないレッスンはキーごと存在しない（未着手）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LessonStatus {
    InProgress,
    Completed,
}

/// 1 つのレッスンの進捗。どのフィールドも省かずに書く（ADR 0170）。
/// 日時は、RFC 3339 の文字列（例: `2026-10-31T13:25:00+09:00`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LessonProgress {
    pub status: LessonStatus,
    pub first_completed_at: Option<String>,
    pub last_completed_at: Option<String>,
    pub completion_count: u64,
    pub last_opened_at: String,
    pub last_sentence_id: Option<String>,
    pub video_watched: bool,
    pub last_passed_session_id: Option<String>,
}

/// `progress.json` の全体。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Progress {
    pub schema_version: u64,
    pub lessons: BTreeMap<String, LessonProgress>,
}

impl Default for Progress {
    /// `progress.json` が無いときの初期状態。
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            lessons: BTreeMap::new(),
        }
    }
}

impl AppDataDir {
    /// 進捗を読む。ファイルが無いときは、何も進んでいない状態を返す（ファイルは作らない）。
    /// 読み込めないときは、退避して初期状態で作り直す。
    pub fn read_progress(&self) -> Result<Progress, StorageError> {
        Ok(self
            .read_versioned_json(PROGRESS_FILE, SCHEMA_VERSION, Progress::default)?
            .value)
    }

    /// 進捗を保存する。
    pub fn write_progress(&self, progress: &Progress) -> Result<(), StorageError> {
        self.write_json(PROGRESS_FILE, progress)
    }
}

/// 進捗の「読んで、変えて、書く」を 1 つずつ行うためのロック。
/// ファイル単位のロックは読み書きの 1 回ずつしか守らないので、更新の全体はこれで直列化する。
static UPDATE_LOCK: Mutex<()> = Mutex::new(());

/// いまの日時を、進捗に書く形（RFC 3339、秒まで。例: `2026-10-31T13:25:00+09:00`）で返す。
pub fn now_string() -> String {
    Local::now().to_rfc3339_opts(SecondsFormat::Secs, false)
}

impl Progress {
    /// レッスンを開いた。初めて開くレッスンは `in_progress` で作り、`last_opened_at` を `now` にする。
    /// 完了済みのレッスンの `status` は変えない。
    pub fn open_lesson(&mut self, lesson_id: &str, now: &str) -> &LessonProgress {
        let entry = self.entry_or_new(lesson_id, now);
        entry.last_opened_at = now.to_string();
        entry
    }

    /// 動画の再生位置を保存する。`video_watched` は、一度 `true` になったら `false` に戻さない。
    /// `video_only` のレッスン（動画のみのレッスン）で初めて `video_watched` が `true` になったときは、完了にする。
    pub fn save_video_position(
        &mut self,
        lesson_id: &str,
        last_sentence_id: &str,
        video_watched: bool,
        video_only: bool,
        now: &str,
    ) -> &LessonProgress {
        let entry = self.entry_or_new(lesson_id, now);
        entry.last_sentence_id = Some(last_sentence_id.to_string());
        if video_watched && !entry.video_watched {
            entry.video_watched = true;
            if video_only {
                entry.status = LessonStatus::Completed;
                entry.first_completed_at = Some(now.to_string());
                entry.last_completed_at = Some(now.to_string());
                entry.completion_count = 1;
            }
        }
        entry
    }

    /// 提出で正解した。レッスンを完了にし、`last_completed_at` を `at`、`last_passed_session_id` を `session_id` にして、
    /// `completion_count` を 1 増やす。`first_completed_at` は、まだ無いときだけ `at` にする（やり直しでは変えない）。
    pub fn record_pass(&mut self, lesson_id: &str, session_id: &str, at: &str) -> &LessonProgress {
        let entry = self.entry_or_new(lesson_id, at);
        entry.status = LessonStatus::Completed;
        if entry.first_completed_at.is_none() {
            entry.first_completed_at = Some(at.to_string());
        }
        entry.last_completed_at = Some(at.to_string());
        entry.completion_count += 1;
        entry.last_passed_session_id = Some(session_id.to_string());
        entry
    }

    fn entry_or_new(&mut self, lesson_id: &str, now: &str) -> &mut LessonProgress {
        self.lessons
            .entry(lesson_id.to_string())
            .or_insert_with(|| LessonProgress {
                status: LessonStatus::InProgress,
                first_completed_at: None,
                last_completed_at: None,
                completion_count: 0,
                last_opened_at: now.to_string(),
                last_sentence_id: None,
                video_watched: false,
                last_passed_session_id: None,
            })
    }
}

impl AppDataDir {
    /// 進捗を読み、`change` で変え、保存して、`change` が返した値を返す。更新は 1 つずつ行う。
    pub fn update_progress<T>(
        &self,
        change: impl FnOnce(&mut Progress) -> T,
    ) -> Result<T, StorageError> {
        let _guard = UPDATE_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
        let mut progress = self.read_progress()?;
        let result = change(&mut progress);
        self.write_progress(&progress)?;
        Ok(result)
    }
}

/// `get_progress` の戻り値。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ProgressSummary {
    /// `chapters.json` にあるレッスンの進捗だけ。
    pub lessons: BTreeMap<String, LessonProgress>,
    /// 推奨レッスンの ID。無いときは `None`。
    pub recommended_lesson_id: Option<String>,
}

impl ProgressSummary {
    /// 進捗から、`chapters.json` に無いレッスンの進捗を除き、推奨レッスンを付ける。
    pub fn new(chapters: &ChapterList, progress: Progress) -> Self {
        let recommended_lesson_id = recommended_lesson_id(chapters, &progress);
        let lessons = progress
            .lessons
            .into_iter()
            .filter(|(id, _)| {
                chapters
                    .chapters
                    .iter()
                    .any(|chapter| chapter.lessons.iter().any(|lesson| &lesson.id == id))
            })
            .collect();
        Self {
            lessons,
            recommended_lesson_id,
        }
    }
}

/// 推奨レッスン（次にやるべきレッスン）を決める。準備中のレッスンは、どの段階でも候補にしない。
///
/// 1. 完了していないレッスンのうち、最後に開いたレッスン（`last_opened_at` が最新。同じ日時なら学習順で先のもの）
/// 2. 1 が無ければ、学習順で最初の未完了レッスン
/// 3. 準備中でないレッスンをすべて完了していれば、なし
pub fn recommended_lesson_id(chapters: &ChapterList, progress: &Progress) -> Option<String> {
    // 学習順の、準備中でないレッスンの ID。
    let candidates: Vec<&str> = chapters
        .chapters
        .iter()
        .flat_map(|chapter| &chapter.lessons)
        .filter(|lesson| !lesson.in_preparation)
        .map(|lesson| lesson.id.as_str())
        .collect();

    let mut last_opened: Option<(&str, DateTime<FixedOffset>)> = None;
    for id in &candidates {
        let Some(entry) = progress.lessons.get(*id) else {
            continue;
        };
        if entry.status != LessonStatus::InProgress {
            continue;
        }
        // 日時を読めないときは、どのレッスンより前に開いたものとして扱う。
        let Ok(opened_at) = DateTime::parse_from_rfc3339(&entry.last_opened_at) else {
            continue;
        };
        if last_opened.is_none_or(|(_, latest)| opened_at > latest) {
            last_opened = Some((id, opened_at));
        }
    }
    if let Some((id, _)) = last_opened {
        return Some(id.to_string());
    }

    candidates
        .into_iter()
        .find(|id| {
            progress
                .lessons
                .get(*id)
                .is_none_or(|entry| entry.status != LessonStatus::Completed)
        })
        .map(str::to_string)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::content::{Chapter, Language, LessonSummary};

    fn lesson(id: &str, in_preparation: bool) -> LessonSummary {
        LessonSummary {
            id: id.into(),
            kind: (!in_preparation).then(|| "lesson".to_string()),
            title: None,
            summary: None,
            estimated_minutes: None,
            in_preparation,
        }
    }

    fn chapters(lessons: &[(&str, bool)]) -> ChapterList {
        ChapterList {
            chapters: vec![Chapter {
                id: "ch01".into(),
                title: "章".into(),
                description: "".into(),
                language: Language::Rust,
                number: 1,
                lessons: lessons
                    .iter()
                    .map(|(id, in_preparation)| lesson(id, *in_preparation))
                    .collect(),
            }],
        }
    }

    fn only_unprepared() -> ChapterList {
        chapters(&[("x", true)])
    }

    fn entry(status: LessonStatus, opened_at: &str) -> LessonProgress {
        let completed = status == LessonStatus::Completed;
        LessonProgress {
            status,
            first_completed_at: completed.then(|| opened_at.to_string()),
            last_completed_at: completed.then(|| opened_at.to_string()),
            completion_count: u64::from(completed),
            last_opened_at: opened_at.into(),
            last_sentence_id: None,
            video_watched: false,
            last_passed_session_id: None,
        }
    }

    fn progress(entries: &[(&str, LessonStatus, &str)]) -> Progress {
        Progress {
            schema_version: 1,
            lessons: entries
                .iter()
                .map(|(id, status, at)| (id.to_string(), entry(*status, at)))
                .collect(),
        }
    }

    use LessonStatus::{Completed, InProgress};

    #[test]
    fn nothing_started_recommends_the_first_lesson() {
        let chapters = chapters(&[("a", false), ("b", false)]);
        assert_eq!(
            recommended_lesson_id(&chapters, &Progress::default()),
            Some("a".into())
        );
    }

    #[test]
    fn with_c_chapters_first_nothing_started_recommends_the_first_c_lesson() {
        let chapter = |id: &str, language, number, lesson_id: &str| Chapter {
            id: id.into(),
            title: "章".into(),
            description: "".into(),
            language,
            number,
            lessons: vec![lesson(lesson_id, false)],
        };
        let chapters = ChapterList {
            chapters: vec![
                chapter("c01", Language::C, 1, "c01-01"),
                chapter("ch01", Language::Rust, 1, "ch01-01"),
            ],
        };
        assert_eq!(
            recommended_lesson_id(&chapters, &Progress::default()),
            Some("c01-01".into())
        );
        // c01-01 が準備中なら、次の準備中でないレッスンを推奨する。
        let mut unprepared = chapters.clone();
        unprepared.chapters[0].lessons = vec![lesson("c01-01", true)];
        assert_eq!(
            recommended_lesson_id(&unprepared, &Progress::default()),
            Some("ch01-01".into())
        );
    }

    #[test]
    fn the_most_recently_opened_unfinished_lesson_comes_first() {
        let chapters = chapters(&[("a", false), ("b", false), ("c", false)]);
        let progress = progress(&[
            ("a", InProgress, "2026-10-31T10:00:00+09:00"),
            ("c", InProgress, "2026-10-31T12:00:00+09:00"),
            ("b", Completed, "2026-10-31T13:00:00+09:00"),
        ]);
        assert_eq!(
            recommended_lesson_id(&chapters, &progress),
            Some("c".into())
        );
    }

    #[test]
    fn opened_times_are_compared_as_instants_across_offsets() {
        let chapters = chapters(&[("a", false), ("b", false)]);
        // a は 01:30Z、b は 02:00+09:00 = 前日の 17:00Z。a のほうが新しい。
        let progress = progress(&[
            ("a", InProgress, "2026-10-31T01:30:00+00:00"),
            ("b", InProgress, "2026-10-31T02:00:00+09:00"),
        ]);
        assert_eq!(
            recommended_lesson_id(&chapters, &progress),
            Some("a".into())
        );
    }

    #[test]
    fn without_an_unfinished_opened_lesson_the_first_unfinished_one_in_order() {
        let chapters = chapters(&[("a", false), ("b", false), ("c", false)]);
        let progress = progress(&[
            ("a", Completed, "2026-10-31T10:00:00+09:00"),
            ("b", Completed, "2026-10-31T11:00:00+09:00"),
        ]);
        assert_eq!(
            recommended_lesson_id(&chapters, &progress),
            Some("c".into())
        );
    }

    #[test]
    fn lessons_in_preparation_are_never_recommended() {
        let chapters = chapters(&[("a", true), ("b", false), ("c", true), ("d", false)]);
        // 準備中のレッスンの進捗があっても、候補にしない。
        let progress = progress(&[("c", InProgress, "2026-10-31T12:00:00+09:00")]);
        assert_eq!(
            recommended_lesson_id(&chapters, &progress),
            Some("b".into())
        );
        assert_eq!(
            recommended_lesson_id(&chapters, &Progress::default()),
            Some("b".into())
        );
    }

    #[test]
    fn all_prepared_lessons_completed_recommends_nothing_even_with_unprepared_left() {
        let chapters = chapters(&[("a", false), ("b", true), ("c", false), ("d", true)]);
        let progress = progress(&[
            ("a", Completed, "2026-10-31T10:00:00+09:00"),
            ("c", Completed, "2026-10-31T11:00:00+09:00"),
        ]);
        assert_eq!(recommended_lesson_id(&chapters, &progress), None);
        assert_eq!(
            recommended_lesson_id(&only_unprepared(), &Progress::default()),
            None
        );
    }

    #[test]
    fn progress_of_lessons_not_in_chapters_is_ignored() {
        let chapters = chapters(&[("a", false), ("b", false)]);
        let progress = progress(&[
            ("removed", InProgress, "2026-10-31T23:00:00+09:00"),
            ("a", Completed, "2026-10-31T10:00:00+09:00"),
        ]);
        let summary = ProgressSummary::new(&chapters, progress);
        assert_eq!(summary.recommended_lesson_id, Some("b".into()));
        assert_eq!(
            summary.lessons.keys().collect::<Vec<_>>(),
            vec![&"a".to_string()]
        );
    }

    const T1: &str = "2026-10-31T13:00:00+09:00";
    const T2: &str = "2026-11-01T09:00:00+09:00";

    #[test]
    fn opening_a_new_lesson_starts_it() {
        let mut progress = Progress::default();
        let entry = progress.open_lesson("a", T1).clone();
        assert_eq!(entry, entry_started(T1));
    }

    fn entry_started(at: &str) -> LessonProgress {
        LessonProgress {
            status: InProgress,
            first_completed_at: None,
            last_completed_at: None,
            completion_count: 0,
            last_opened_at: at.into(),
            last_sentence_id: None,
            video_watched: false,
            last_passed_session_id: None,
        }
    }

    #[test]
    fn reopening_a_completed_lesson_keeps_it_completed() {
        let mut progress = Progress::default();
        progress.save_video_position("a", "m1", true, true, T1);
        let entry = progress.open_lesson("a", T2);
        assert_eq!(entry.status, Completed);
        assert_eq!(entry.last_opened_at, T2);
        assert_eq!(entry.first_completed_at.as_deref(), Some(T1));
    }

    #[test]
    fn video_watched_never_goes_back_to_false() {
        let mut progress = Progress::default();
        progress.save_video_position("a", "m1", true, false, T1);
        let entry = progress.save_video_position("a", "m2", false, false, T2);
        assert!(entry.video_watched);
        assert_eq!(entry.last_sentence_id.as_deref(), Some("m2"));
    }

    #[test]
    fn watching_a_video_only_lesson_to_the_end_completes_it_once() {
        let mut progress = Progress::default();
        let entry = progress.save_video_position("a", "m1", false, true, T1);
        assert_eq!(entry.status, InProgress);
        assert_eq!(entry.completion_count, 0);

        let entry = progress
            .save_video_position("a", "m9", true, true, T1)
            .clone();
        assert_eq!(entry.status, Completed);
        assert_eq!(entry.first_completed_at.as_deref(), Some(T1));
        assert_eq!(entry.last_completed_at.as_deref(), Some(T1));
        assert_eq!(entry.completion_count, 1);
        assert_eq!(entry.last_passed_session_id, None);

        // 再生し直しても、完了の項目は変わらない。
        let entry = progress.save_video_position("a", "m3", true, true, T2);
        assert_eq!(entry.first_completed_at.as_deref(), Some(T1));
        assert_eq!(entry.last_completed_at.as_deref(), Some(T1));
        assert_eq!(entry.completion_count, 1);
        let entry = progress.save_video_position("a", "m3", false, true, T2);
        assert_eq!(entry.status, Completed);
    }

    #[test]
    fn watching_a_lesson_with_an_exercise_does_not_complete_it() {
        let mut progress = Progress::default();
        let entry = progress.save_video_position("a", "m9", true, false, T1);
        assert!(entry.video_watched);
        assert_eq!(entry.status, InProgress);
        assert_eq!(entry.completion_count, 0);
    }

    #[test]
    fn passing_completes_the_lesson() {
        let mut progress = Progress::default();
        progress.open_lesson("a", T1);
        let entry = progress.record_pass("a", "20261031T130000_a", T1).clone();
        assert_eq!(
            entry,
            LessonProgress {
                status: Completed,
                first_completed_at: Some(T1.into()),
                last_completed_at: Some(T1.into()),
                completion_count: 1,
                last_opened_at: T1.into(),
                last_sentence_id: None,
                video_watched: false,
                last_passed_session_id: Some("20261031T130000_a".into()),
            }
        );
    }

    #[test]
    fn passing_a_completed_lesson_again_counts_up_and_keeps_the_first_completion() {
        let mut progress = Progress::default();
        progress.open_lesson("a", T1);
        progress.record_pass("a", "20261031T130000_a", T1);
        progress.open_lesson("a", T2);
        let entry = progress.record_pass("a", "20261101T090000_a", T2);
        assert_eq!(entry.status, Completed);
        assert_eq!(entry.completion_count, 2);
        assert_eq!(entry.first_completed_at.as_deref(), Some(T1));
        assert_eq!(entry.last_completed_at.as_deref(), Some(T2));
        assert_eq!(
            entry.last_passed_session_id.as_deref(),
            Some("20261101T090000_a")
        );
    }

    #[test]
    fn all_fields_are_written_with_null_for_undecided_ones() {
        let progress = Progress {
            schema_version: 1,
            lessons: BTreeMap::from([(
                "ch01-01".to_string(),
                LessonProgress {
                    status: InProgress,
                    first_completed_at: None,
                    last_completed_at: None,
                    completion_count: 0,
                    last_opened_at: "2026-10-31T13:02:00+09:00".into(),
                    last_sentence_id: None,
                    video_watched: false,
                    last_passed_session_id: None,
                },
            )]),
        };
        assert_eq!(
            serde_json::to_value(progress).unwrap(),
            json!({
                "schema_version": 1,
                "lessons": {
                    "ch01-01": {
                        "status": "in_progress",
                        "first_completed_at": null,
                        "last_completed_at": null,
                        "completion_count": 0,
                        "last_opened_at": "2026-10-31T13:02:00+09:00",
                        "last_sentence_id": null,
                        "video_watched": false,
                        "last_passed_session_id": null
                    }
                }
            })
        );
    }
}
