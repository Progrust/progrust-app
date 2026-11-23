// 動画の下に置くプレーヤーの操作（design-system/progrust/components/VideoControls、
// documents/spec/screens/video-screen.md「プレーヤーの操作」）。
// 状態は持たず、値と操作を呼び出し側（VideoPlayer）から受け取る。
import {
  Captions as CaptionsIcon,
  ChevronDown,
  Pause,
  Play,
  RotateCcw,
  RotateCw,
  Volume2,
  VolumeX,
} from "lucide-react";
import type { MouseEvent } from "react";
import type { PlaybackRate } from "../commands/settings";
import { FPS, type Timeline } from "./timeline";
import { formatTime, sceneMarkRatios, seekBarRatio } from "./playerMath";
import { Slider } from "./Slider";

// 動画画面の再生速度（documents/spec/screens/video-screen.md「プレーヤーの操作」）。
export const PLAYBACK_RATES: PlaybackRate[] = [0.75, 1, 1.25, 1.5, 2];

// 音量・ミュート・再生速度・字幕の表示（settings.json の video。
// documents/spec/screens/settings-screen.md「settings.json」）。
export type VideoSettings = {
  // 0〜1。
  volume: number;
  muted: boolean;
  playbackRate: PlaybackRate;
  captions: boolean;
};

// 右端の行動（「演習へ」「次のレッスンへ」）。何も置かないときは null。
export type VideoEndAction = {
  label: string;
  onSelect: () => void;
};

export type VideoControlsProps = {
  // 動画のタイムライン。読み込みが済むまでと失敗したときは null（再生の操作を使えない）。
  timeline: Timeline | null;
  frame: number;
  playing: boolean;
  settings: VideoSettings;
  endAction: VideoEndAction | null;
  onTogglePlay: (event: MouseEvent) => void;
  // シークバーで押した位置・ドラッグした位置（0 から 1 の割合）。
  onSeek: (ratio: number) => void;
  onSkipBack: () => void;
  onSkipForward: () => void;
  onSettingsChange: (patch: Partial<VideoSettings>) => void;
};

// 「0 分 42 秒」の形（シークバーの aria-valuetext）。
function spokenTime(seconds: number): string {
  return `${Math.floor(seconds / 60)} 分 ${seconds % 60} 秒`;
}

// ボタンを押してもフォーカスを移さない。フォーカスのあるボタンが Space でも押され、
// キーボードショートカットと二重に切り替わるのを防ぐ（documents/verification/issue-3-player-custom-controls.md）。
function keepFocus(event: MouseEvent) {
  event.preventDefault();
}

export function VideoControls({
  timeline,
  frame,
  playing,
  settings,
  endAction,
  onTogglePlay,
  onSeek,
  onSkipBack,
  onSkipForward,
  onSettingsChange,
}: VideoControlsProps) {
  const disabled = timeline === null;
  const durationInFrames = timeline?.durationInFrames ?? 0;
  const [currentTime, totalTime] = formatTime(frame, durationInFrames).split(
    " / ",
  );
  const nowSeconds = Math.floor(frame / FPS);
  const totalSeconds = Math.ceil(durationInFrames / FPS);
  const playLabel = playing ? "一時停止" : "再生";

  return (
    <div className="pg-video-controls">
      <Slider
        label="再生位置"
        accent
        ratio={timeline === null ? 0 : seekBarRatio(timeline, frame)}
        min={0}
        max={totalSeconds}
        now={nowSeconds}
        valueText={spokenTime(nowSeconds)}
        marks={timeline === null ? [] : sceneMarkRatios(timeline)}
        disabled={disabled}
        onChange={onSeek}
      />
      <div className="pg-video-bar">
        <div className="pg-video-group">
          <button
            type="button"
            className="pg-icon-btn pg-icon-btn--ink"
            aria-label={playLabel}
            title={`${playLabel}（Space）`}
            aria-keyshortcuts="Space"
            disabled={disabled}
            onMouseDown={keepFocus}
            onClick={onTogglePlay}
          >
            {playing ? (
              <Pause
                className="pg-icon"
                aria-hidden="true"
                size={18}
                fill="currentColor"
                strokeWidth={2.2}
              />
            ) : (
              <Play
                className="pg-icon"
                aria-hidden="true"
                size={18}
                fill="currentColor"
                strokeWidth={2.2}
              />
            )}
          </button>
          <span className="pg-video-time">
            <b>{currentTime}</b> / {totalTime}
          </span>
          <button
            type="button"
            className="pg-btn"
            aria-label="5 秒戻る"
            title="5 秒戻る（←）"
            aria-keyshortcuts="ArrowLeft"
            disabled={disabled}
            onMouseDown={keepFocus}
            onClick={onSkipBack}
          >
            <RotateCcw
              className="pg-icon"
              aria-hidden="true"
              size={14}
              strokeWidth={2.4}
            />
            5 秒
          </button>
          <button
            type="button"
            className="pg-btn"
            aria-label="5 秒進む"
            title="5 秒進む（→）"
            aria-keyshortcuts="ArrowRight"
            disabled={disabled}
            onMouseDown={keepFocus}
            onClick={onSkipForward}
          >
            5 秒
            <RotateCw
              className="pg-icon"
              aria-hidden="true"
              size={14}
              strokeWidth={2.4}
            />
          </button>
        </div>
        <div className="pg-video-group">
          <div className="pg-select-wrap pg-select-wrap--pill">
            <select
              className="pg-select pg-select--pill"
              aria-label="再生速度"
              value={settings.playbackRate}
              onChange={(event) =>
                onSettingsChange({
                  playbackRate: Number(event.target.value) as PlaybackRate,
                })
              }
            >
              {PLAYBACK_RATES.map((rate) => (
                <option key={rate} value={rate}>
                  {rate} 倍
                </option>
              ))}
            </select>
            <ChevronDown
              className="pg-icon"
              aria-hidden="true"
              size={14}
              strokeWidth={2.4}
            />
          </div>
          <div className="pg-volume">
            <button
              type="button"
              className="pg-icon-btn pg-icon-btn--md"
              aria-label="ミュート"
              aria-pressed={settings.muted}
              onMouseDown={keepFocus}
              onClick={() => onSettingsChange({ muted: !settings.muted })}
            >
              {settings.muted ? (
                <VolumeX
                  className="pg-icon"
                  aria-hidden="true"
                  size={16}
                  strokeWidth={2.2}
                />
              ) : (
                <Volume2
                  className="pg-icon"
                  aria-hidden="true"
                  size={16}
                  strokeWidth={2.2}
                />
              )}
            </button>
            {/* ミュートしても、音量の位置は変えない（VideoControls の README）。 */}
            <Slider
              label="音量"
              ratio={settings.volume}
              min={0}
              max={100}
              now={Math.round(settings.volume * 100)}
              onChange={(ratio) =>
                onSettingsChange({ volume: Math.round(ratio * 100) / 100 })
              }
            />
          </div>
          <button
            type="button"
            className="pg-btn"
            aria-pressed={settings.captions}
            onMouseDown={keepFocus}
            onClick={() => onSettingsChange({ captions: !settings.captions })}
          >
            <CaptionsIcon
              className="pg-icon"
              aria-hidden="true"
              size={14}
              strokeWidth={2.4}
            />
            字幕
          </button>
        </div>
        {endAction !== null && (
          <div className="pg-video-end">
            <button
              type="button"
              className="pg-btn pg-btn--accent pg-btn--lg"
              onClick={endAction.onSelect}
            >
              {endAction.label}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}
