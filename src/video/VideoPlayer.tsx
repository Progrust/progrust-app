// 動画のプレーヤー（design-system/progrust/components/VideoPlayer、
// documents/spec/screens/video-screen.md、documents/spec/content-format/video/structure.md）。
// 上から、動画の台紙（Remotion の Player）、字幕（Captions）、操作（VideoControls）を積む。
//
// 根の要素は、置かれた領域（呼び出し側の flex の残り）いっぱいに広がり、その中に、
// 字幕と操作の分を引いた高さと幅に収まる最大の 16:9 で動画を置く（videoPlayer.css）。
import { Player, type PlayerRef } from "@remotion/player";
import { LoaderCircle } from "lucide-react";
import { useEffect, useMemo, useRef, useState } from "react";
import type { MouseEvent } from "react";
import type { LessonKind } from "../commands/content";
import { Captions } from "./Captions";
import { LessonVideo, type LessonVideoProps } from "./LessonVideo";
import {
  currentSentence,
  frameAtRatio,
  hasReachedEnd,
  sentenceStarts,
  skipTarget,
} from "./playerMath";
import { computeTimeline, FPS, type Timeline } from "./timeline";
import { usePlayerShortcuts } from "./usePlayerShortcuts";
import {
  VideoControls,
  type VideoEndAction,
  type VideoSettings,
} from "./VideoControls";
import "./videoPlayer.css";

export type { VideoEndAction, VideoSettings } from "./VideoControls";

// 動画の読み込みの状態。済んだら、Player の inputProps（#84 の部品とレッスンの動画のコード）を持つ。
export type VideoLoad =
  | { state: "loading" }
  | { state: "failed" }
  | { state: "ready"; video: LessonVideoProps };

export type VideoPlayerProps = {
  load: VideoLoad;
  // 動画を囲む要素の data-chapter（章カラーの番号）。章に属さずに表示するときは null。
  chapterColor: number | null;
  // レッスンの種類。右端に「次のレッスンへ」が出ることがある動画のみのレッスンでは、
  // 操作を 1 行に並べられる幅が違うので、動画の大きさの計算に使う（videoPlayer.css）。
  lessonKind: LessonKind;
  // 開いたときの再生位置（フレーム）。
  initialFrame: number;
  // 音量・ミュート・再生速度・字幕の初期値。
  initialSettings: VideoSettings;
  endAction: VideoEndAction | null;
  // 再生中の文が変わった。開いたときの文では呼ばない。
  onSentenceChange?: (sentenceId: string) => void;
  // 一時停止した（最後まで再生して止まったときを含む）。
  onPause?: (sentenceId: string) => void;
  // 末尾に着いた（再生で着いたときと、シークで着いたとき）。
  onEnd?: (sentenceId: string) => void;
  // 音量・ミュート・再生速度・字幕を変えた。変えた後の値の全体を渡す。
  onSettingsChange?: (settings: VideoSettings) => void;
  // 「もう一度読み込む」が押された。
  onRetry: () => void;
};

// 呼び出し側の関数のうち、最新のものを ref に持つ。
function useLatest<T>(value: T) {
  const ref = useRef(value);
  useEffect(() => {
    ref.current = value;
  });
  return ref;
}

// 台本と音声の長さからタイムラインを作る。作れないとき（台本と音声の長さが合わない、
// 動画の長さが 0 など）は null（読み込みに失敗したものとして扱う）。
function timelineOf(video: LessonVideoProps): Timeline | null {
  try {
    const timeline = computeTimeline(
      video.script,
      video.durations,
      video.cueReference,
    );
    return timeline.durationInFrames > 0 ? timeline : null;
  } catch {
    return null;
  }
}

export function VideoPlayer({
  load,
  chapterColor,
  lessonKind,
  initialFrame,
  initialSettings,
  endAction,
  onSentenceChange,
  onPause,
  onEnd,
  onSettingsChange,
  onRetry,
}: VideoPlayerProps) {
  // 設定は、読み込みの前後で変わらないよう、ここで持つ。
  const [settings, setSettings] = useState(initialSettings);
  const changeSettings = (patch: Partial<VideoSettings>) => {
    const next = { ...settings, ...patch };
    // 同じ位置を押したときや、ドラッグしても値が変わらないときは知らせない。
    if (
      (Object.keys(next) as (keyof VideoSettings)[]).every(
        (key) => next[key] === settings[key],
      )
    ) {
      return;
    }
    setSettings(next);
    onSettingsChange?.(next);
  };

  const video = load.state === "ready" ? load.video : null;
  const timeline = useMemo(
    () => (video === null ? null : timelineOf(video)),
    [video],
  );

  return (
    <div className="video-player">
      <div
        className="pg-video"
        data-chapter={chapterColor ?? undefined}
        data-lesson-kind={lessonKind}
      >
        {video !== null && timeline !== null ? (
          <LoadedVideo
            // 開いたときの位置は、Player を作るときだけ読まれるので、レッスンごとに作り直す。
            key={video.lesson.id}
            video={video}
            timeline={timeline}
            initialFrame={initialFrame}
            settings={settings}
            endAction={endAction}
            onSettingsChange={changeSettings}
            onSentenceChange={onSentenceChange}
            onPause={onPause}
            onEnd={onEnd}
            onRetry={onRetry}
          />
        ) : (
          <>
            <div className="pg-video-frame">
              {load.state === "loading" ? (
                <LoadingLayer />
              ) : (
                <FailedLayer onRetry={onRetry} />
              )}
            </div>
            <Captions text={null} />
            <VideoControls
              timeline={null}
              frame={0}
              playing={false}
              settings={settings}
              endAction={endAction}
              onTogglePlay={() => {}}
              onSeek={() => {}}
              onSkipBack={() => {}}
              onSkipForward={() => {}}
              onSettingsChange={changeSettings}
            />
          </>
        )}
      </div>
    </div>
  );
}

// 読み込み中の、回る印（ADR 0173）。
function LoadingLayer() {
  return (
    <div
      className="video-player-layer"
      role="status"
      aria-label="動画を読み込んでいます"
    >
      <LoaderCircle
        className="pg-icon pg-spin video-player-spinner"
        aria-hidden="true"
        size={18}
        strokeWidth={2.2}
      />
    </div>
  );
}

// 読み込みに失敗したときの文と「もう一度読み込む」（ADR 0173）。
function FailedLayer({ onRetry }: { onRetry: () => void }) {
  return (
    <div className="video-player-layer" role="alert">
      <p className="video-player-message">動画を読み込めませんでした</p>
      <button type="button" className="pg-btn" onClick={onRetry}>
        もう一度読み込む
      </button>
    </div>
  );
}

// Player の errorFallback に置き、描画のエラーを呼び出し側に伝える。何も描かない。
function RenderError({ onError }: { onError: (failed: boolean) => void }) {
  useEffect(() => {
    onError(true);
  }, [onError]);
  return null;
}

type LoadedVideoProps = {
  video: LessonVideoProps;
  timeline: Timeline;
  initialFrame: number;
  settings: VideoSettings;
  endAction: VideoEndAction | null;
  onSettingsChange: (patch: Partial<VideoSettings>) => void;
  onSentenceChange?: (sentenceId: string) => void;
  onPause?: (sentenceId: string) => void;
  onEnd?: (sentenceId: string) => void;
  onRetry: () => void;
};

// 読み込みが済んだ動画。Player を置き、再生位置と再生中かどうかを Player のイベントから受け取る。
function LoadedVideo({
  video,
  timeline,
  initialFrame,
  settings,
  endAction,
  onSettingsChange,
  onSentenceChange,
  onPause,
  onEnd,
  onRetry,
}: LoadedVideoProps) {
  // Player は、この部品を描いた後に作られるので、ref ではなく state で受け取り、
  // 作られたときにイベントを付ける。
  const [player, setPlayer] = useState<PlayerRef | null>(null);
  const [frame, setFrame] = useState(initialFrame);
  const [playing, setPlaying] = useState(false);
  // レッスンの動画のコードが描画中にエラーを出した。
  const [renderFailed, setRenderFailed] = useState(false);

  const sentences = useMemo(() => sentenceStarts(timeline), [timeline]);
  const texts = useMemo(
    () => new Map(video.script.sentences.map((s) => [s.id, s.text])),
    [video.script],
  );
  const sentenceId = currentSentence(sentences, frame)?.id;
  const ended = hasReachedEnd(timeline, frame);

  const callbacks = useLatest({ onSentenceChange, onPause, onEnd, sentenceId });

  useEffect(() => {
    if (player === null) {
      return;
    }
    const onFrame = (event: { detail: { frame: number } }) => {
      setFrame(event.detail.frame);
    };
    const onPlay = () => setPlaying(true);
    const onPaused = () => {
      setPlaying(false);
      // 一時停止した位置の文。Player のいまの位置から決める。
      const id = currentSentence(sentences, player.getCurrentFrame())?.id;
      if (id !== undefined) {
        callbacks.current.onPause?.(id);
      }
    };
    player.addEventListener("frameupdate", onFrame);
    player.addEventListener("seeked", onFrame);
    player.addEventListener("play", onPlay);
    player.addEventListener("pause", onPaused);
    return () => {
      player.removeEventListener("frameupdate", onFrame);
      player.removeEventListener("seeked", onFrame);
      player.removeEventListener("play", onPlay);
      player.removeEventListener("pause", onPaused);
    };
  }, [player, sentences, callbacks]);

  // 再生中の文が変わったら知らせる。開いたときの文では知らせない。
  const previousSentence = useRef(sentenceId);
  useEffect(() => {
    if (sentenceId !== undefined && sentenceId !== previousSentence.current) {
      callbacks.current.onSentenceChange?.(sentenceId);
    }
    previousSentence.current = sentenceId;
  }, [sentenceId, callbacks]);

  // 末尾に着いたら、着くたびに 1 回知らせる。開いたときに末尾にいても知らせない。
  const previousEnded = useRef(ended);
  useEffect(() => {
    const id = callbacks.current.sentenceId;
    if (ended && !previousEnded.current && id !== undefined) {
      callbacks.current.onEnd?.(id);
    }
    previousEnded.current = ended;
  }, [ended, callbacks]);

  // 音量とミュートは、Player を作った後は、Player の関数で変える。
  useEffect(() => {
    player?.setVolume(settings.volume);
  }, [player, settings.volume]);
  useEffect(() => {
    if (settings.muted) {
      player?.mute();
    } else {
      player?.unmute();
    }
  }, [player, settings.muted]);

  const togglePlay = (event?: MouseEvent) => {
    if (player === null || renderFailed) {
      return;
    }
    if (player.isPlaying()) {
      player.pause();
    } else {
      player.play(event);
    }
  };
  const skip = (direction: "back" | "forward") => {
    if (player !== null && !renderFailed) {
      player.seekTo(skipTarget(timeline, player.getCurrentFrame(), direction));
    }
  };
  usePlayerShortcuts({
    onTogglePlay: () => togglePlay(),
    onSkipBack: () => skip("back"),
    onSkipForward: () => skip("forward"),
  });

  const captionText = settings.captions
    ? (texts.get(sentenceId ?? "") ?? "")
    : null;

  return (
    <>
      <div className="pg-video-frame">
        <div className="video-player-stage">
          <Player
            ref={setPlayer}
            component={LessonVideo}
            inputProps={video}
            durationInFrames={timeline.durationInFrames}
            fps={FPS}
            compositionWidth={1280}
            compositionHeight={720}
            initialFrame={initialFrame}
            initialVolume={settings.volume}
            initiallyMuted={settings.muted}
            playbackRate={settings.playbackRate}
            // 組み込みの操作を使わない。末尾で止め、先頭に戻さず、繰り返さない（ADR 0178）。
            controls={false}
            spaceKeyToPlayOrPause={false}
            clickToPlay={false}
            doubleClickToFullscreen={false}
            moveToBeginningWhenEnded={false}
            loop={false}
            autoPlay={false}
            acknowledgeRemotionLicense
            style={{ width: "100%", height: "100%" }}
            // Player の中は 1280×720 を縮めて描くので、失敗の表示は Player の外（台紙の上）に重ねる。
            errorFallback={() => <RenderError onError={setRenderFailed} />}
          />
        </div>
        {renderFailed && <FailedLayer onRetry={onRetry} />}
      </div>
      <Captions text={captionText} />
      <VideoControls
        // 描画に失敗したときは、読み込みに失敗したときと同じく、再生できない。
        timeline={renderFailed ? null : timeline}
        frame={renderFailed ? 0 : frame}
        playing={playing}
        settings={settings}
        endAction={endAction}
        onTogglePlay={togglePlay}
        onSeek={(ratio) => player?.seekTo(frameAtRatio(timeline, ratio))}
        onSkipBack={() => skip("back")}
        onSkipForward={() => skip("forward")}
        onSettingsChange={onSettingsChange}
      />
    </>
  );
}
