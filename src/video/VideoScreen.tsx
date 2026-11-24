// 動画画面。画面の配置は design-system/ui-design/Video.dc.html、
// 決まりは documents/spec/screens/video-screen.md、documents/spec/lesson-flow/progress.md、flow.md。
// 教材と進捗と設定を読み込み、動画のプレーヤー（VideoPlayer）に渡す。
// 再生位置は、保存のきっかけ（videoScreenLogic.ts）で `save_video_position` に保存し、
// 音量・ミュート・再生速度・字幕は `update_settings` に保存する。
import { loadLessonVideo } from "virtual:progrust-lesson-videos";
import { ChevronLeft } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { MouseEvent } from "react";
import { findLesson, numberChapters } from "../chapterNumbers";
import { getLesson, listChapters } from "../commands/content";
import type { Chapter, LessonDetail, LessonKind } from "../commands/content";
import { openLesson, saveVideoPosition } from "../commands/progress";
import type { LessonProgress } from "../commands/progress";
import { getSettings, updateSettings } from "../commands/settings";
import type { Settings } from "../commands/settings";
import { lessonRoute, nextLesson } from "../lessons/nextLesson";
import { useNavigation } from "../router";
import { currentSentence, sentenceStarts } from "./playerMath";
import { computeTimeline } from "./timeline";
import {
  initialFrame as initialFrameOf,
  rightAction,
  savePositionArgs,
} from "./videoScreenLogic";
import type { SaveTrigger } from "./videoScreenLogic";
import { VideoPlayer } from "./VideoPlayer";
import type { VideoLoad, VideoSettings } from "./VideoPlayer";
import "./videoScreen.css";

// 設定を読み込めなかったときに使う値（documents/spec/screens/settings-screen.md「settings.json」の既定値）。
const DEFAULT_SETTINGS: VideoSettings = {
  volume: 1,
  muted: false,
  playbackRate: 1,
  captions: false,
};

// 読み込んだもの。
type Loaded = {
  chapters: Chapter[];
  detail: LessonDetail;
  progress: LessonProgress | null;
  settings: Settings;
  video: Extract<VideoLoad, { state: "ready" }>["video"];
  // 開いたときの再生位置（フレーム）と、その位置の文の ID。
  initialFrame: number;
  initialSentenceId: string | undefined;
};

// 読み込みの途中や失敗のあとでも、分かっているもの。文脈の行と、右端の行動（「演習へ」など）を、
// 動画の読み込みの状態によらずに決めるために、動画のコードとは別に持つ。
type Known = {
  chapters: Chapter[] | null;
  progress: LessonProgress | null | undefined;
  kind: LessonKind | null;
};

const NOTHING_KNOWN: Known = {
  chapters: null,
  progress: undefined,
  kind: null,
};

type State =
  | { state: "loading" }
  | { state: "failed" }
  | { state: "ready"; loaded: Loaded };

function toVideoSettings(settings: Settings): VideoSettings {
  return {
    volume: settings.video.volume,
    muted: settings.video.muted,
    playbackRate: settings.video.playback_rate,
    captions: settings.video.captions,
  };
}

// 教材・進捗・設定・動画のコードを読み込む。`open_lesson` は、画面を開いたときに 1 回だけ呼ぶ
// （「もう一度読み込む」では、呼んでいない場合だけ呼ぶ）。
async function load(
  lessonId: string,
  opened: { current: Promise<LessonProgress | null> | null },
  onKnown: (patch: Partial<Known>) => void,
): Promise<Loaded> {
  const chaptersPromise = listChapters().then(({ chapters }) => {
    onKnown({ chapters });
    return chapters;
  });
  if (opened.current === null) {
    const promise = openLesson(lessonId).then((o) => o.progress);
    // 失敗したときは、次の読み込みで呼び直す。
    promise.catch(() => {
      if (opened.current === promise) {
        opened.current = null;
      }
    });
    opened.current = promise;
  }
  const progressPromise = opened.current.then((progress) => {
    onKnown({ progress });
    return progress;
  });
  const detailPromise = getLesson(lessonId).then((detail) => {
    onKnown({ kind: detail.lesson.kind });
    return detail;
  });
  const [chapters, detail, progress, settings, code] = await Promise.all([
    chaptersPromise,
    detailPromise,
    progressPromise,
    getSettings(),
    loadLessonVideo(lessonId),
  ]);
  if (
    detail.script === null ||
    detail.voice_dir === null ||
    detail.lesson.kind === "chapter_review" ||
    detail.lesson.kind === "generated"
  ) {
    throw new Error(`レッスン ${lessonId} に動画が無い`);
  }
  const video = {
    script: detail.script,
    durations: code.durations,
    cueReference: code.cueReference,
    scenes: code.scenes,
    voiceDir: detail.voice_dir,
    chapters,
    lesson: detail.lesson,
  };
  const timeline = computeTimeline(
    video.script,
    video.durations,
    video.cueReference,
  );
  const initialFrame = initialFrameOf(
    timeline,
    progress?.last_sentence_id ?? null,
  );
  return {
    chapters,
    detail,
    progress,
    settings,
    video,
    initialFrame,
    initialSentenceId: currentSentence(sentenceStarts(timeline), initialFrame)
      ?.id,
  };
}

export function VideoScreen({ lessonId }: { lessonId: string }) {
  const [attempt, setAttempt] = useState(0);
  const [state, setState] = useState<State>({ state: "loading" });
  const [known, setKnown] = useState(NOTHING_KNOWN);
  const opened = useRef<Promise<LessonProgress | null> | null>(null);

  useEffect(() => {
    let cancelled = false;
    load(lessonId, opened, (patch) => {
      if (!cancelled) {
        setKnown((k) => ({ ...k, ...patch }));
      }
    }).then(
      (loaded) => {
        if (!cancelled) {
          setState({ state: "ready", loaded });
        }
      },
      () => {
        if (!cancelled) {
          setState({ state: "failed" });
        }
      },
    );
    return () => {
      cancelled = true;
    };
  }, [lessonId, attempt]);

  const retry = useCallback(() => {
    setState({ state: "loading" });
    setAttempt((n) => n + 1);
  }, []);

  return (
    <VideoScreenBody
      lessonId={lessonId}
      state={state}
      known={known}
      onRetry={retry}
    />
  );
}

function VideoScreenBody({
  lessonId,
  state,
  known,
  onRetry,
}: {
  lessonId: string;
  state: State;
  known: Known;
  onRetry: () => void;
}) {
  const { navigate } = useNavigation();
  const loaded = state.state === "ready" ? state.loaded : null;

  // 保存に使う、いまの再生位置の文と、末尾に着いたか。
  const sentenceId = useRef<string | undefined>(undefined);
  const reachedEndRef = useRef(false);
  const [reachedEnd, setReachedEnd] = useState(false);
  useEffect(() => {
    sentenceId.current = loaded?.initialSentenceId;
  }, [loaded]);

  const save = useCallback(
    (trigger: SaveTrigger, id?: string) => {
      const args = savePositionArgs(
        lessonId,
        trigger,
        id ?? sentenceId.current,
        reachedEndRef.current,
      );
      if (args !== null) {
        saveVideoPosition(
          args.lesson_id,
          args.last_sentence_id,
          args.video_watched,
        ).catch(() => {
          // 保存に失敗しても、再生は続ける。次のきっかけで、また保存する。
        });
      }
    },
    [lessonId],
  );

  // 画面を離れるとき（戻るリンク、「演習へ」「次のレッスンへ」、ヘッダーのメニュー）に保存する。
  const saveRef = useRef(save);
  useEffect(() => {
    saveRef.current = save;
  }, [save]);
  const hasLoaded = loaded !== null;
  useEffect(() => {
    if (!hasLoaded) {
      return;
    }
    return () => saveRef.current("leave");
  }, [hasLoaded]);

  // 音量・ミュート・再生速度・字幕の保存。続けて変えたときに順番が入れ替わらないよう、1 つずつ送る。
  const settingsBase = useRef<Settings | null>(null);
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  useEffect(() => {
    settingsBase.current = loaded?.settings ?? null;
  }, [loaded]);
  const saveSettings = useCallback((next: VideoSettings) => {
    queue.current = queue.current.then(async () => {
      const base = settingsBase.current;
      if (base === null) {
        return;
      }
      try {
        settingsBase.current = await updateSettings({
          ...base,
          video: {
            volume: next.volume,
            muted: next.muted,
            playback_rate: next.playbackRate,
            captions: next.captions,
          },
        });
      } catch {
        // 保存に失敗しても、いまの再生には使い続ける。
      }
    });
  }, []);

  const chapters = known.chapters;
  const found = useMemo(
    () =>
      chapters === null ? null : findLesson(numberChapters(chapters), lessonId),
    [chapters, lessonId],
  );

  const following = chapters === null ? null : nextLesson(chapters, lessonId);
  const kind = known.kind ?? found?.lesson.lesson.kind ?? "lesson";
  // 表示する条件は、動画の読み込みの状態によらない（video-screen.md）。
  const action = rightAction(
    kind,
    known.progress,
    reachedEnd,
    following !== null,
  );

  const back = (event: MouseEvent) => {
    event.preventDefault();
    navigate({ name: "lessons" });
  };

  const endAction =
    action === "exercise"
      ? {
          label: "演習へ",
          onSelect: () => navigate({ name: "exercise", lessonId }),
        }
      : action === "next_lesson" && following !== null
        ? {
            label: "次のレッスンへ",
            // 次のレッスンが章末復習レッスンなら、演習画面を開く。
            onSelect: () => navigate(lessonRoute(following.id, following.kind)),
          }
        : null;

  const videoLoad: VideoLoad =
    loaded === null
      ? { state: state.state === "loading" ? "loading" : "failed" }
      : { state: "ready", video: loaded.video };

  return (
    <main
      className="video-screen"
      data-chapter={found === null ? undefined : found.chapter.color}
    >
      <div className="pg-context">
        <a className="pg-back" href="#" onClick={back}>
          <ChevronLeft
            className="pg-icon"
            aria-hidden="true"
            size={14}
            strokeWidth={2.4}
          />
          レッスン一覧
        </a>
        {found !== null && (
          <>
            <span className="pg-chip">
              <span className="pg-chip-num">{found.chapter.number}</span>
              {found.chapter.chapter.title}
            </span>
            <span className="pg-context-title">
              {found.lesson.number} {found.lesson.lesson.title}
            </span>
          </>
        )}
      </div>
      <VideoPlayer
        // 設定を読み込んだら（初期値が決まったら）、作り直す。
        key={loaded === null ? "waiting" : "loaded"}
        load={videoLoad}
        chapterColor={found === null ? null : found.chapter.color}
        lessonKind={kind}
        initialFrame={loaded?.initialFrame ?? 0}
        initialSettings={
          loaded === null ? DEFAULT_SETTINGS : toVideoSettings(loaded.settings)
        }
        endAction={endAction}
        onSentenceChange={(id) => {
          sentenceId.current = id;
          save("sentence_changed", id);
        }}
        onPause={(id) => {
          sentenceId.current = id;
          save("paused", id);
        }}
        onEnd={(id) => {
          sentenceId.current = id;
          reachedEndRef.current = true;
          setReachedEnd(true);
          save("ended", id);
        }}
        onSettingsChange={saveSettings}
        onRetry={onRetry}
      />
    </main>
  );
}
