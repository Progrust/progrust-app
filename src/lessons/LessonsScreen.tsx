// レッスン一覧画面。画面の配置は design-system/ui-design/Lessons.dc.html、
// 決まりは documents/spec/screens/lesson-list.md。
import { Check, Clock } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { MouseEvent } from "react";
import { numberChapters } from "../chapterNumbers";
import { listChapters } from "../commands/content";
import type { ChapterList } from "../commands/content";
import { getProgress } from "../commands/progress";
import type { ProgressSummary } from "../commands/progress";
import { useNavigation } from "../router";
import { buildLessonList } from "./lessonList";
import type { ChapterSection, LessonRow } from "./lessonList";
import "./lessons.css";

const STATE_LABEL = { todo: "未着手", doing: "進行中", done: "完了" } as const;

// 章の区切りの要素の ID。章の番号は言語ごとに 1 から始まるので、章 ID で決める。
function chapterAnchor(chapterId: string): string {
  return `chapter-${chapterId}`;
}

function scrollToChapter(chapterId: string) {
  document
    .getElementById(chapterAnchor(chapterId))
    ?.scrollIntoView?.({ block: "start" });
}

// 章と進捗を読み込んで、レッスン一覧を表示する。
export function LessonsScreen() {
  const [data, setData] = useState<{
    chapters: ChapterList;
    progress: ProgressSummary;
  } | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let cancelled = false;
    Promise.all([listChapters(), getProgress()]).then(
      ([chapters, progress]) => {
        if (!cancelled) {
          setData({ chapters, progress });
        }
      },
      () => {
        if (!cancelled) {
          setFailed(true);
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, []);
  if (failed) {
    return (
      <main className="lessons-wrap">
        <p role="alert">レッスンの一覧を読み込めませんでした。</p>
      </main>
    );
  }
  if (data === null) {
    return <main className="lessons-wrap" aria-busy="true" />;
  }
  return <LessonList chapters={data.chapters} progress={data.progress} />;
}

export function LessonList({
  chapters,
  progress,
}: {
  chapters: ChapterList;
  progress: ProgressSummary;
}) {
  const { navigate } = useNavigation();
  const model = useMemo(
    () => buildLessonList(numberChapters(chapters.chapters), progress),
    [chapters, progress],
  );
  const recommendedChapterId = model.recommendedChapter?.id ?? null;
  // 初期表示では、推奨レッスンのある章までスクロールする。
  useEffect(() => {
    if (recommendedChapterId !== null) {
      scrollToChapter(recommendedChapterId);
    }
  }, [recommendedChapterId]);

  const open = (row: Extract<LessonRow, { preparing: false }>) => {
    navigate(
      row.kind === "chapter_review"
        ? { name: "exercise", lessonId: row.id }
        : { name: "video", lessonId: row.id },
    );
  };
  const jump = (chapterId: string) => (event: MouseEvent) => {
    event.preventDefault();
    scrollToChapter(chapterId);
  };

  return (
    <main className="lessons-wrap">
      <div className="lessons-head">
        <div>
          <h1>レッスン</h1>
          <p>
            <b>{model.done}</b> / {model.total}{" "}
            レッスン完了。どのレッスンからでも始められます。
          </p>
        </div>
        {model.recommendedChapter !== null && model.recommendedRow !== null && (
          <div
            className="lessons-head-end"
            data-chapter={model.recommendedChapter.color}
          >
            <a
              className="pg-btn pg-btn--sm"
              href={`#${chapterAnchor(model.recommendedChapter.id)}`}
              onClick={jump(model.recommendedChapter.id)}
            >
              <span className="pg-next">次はここ</span>
              {model.recommendedRow.number} {model.recommendedRow.title}
            </a>
          </div>
        )}
      </div>
      <div className="lessons-body">
        <nav className="pg-index" aria-label="章">
          <p className="pg-index-title">章</p>
          {model.parts.map((part) => (
            <div key={part.language} className="lessons-index-part">
              <p className="subheading lessons-index-part-title">
                {part.title}
              </p>
              {part.sections.map((section) => (
                <a
                  key={section.id}
                  href={`#${chapterAnchor(section.id)}`}
                  data-chapter={section.color}
                  onClick={jump(section.id)}
                >
                  <span className="pg-badge">{section.badge}</span>
                  <span>{section.title}</span>
                  <IndexCount section={section} />
                </a>
              ))}
            </div>
          ))}
        </nav>
        <div className="lessons-chapters">
          {model.parts.map((part) => (
            <section
              key={part.language}
              className="lessons-part"
              aria-labelledby={`part-${part.language}-title`}
            >
              <h2
                id={`part-${part.language}-title`}
                className="heading lessons-part-title"
              >
                {part.title}
              </h2>
              <div className="lessons-part-chapters">
                {part.sections.map((section) => (
                  <ChapterSectionView
                    key={section.id}
                    section={section}
                    onOpen={open}
                  />
                ))}
              </div>
            </section>
          ))}
        </div>
      </div>
    </main>
  );
}

function ChapterSectionView({
  section,
  onOpen,
}: {
  section: ChapterSection;
  onOpen: (row: Extract<LessonRow, { preparing: false }>) => void;
}) {
  const anchor = chapterAnchor(section.id);
  return (
    <section
      id={anchor}
      data-chapter={section.color}
      aria-labelledby={`${anchor}-title`}
    >
      <div className="pg-chapter-head">
        <span className="pg-badge">{section.badge}</span>
        <div>
          <h2 id={`${anchor}-title`}>
            {section.label}
            {"　"}
            {section.title}
          </h2>
          <p>{section.description}</p>
        </div>
        <span className="pg-chapter-count">
          {section.total === 0 ? (
            "準備中"
          ) : (
            <>
              <b>{section.done}</b> / {section.total} 完了
            </>
          )}
        </span>
      </div>
      <ul className="pg-lessons">
        {section.rows.map((row) => (
          <li key={row.id}>
            {row.preparing ? (
              <PreparingRow row={row} />
            ) : (
              <LessonRowView row={row} onOpen={onOpen} />
            )}
          </li>
        ))}
      </ul>
    </section>
  );
}

function IndexCount({ section }: { section: ChapterSection }) {
  if (section.total === 0) {
    return <small>準備中</small>;
  }
  if (section.done === section.total) {
    return <Check aria-label="完了" role="img" size={14} strokeWidth={2.6} />;
  }
  return (
    <small>
      {section.done}/{section.total}
    </small>
  );
}

// 準備中のレッスンの行。番号と「準備中」だけを表示し、選んでも反応しない。
function PreparingRow({
  row,
}: {
  row: Extract<LessonRow, { preparing: true }>;
}) {
  return (
    <div className="pg-lesson lessons-row-preparing">
      <span className="pg-lesson-no">{row.number}</span>
      <div>
        <p className="pg-lesson-name">
          <span className="pg-tag">準備中</span>
        </p>
      </div>
    </div>
  );
}

function LessonRowView({
  row,
  onOpen,
}: {
  row: Extract<LessonRow, { preparing: false }>;
  onOpen: (row: Extract<LessonRow, { preparing: false }>) => void;
}) {
  return (
    <a
      className="pg-lesson"
      href="#"
      aria-current={row.recommended ? "true" : undefined}
      onClick={(event) => {
        event.preventDefault();
        onOpen(row);
      }}
    >
      <span className="pg-lesson-no">{row.number}</span>
      <div>
        <p className="pg-lesson-name">
          {row.title}
          {row.recommended && <span className="pg-next">次はここ</span>}
          {row.kind === "chapter_review" && (
            <span className="pg-tag">復習</span>
          )}
          {row.kind === "video_only" && (
            <span className="pg-tag">動画のみ</span>
          )}
        </p>
        <p className="pg-lesson-desc">{row.summary}</p>
      </div>
      <span className="pg-lesson-time">
        <Clock aria-hidden="true" size={13} strokeWidth={2.4} />
        {row.minutes === null ? "" : `${row.minutes} 分`}
      </span>
      <span className={`pg-status pg-status--${row.state}`}>
        {row.state === "done" && (
          <Check aria-hidden="true" size={12} strokeWidth={3} />
        )}
        {STATE_LABEL[row.state]}
      </span>
    </a>
  );
}
