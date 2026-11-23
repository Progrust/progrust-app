//! トップページ（spec/screens/top-page.md）に渡す値 `get_dashboard`
//! （spec/architecture/commands/learning-record.md）の計算。
//!
//! 分析に関する項目（`analysis`）は M5 で作る。それまでは、学習記録がない状態（`none`）を返す。

use serde::Serialize;

use crate::content::{Chapter, ChapterList, LessonSummary};
use crate::progress::{LessonStatus, Progress, recommended_lesson_id};

/// 章ごとの完了率に表示する章の数。
const CHAPTER_TILES: usize = 9;

/// `get_dashboard` の戻り値。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Dashboard {
    pub next_lesson: Option<NextLesson>,
    pub lesson_counts: LessonCounts,
    pub chapters: Vec<ChapterCount>,
    pub analysis: Analysis,
}

/// 次にやるべきレッスン。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct NextLesson {
    pub lesson_id: String,
    pub chapter_id: String,
    pub chapter_title: String,
    pub title: String,
    pub summary: String,
    /// 進捗があれば `true`。`false` は未着手。
    pub started: bool,
}

/// 完了したレッスンの数と、全レッスンの数。どちらも準備中のレッスンを数えない。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct LessonCounts {
    pub completed: usize,
    pub total: usize,
}

/// 章ごとの完了数。準備中のレッスンを数えない。準備中でないレッスンが無い章は `total` が 0 になる。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChapterCount {
    pub id: String,
    pub title: String,
    pub completed: usize,
    pub total: usize,
}

/// 分析に関する項目。M5 までは、常に `state` が `none` で、ほかは空にする。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Analysis {
    pub state: &'static str,
    pub weak_elements: Vec<serde_json::Value>,
    pub review_due_elements: Vec<serde_json::Value>,
    pub regressed_elements: Vec<serde_json::Value>,
    pub last_4_weeks: Vec<u64>,
    pub summary_md: Option<String>,
}

impl Analysis {
    fn none() -> Self {
        Self {
            state: "none",
            weak_elements: Vec::new(),
            review_due_elements: Vec::new(),
            regressed_elements: Vec::new(),
            last_4_weeks: Vec::new(),
            summary_md: None,
        }
    }
}

impl Dashboard {
    pub fn new(chapters: &ChapterList, progress: &Progress) -> Self {
        let counts: Vec<ChapterCount> = chapters
            .chapters
            .iter()
            .map(|chapter| chapter_count(chapter, progress))
            .collect();
        let lesson_counts = LessonCounts {
            completed: counts.iter().map(|count| count.completed).sum(),
            total: counts.iter().map(|count| count.total).sum(),
        };

        let next = recommended_lesson_id(chapters, progress).and_then(|id| {
            chapters
                .chapters
                .iter()
                .enumerate()
                .find_map(|(index, chapter)| {
                    let lesson = chapter.lessons.iter().find(|lesson| lesson.id == id)?;
                    Some((index, next_lesson(chapter, lesson, progress)))
                })
        });
        let next_chapter_index = next.as_ref().map(|(index, _)| *index);

        let (start, end) = visible_chapters(counts.len(), next_chapter_index);
        Self {
            next_lesson: next.map(|(_, lesson)| lesson),
            lesson_counts,
            chapters: counts[start..end].to_vec(),
            analysis: Analysis::none(),
        }
    }
}

fn next_lesson(chapter: &Chapter, lesson: &LessonSummary, progress: &Progress) -> NextLesson {
    NextLesson {
        lesson_id: lesson.id.clone(),
        chapter_id: chapter.id.clone(),
        chapter_title: chapter.title.clone(),
        title: lesson.title.clone().unwrap_or_default(),
        summary: lesson.summary.clone().unwrap_or_default(),
        started: progress.lessons.contains_key(&lesson.id),
    }
}

fn chapter_count(chapter: &Chapter, progress: &Progress) -> ChapterCount {
    let prepared = chapter
        .lessons
        .iter()
        .filter(|lesson| !lesson.in_preparation);
    let (mut completed, mut total) = (0, 0);
    for lesson in prepared {
        total += 1;
        if progress
            .lessons
            .get(&lesson.id)
            .is_some_and(|entry| entry.status == LessonStatus::Completed)
        {
            completed += 1;
        }
    }
    ChapterCount {
        id: chapter.id.clone(),
        title: chapter.title.clone(),
        completed,
        total,
    }
}

/// 表示する章の範囲（`start..end`）を返す。
/// 次にやるべきレッスンの章の 2 つ前の章から続く 9 章。はみ出す分は反対側にずらす。
/// 次にやるべきレッスンが無ければ、最後の 9 章。章が 9 章以下なら、すべての章。
fn visible_chapters(count: usize, next_chapter_index: Option<usize>) -> (usize, usize) {
    let last_start = count.saturating_sub(CHAPTER_TILES);
    let start = match next_chapter_index {
        Some(index) => index.saturating_sub(2).min(last_start),
        None => last_start,
    };
    (start, count.min(start + CHAPTER_TILES))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::progress::LessonProgress;

    fn lesson(id: &str, in_preparation: bool) -> LessonSummary {
        LessonSummary {
            id: id.into(),
            kind: (!in_preparation).then(|| "lesson".to_string()),
            title: (!in_preparation).then(|| format!("{id} のタイトル")),
            summary: (!in_preparation).then(|| format!("{id} の概要")),
            estimated_minutes: None,
            in_preparation,
        }
    }

    fn chapter(index: usize, lessons: &[(&str, bool)]) -> Chapter {
        Chapter {
            id: format!("ch{index:02}"),
            title: format!("第{index}章"),
            description: String::new(),
            lessons: lessons.iter().map(|(id, prep)| lesson(id, *prep)).collect(),
        }
    }

    /// 1 章に 1 レッスンずつの `n` 章。レッスン ID は `l01`、`l02`、…。
    fn many_chapters(n: usize) -> ChapterList {
        ChapterList {
            chapters: (1..=n)
                .map(|i| chapter(i, &[(format!("l{i:02}").as_str(), false)]))
                .collect(),
        }
    }

    fn entry(status: LessonStatus, at: &str) -> LessonProgress {
        let completed = status == LessonStatus::Completed;
        LessonProgress {
            status,
            first_completed_at: completed.then(|| at.to_string()),
            last_completed_at: completed.then(|| at.to_string()),
            completion_count: u64::from(completed),
            last_opened_at: at.into(),
            last_sentence_id: None,
            video_watched: false,
            last_passed_session_id: None,
        }
    }

    fn progress(entries: &[(&str, LessonStatus)]) -> Progress {
        Progress {
            schema_version: 1,
            lessons: entries
                .iter()
                .map(|(id, status)| (id.to_string(), entry(*status, "2026-10-31T10:00:00+09:00")))
                .collect(),
        }
    }

    fn chapter_ids(dashboard: &Dashboard) -> Vec<&str> {
        dashboard.chapters.iter().map(|c| c.id.as_str()).collect()
    }

    fn ids(from: usize, to: usize) -> Vec<String> {
        (from..=to).map(|i| format!("ch{i:02}")).collect()
    }

    #[test]
    fn nine_chapters_start_two_before_the_next_lessons_chapter() {
        let chapters = many_chapters(23);
        // 第1〜3章を完了した。次にやるべきレッスンは第4章にある。
        let progress = progress(&[
            ("l01", LessonStatus::Completed),
            ("l02", LessonStatus::Completed),
            ("l03", LessonStatus::Completed),
        ]);
        let dashboard = Dashboard::new(&chapters, &progress);
        assert_eq!(dashboard.next_lesson.as_ref().unwrap().chapter_id, "ch04");
        assert_eq!(chapter_ids(&dashboard), ids(2, 10));
    }

    #[test]
    fn chapters_that_overflow_are_shifted_to_the_other_side() {
        let chapters = many_chapters(23);
        // 第1章が次: 第1〜9章。
        let dashboard = Dashboard::new(&chapters, &Progress::default());
        assert_eq!(chapter_ids(&dashboard), ids(1, 9));
        // 第2章が次: 第1〜9章（2 つ前は存在しないので、前にずらす）。
        let dashboard = Dashboard::new(&chapters, &progress(&[("l01", LessonStatus::Completed)]));
        assert_eq!(chapter_ids(&dashboard), ids(1, 9));
        // 第23章が次: 第15〜23章。
        let all_but_last: Vec<(String, LessonStatus)> = (1..=22)
            .map(|i| (format!("l{i:02}"), LessonStatus::Completed))
            .collect();
        let refs: Vec<(&str, LessonStatus)> = all_but_last
            .iter()
            .map(|(id, s)| (id.as_str(), *s))
            .collect();
        let dashboard = Dashboard::new(&chapters, &progress(&refs));
        assert_eq!(dashboard.next_lesson.as_ref().unwrap().chapter_id, "ch23");
        assert_eq!(chapter_ids(&dashboard), ids(15, 23));
    }

    #[test]
    fn without_a_next_lesson_the_last_nine_chapters_are_shown() {
        let chapters = many_chapters(23);
        let done: Vec<(String, LessonStatus)> = (1..=23)
            .map(|i| (format!("l{i:02}"), LessonStatus::Completed))
            .collect();
        let refs: Vec<(&str, LessonStatus)> =
            done.iter().map(|(id, s)| (id.as_str(), *s)).collect();
        let dashboard = Dashboard::new(&chapters, &progress(&refs));
        assert_eq!(dashboard.next_lesson, None);
        assert_eq!(chapter_ids(&dashboard), ids(15, 23));
    }

    #[test]
    fn nine_or_fewer_chapters_are_all_shown() {
        let dashboard = Dashboard::new(&many_chapters(9), &Progress::default());
        assert_eq!(chapter_ids(&dashboard), ids(1, 9));
        let dashboard = Dashboard::new(&many_chapters(3), &Progress::default());
        assert_eq!(chapter_ids(&dashboard), ids(1, 3));
        let dashboard = Dashboard::new(&many_chapters(0), &Progress::default());
        assert!(dashboard.chapters.is_empty());
        assert_eq!(dashboard.next_lesson, None);
    }

    #[test]
    fn lessons_in_preparation_are_not_counted() {
        // 7 レッスンのうち 2 レッスンが準備中。3 レッスンを完了した。
        let chapters = ChapterList {
            chapters: vec![chapter(
                1,
                &[
                    ("a", false),
                    ("b", false),
                    ("c", false),
                    ("d", false),
                    ("e", false),
                    ("p", true),
                    ("q", true),
                ],
            )],
        };
        let progress = progress(&[
            ("a", LessonStatus::Completed),
            ("b", LessonStatus::Completed),
            ("c", LessonStatus::Completed),
        ]);
        let dashboard = Dashboard::new(&chapters, &progress);
        assert_eq!(dashboard.chapters[0].completed, 3);
        assert_eq!(dashboard.chapters[0].total, 5);
        assert_eq!(
            dashboard.lesson_counts,
            LessonCounts {
                completed: 3,
                total: 5
            }
        );
    }

    #[test]
    fn a_chapter_with_only_unprepared_lessons_has_a_zero_total() {
        let chapters = ChapterList {
            chapters: vec![chapter(1, &[("p", true)]), chapter(2, &[("a", false)])],
        };
        let dashboard = Dashboard::new(&chapters, &Progress::default());
        assert_eq!(dashboard.chapters[0].total, 0);
        assert_eq!(dashboard.chapters[0].completed, 0);
        assert_eq!(dashboard.lesson_counts.total, 1);
    }

    #[test]
    fn the_next_lesson_carries_its_chapter_and_whether_it_was_started() {
        let chapters = many_chapters(3);
        let dashboard = Dashboard::new(&chapters, &Progress::default());
        assert_eq!(
            dashboard.next_lesson,
            Some(NextLesson {
                lesson_id: "l01".into(),
                chapter_id: "ch01".into(),
                chapter_title: "第1章".into(),
                title: "l01 のタイトル".into(),
                summary: "l01 の概要".into(),
                started: false,
            })
        );
        let dashboard = Dashboard::new(&chapters, &progress(&[("l01", LessonStatus::InProgress)]));
        assert!(dashboard.next_lesson.unwrap().started);
    }

    #[test]
    fn analysis_is_always_none_until_m5() {
        let dashboard = Dashboard::new(&many_chapters(1), &Progress::default());
        assert_eq!(
            serde_json::to_value(&dashboard.analysis).unwrap(),
            serde_json::json!({
                "state": "none",
                "weak_elements": [],
                "review_due_elements": [],
                "regressed_elements": [],
                "last_4_weeks": [],
                "summary_md": null
            })
        );
    }
}
