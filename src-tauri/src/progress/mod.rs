//! 進捗（`progress.json`。spec/lesson-flow/progress.md）の型と読み書き、推奨レッスン
//! （spec/lesson-flow/order.md）の計算。

use std::collections::BTreeMap;

use chrono::{DateTime, FixedOffset};
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
    use crate::content::{Chapter, LessonSummary};

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
