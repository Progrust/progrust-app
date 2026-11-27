// 設定画面。画面の配置は design-system/ui-design/Settings.dc.html、
// 決まりは documents/spec/screens/settings-screen.md。
// 作ってある区分は「表示」「動画」「AI」「環境」「アプリ」。「学習データ」の区分は、学習データを扱う処理と一緒に作る。
import { ChevronDown, Volume2, VolumeX } from "lucide-react";
import { useEffect, useRef, useState } from "react";
import type { CSSProperties, KeyboardEvent, PointerEvent } from "react";
import { redetectTools } from "../commands/environment";
import {
  getAppVersion,
  getSettings,
  updateSettings,
} from "../commands/settings";
import type { PlaybackRate, Settings, Theme } from "../commands/settings";
import { applyTheme } from "../theme";
import { AiSection } from "./AiSection";
import { EnvironmentSection } from "./EnvironmentSection";
import { Row, Section } from "./SettingsParts";
import "./settings.css";

const THEMES: { value: Theme; label: string }[] = [
  { value: "light", label: "ライト" },
  { value: "dark", label: "ダーク" },
  { value: "system", label: "システムに合わせる" },
];

// 動画画面の再生速度（documents/spec/screens/video-screen.md）。
const RATES: { value: PlaybackRate; label: string }[] = [
  { value: 0.75, label: "0.75 倍" },
  { value: 1, label: "1 倍" },
  { value: 1.25, label: "1.25 倍" },
  { value: 1.5, label: "1.5 倍" },
  { value: 2, label: "2 倍" },
];

const FONT_SIZES = Array.from({ length: 13 }, (_, i) => 12 + i);

type Loaded = { settings: Settings; version: string };

// 設定とバージョンを読み込んで、設定画面を表示する。
export function SettingsScreen() {
  const [loaded, setLoaded] = useState<Loaded | null>(null);
  const [failed, setFailed] = useState(false);
  useEffect(() => {
    let cancelled = false;
    Promise.all([getSettings(), getAppVersion()]).then(
      ([settings, app]) => {
        if (!cancelled) {
          setLoaded({ settings, version: app.version });
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
      <main className="settings-wrap">
        <p role="alert">設定を読み込めませんでした。</p>
      </main>
    );
  }
  if (loaded === null) {
    return <main className="settings-wrap" aria-busy="true" />;
  }
  return <SettingsView initial={loaded.settings} version={loaded.version} />;
}

export function SettingsView({
  initial,
  version,
}: {
  initial: Settings;
  version: string;
}) {
  const [settings, setSettings] = useState(initial);
  const [saveFailed, setSaveFailed] = useState(false);
  // 続けて変えたときに、保存の順番が入れ替わらないように、保存は 1 つずつ行う。
  const latest = useRef(initial);
  const queue = useRef<Promise<void>>(Promise.resolve());

  const change = (next: Settings) => {
    latest.current = next;
    setSettings(next);
    setSaveFailed(false);
    if (next.theme !== settings.theme) {
      applyTheme(next.theme);
    }
    queue.current = queue.current.then(async () => {
      try {
        await updateSettings(latest.current);
      } catch (error: unknown) {
        console.error("update_settings に失敗した", error);
        setSaveFailed(true);
      }
    });
  };
  const changeVideo = (patch: Partial<Settings["video"]>) => {
    change({ ...latest.current, video: { ...latest.current.video, ...patch } });
  };

  // パスを保存して、検出し直す。保存は設定の保存の順番に並べ、保存できなかったときは検出し直さずに reject する。
  const saveToolPath = (
    tool: keyof Settings["tool_paths"],
    path: string | null,
  ): Promise<void> => {
    const run = queue.current.then(async () => {
      const before = latest.current.tool_paths;
      const next = {
        ...latest.current,
        tool_paths: { ...before, [tool]: path },
      };
      latest.current = next;
      try {
        await updateSettings(next);
      } catch (error: unknown) {
        // 保存できなかったパスは、画面にも、次の保存にも残さない。
        latest.current = { ...latest.current, tool_paths: before };
        throw error;
      }
      setSettings((current) => ({ ...current, tool_paths: next.tool_paths }));
      await redetectTools();
    });
    queue.current = run.catch((error: unknown) => {
      console.error("パスの保存か検出し直しに失敗した", error);
    });
    return run;
  };

  // モデルを保存する。保存は設定の保存の順番に並べ、保存できなかったときは保存前の値に戻して reject する。
  const saveModel = (
    key: keyof Settings["models"],
    model: string,
  ): Promise<void> => {
    const run = queue.current.then(async () => {
      const before = latest.current.models;
      const next = {
        ...latest.current,
        models: { ...before, [key]: model },
      };
      latest.current = next;
      try {
        await updateSettings(next);
      } catch (error: unknown) {
        // 保存できなかった値は、画面にも、次の保存にも残さない。
        latest.current = { ...latest.current, models: before };
        throw error;
      }
      setSettings((current) => ({ ...current, models: next.models }));
    });
    queue.current = run.catch((error: unknown) => {
      console.error("モデルの保存に失敗した", error);
    });
    return run;
  };

  return (
    <main className="settings-wrap">
      <div className="settings-head">
        <div>
          <h1>設定</h1>
          <p>表示・動画・AI・環境の設定です。変更はすぐに保存されます。</p>
        </div>
      </div>
      {saveFailed && <p role="alert">設定を保存できませんでした。</p>}

      <Section id="s-view" title="表示">
        <Row
          title="テーマ"
          help="「システムに合わせる」では、OS の設定が変わると切り替わります。"
        >
          <div className="pg-seg" role="radiogroup" aria-label="テーマ">
            {THEMES.map((item) => (
              <button
                key={item.value}
                type="button"
                role="radio"
                aria-checked={settings.theme === item.value}
                onClick={() => {
                  change({ ...latest.current, theme: item.value });
                }}
              >
                {item.label}
              </button>
            ))}
          </div>
        </Row>
        <Row title="エディタの文字サイズ" help="12〜24px。既定は 14px です。">
          <div className="pg-select-wrap" style={{ width: 120 }}>
            <select
              className="pg-select"
              aria-label="エディタの文字サイズ"
              value={settings.editor_font_size}
              onChange={(event) => {
                change({
                  ...latest.current,
                  editor_font_size: Number(event.target.value),
                });
              }}
            >
              {FONT_SIZES.map((size) => (
                <option key={size} value={size}>
                  {size}px
                </option>
              ))}
            </select>
            <ChevronDown
              className="pg-icon"
              aria-hidden="true"
              size={16}
              strokeWidth={2.2}
            />
          </div>
        </Row>
      </Section>

      <Section id="s-video" title="動画">
        <Row title="音量とミュート" help="動画の音量と、ミュートするかどうか。">
          <div className="pg-volume">
            <button
              type="button"
              className="pg-icon-btn pg-icon-btn--md"
              aria-label="ミュート"
              aria-pressed={settings.video.muted}
              onClick={() => {
                changeVideo({ muted: !latest.current.video.muted });
              }}
            >
              {settings.video.muted ? (
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
            <VolumeSlider
              value={settings.video.volume}
              onPreview={(volume) => {
                setSettings({
                  ...latest.current,
                  video: { ...latest.current.video, volume },
                });
              }}
              onCommit={(volume) => {
                changeVideo({ volume });
              }}
            />
          </div>
        </Row>
        <Row title="再生速度" help="動画を再生する速さ。">
          <div className="pg-seg" role="radiogroup" aria-label="再生速度">
            {RATES.map((item) => (
              <button
                key={item.value}
                type="button"
                role="radio"
                aria-checked={settings.video.playback_rate === item.value}
                onClick={() => {
                  changeVideo({ playback_rate: item.value });
                }}
              >
                {item.label}
              </button>
            ))}
          </div>
        </Row>
        <Row title="字幕" help="再生中の文を、動画の下に表示します。">
          <label className="pg-switch">
            <input
              type="checkbox"
              role="switch"
              checked={settings.video.captions}
              onChange={(event) => {
                changeVideo({ captions: event.target.checked });
              }}
            />
            表示する
          </label>
        </Row>
      </Section>

      <AiSection models={settings.models} onSave={saveModel} />

      <EnvironmentSection
        toolPaths={settings.tool_paths}
        onSavePath={saveToolPath}
      />

      <Section id="s-app" title="アプリ">
        <Row title="バージョン" help={`Progrust ${version}`}>
          {null}
        </Row>
      </Section>
    </main>
  );
}

const clamp01 = (n: number) => Math.min(1, Math.max(0, n));

// 音量のスライダー（デザインシステムの Slider）。ドラッグの間は表示だけを変え、離したときに保存する。
function VolumeSlider({
  value,
  onPreview,
  onCommit,
}: {
  value: number;
  onPreview: (volume: number) => void;
  onCommit: (volume: number) => void;
}) {
  const dragging = useRef<number | null>(null);
  const valueAt = (event: PointerEvent<HTMLDivElement>) => {
    const rect = event.currentTarget.getBoundingClientRect();
    return rect.width === 0
      ? value
      : clamp01((event.clientX - rect.left) / rect.width);
  };
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step: Record<string, number> = {
      ArrowLeft: -0.05,
      ArrowDown: -0.05,
      ArrowRight: 0.05,
      ArrowUp: 0.05,
    };
    let next: number | undefined;
    if (event.key in step) {
      next = clamp01(Math.round((value + step[event.key]) * 100) / 100);
    } else if (event.key === "Home") {
      next = 0;
    } else if (event.key === "End") {
      next = 1;
    }
    if (next !== undefined) {
      event.preventDefault();
      onCommit(next);
    }
  };
  return (
    <div
      className="pg-slider"
      role="slider"
      tabIndex={0}
      aria-label="音量"
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(value * 100)}
      style={{ "--pg-slider-value": value } as CSSProperties}
      onKeyDown={onKeyDown}
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture?.(event.pointerId);
        const next = valueAt(event);
        dragging.current = next;
        onPreview(next);
      }}
      onPointerMove={(event) => {
        if (dragging.current !== null) {
          const next = valueAt(event);
          dragging.current = next;
          onPreview(next);
        }
      }}
      onPointerUp={() => {
        if (dragging.current !== null) {
          const next = dragging.current;
          dragging.current = null;
          onCommit(next);
        }
      }}
    >
      <span className="pg-slider-track">
        <span className="pg-slider-fill" />
      </span>
      <span className="pg-slider-thumb" />
    </div>
  );
}
