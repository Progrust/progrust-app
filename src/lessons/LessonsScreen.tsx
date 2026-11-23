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

function scrollToChapter(chapterNumber: number) {
  document
    .getElementById(`ch${chapterNumber}`)
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
  const recommendedNumber = model.recommendedChapter?.number ?? null;
  // 初期表示では、推奨レッスンのある章までスクロールする。
  useEffect(() => {
    if (recommendedNumber !== null) {
      scrollToChapter(recommendedNumber);
    }
  }, [recommendedNumber]);

  const open = (row: Extract<LessonRow, { preparing: false }>) => {
    navigate(
      row.kind === "chapter_review"
        ? { name: "exercise", lessonId: row.id }
        : { name: "video", lessonId: row.id },
    );
  };
  const jump = (chapterNumber: number) => (event: MouseEvent) => {
    event.preventDefault();
    scrollToChapter(chapterNumber);
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
              href={`#ch${model.recommendedChapter.number}`}
              onClick={jump(model.recommendedChapter.number)}
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
          {model.sections.map((section) => (
            <a
              key={section.number}
              href={`#ch${section.number}`}
              data-chapter={section.color}
              onClick={jump(section.number)}
            >
              <span className="pg-badge">{section.number}</span>
              <span>{section.title}</span>
              <IndexCount section={section} />
            </a>
          ))}
        </nav>
        <div className="lessons-chapters">
          {model.sections.map((section) => (
            <section
              key={section.number}
              id={`ch${section.number}`}
              data-chapter={section.color}
              aria-labelledby={`ch${section.number}-title`}
            >
              <div className="pg-chapter-head">
                <span className="pg-badge">{section.number}</span>
                <div>
                  <h2 id={`ch${section.number}-title`}>
                    第{section.number}章{"　"}
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
                      <LessonRowView row={row} onOpen={open} />
                    )}
                  </li>
                ))}
              </ul>
            </section>
          ))}
        </div>
      </div>
    </main>
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
