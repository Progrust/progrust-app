// トップページ。画面の配置は design-system/ui-design/Main.dc.html、
// 決まりは documents/spec/screens/top-page.md。
import { Check, ListChecks } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import type { MouseEvent, ReactNode } from "react";
import { chapterColor, findLesson, numberChapters } from "../chapterNumbers";
import type { NumberedChapter } from "../chapterNumbers";
import { listChapters } from "../commands/content";
import type { ChapterList } from "../commands/content";
import { getDashboard } from "../commands/learningRecord";
import type { Dashboard } from "../commands/learningRecord";
import { CargoBanner, useEnvironmentStatus } from "../environment";
import { useNavigation } from "../router";
import "./home.css";

const EMPTY_ANALYSIS = "演習に正解すると、ここに分析が表示されます";
const UNREADABLE_ANALYSIS = "分析ファイルを読み込めません";

// ダッシュボードと章の並びを読み込んで、トップページを表示する。
export function HomeScreen() {
  const [data, setData] = useState<{
    dashboard: Dashboard;
    chapters: ChapterList;
  } | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let cancelled = false;
    Promise.all([getDashboard(), listChapters()]).then(
      ([dashboard, chapters]) => {
        if (!cancelled) {
          setData({ dashboard, chapters });
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
      <main className="pgx-home">
        <p role="alert">トップページを読み込めませんでした。</p>
      </main>
    );
  }
  if (data === null) {
    return <main className="pgx-home" aria-busy="true" />;
  }
  return <Home dashboard={data.dashboard} chapters={data.chapters} />;
}

export function Home({
  dashboard,
  chapters,
}: {
  dashboard: Dashboard;
  chapters: ChapterList;
}) {
  const { navigate } = useNavigation();
  const environment = useEnvironmentStatus();
  const numbered = useMemo(() => numberChapters(chapters.chapters), [chapters]);
  const next = dashboard.next_lesson;
  const found = next === null ? null : findLesson(numbered, next.lesson_id);
  const openLessons = (event: MouseEvent) => {
    event.preventDefault();
    navigate({ name: "lessons" });
  };

  let hero: ReactNode;
  if (next === null) {
    hero = (
      <section className="pg-hero pgx-hero-done" aria-label="次にやること">
        <div className="pg-emblem">
          <Check aria-hidden="true" size={36} strokeWidth={2.8} />
        </div>
        <div className="pg-hero-body">
          <p className="pg-hero-eyebrow">
            {dashboard.lesson_counts.completed} /{" "}
            {dashboard.lesson_counts.total} レッスン完了
          </p>
          <h1 className="pg-hero-title">すべてのレッスンを完了しました</h1>
          <p className="pg-hero-desc">
            苦手な要素を中心に、AI が新しい演習問題を作れます。
          </p>
        </div>
        <button
          type="button"
          className="pg-btn pg-btn--accent pg-btn--xl"
          onClick={() => navigate({ name: "generated" })}
        >
          問題を生成する
        </button>
      </section>
    );
  } else if (found === null) {
    hero = <p role="alert">次のレッスンを読み込めませんでした。</p>;
  } else {
    const { lesson, number } = found.lesson;
    const minutes =
      lesson.estimated_minutes === null
        ? ""
        : `目安 ${lesson.estimated_minutes} 分。`;
    hero = (
      <section
        className="pg-hero"
        data-chapter={found.chapter.color}
        aria-label="次のレッスン"
      >
        <div className="pg-emblem">
          <small>第{found.chapter.number}章</small>
          <b>{number}</b>
        </div>
        <div className="pg-hero-body">
          <p className="pg-hero-eyebrow">
            次のレッスン{"　"}
            {next.chapter_title}
          </p>
          <h1 className="pg-hero-title">{next.title}</h1>
          <p className="pg-hero-desc">
            {next.summary}
            {minutes}
          </p>
        </div>
        <button
          type="button"
          className="pg-btn pg-btn--accent pg-btn--xl"
          onClick={() =>
            navigate(
              lesson.kind === "chapter_review"
                ? { name: "exercise", lessonId: next.lesson_id }
                : { name: "video", lessonId: next.lesson_id },
            )
          }
        >
          {next.started ? "続きから" : "始める"}
        </button>
      </section>
    );
  }

  const analysisMessage =
    dashboard.analysis.state === "unreadable"
      ? UNREADABLE_ANALYSIS
      : EMPTY_ANALYSIS;
  const hasAnalysis = dashboard.analysis.state === "ok";

  return (
    <main className="pgx-home">
      {environment !== null && environment.cargo.status !== "found" && (
        <div className="pgx-banners">
          <CargoBanner cargo={environment.cargo} />
        </div>
      )}
      <div className="pgx-home-row">
        {hero}
        <section className="pg-panel" aria-label="総評">
          <h2>総評</h2>
          {hasAnalysis ? (
            <p>{dashboard.analysis.summary_md}</p>
          ) : (
            <div className="pg-panel-empty">
              <p>{analysisMessage}</p>
            </div>
          )}
        </section>
      </div>

      <section className="pgx-chapter-progress" aria-label="章ごとの進み具合">
        <div className="pgx-chapter-progress-head">
          <h2>章ごとの進み具合</h2>
          <span>
            <b>{dashboard.lesson_counts.completed}</b> /{" "}
            {dashboard.lesson_counts.total} レッスン完了
          </span>
        </div>
        <nav className="pg-tiles" aria-label="章の一覧">
          {dashboard.chapters.map((count) => {
            const chapter = numbered.find((n) => n.chapter.id === count.id);
            return (
              <ChapterTileView
                key={count.id}
                number={chapter?.number ?? 0}
                chapter={chapter}
                count={count}
                current={next !== null && next.chapter_id === count.id}
                onOpen={openLessons}
              />
            );
          })}
          <a className="pg-tile pg-tile--more" href="#" onClick={openLessons}>
            レッスン一覧を開く
          </a>
        </nav>
      </section>

      {!hasAnalysis && (
        <section className="pg-panel pgx-home-fill" aria-label="学習の分析">
          <h2>学習の分析</h2>
          <div className="pg-panel-empty">
            <span className="pg-empty-icon">
              <ListChecks aria-hidden="true" size={18} strokeWidth={2.2} />
            </span>
            <p>{analysisMessage}</p>
            {dashboard.analysis.state === "none" && (
              <p className="pgx-muted pgx-home-note">
                苦手な要素・復習するとよい要素・学習ペースを、AI
                が学習記録から分析します。
              </p>
            )}
          </div>
        </section>
      )}
    </main>
  );
}

function ChapterTileView({
  number,
  chapter,
  count,
  current,
  onOpen,
}: {
  number: number;
  chapter: NumberedChapter | undefined;
  count: Dashboard["chapters"][number];
  current: boolean;
  onOpen: (event: MouseEvent) => void;
}) {
  const preparing = count.total === 0;
  const percent = preparing
    ? 0
    : Math.round((count.completed / count.total) * 100);
  return (
    <a
      className="pg-tile"
      href="#"
      data-chapter={chapter?.color ?? chapterColor(number || 1)}
      aria-current={current ? "true" : undefined}
      onClick={onOpen}
    >
      <div className="pg-tile-head">
        <span className="pg-badge">{number}</span>
        <span className="pg-tile-name">{count.title}</span>
        {current && <span className="pg-tile-state">学習中</span>}
      </div>
      <div className="pg-tile-foot">
        <div className="pg-progress">
          <span style={{ width: `${percent}%` }} />
        </div>
        <span className="pg-tile-count">
          {preparing ? "準備中" : `${count.completed}/${count.total}`}
        </span>
      </div>
    </a>
  );
}
