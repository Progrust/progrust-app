import { clearMocks, mockConvertFileSrc } from "@tauri-apps/api/mocks";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { useCurrentFrame } from "remotion";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { Chapter, Script } from "../commands/content";
import type { LessonVideoProps } from "./LessonVideo";
import type { Durations } from "./timeline";
import {
  VideoPlayer,
  type VideoLoad,
  type VideoPlayerProps,
  type VideoSettings,
} from "./VideoPlayer";

beforeEach(() => {
  mockConvertFileSrc("macos");
  // remotion 4.0.545 は、NODE_ENV が test のとき、Html5Audio をタイムラインに登録しようとし、
  // Player が登録先を用意していないので、描画時にエラーになる（アプリでは NODE_ENV が test にならない）。
  vi.stubEnv("NODE_ENV", "development");
  // jsdom は音声を再生できない（play() が Promise を返さない）。
  vi.spyOn(HTMLMediaElement.prototype, "play").mockResolvedValue(undefined);
  vi.spyOn(HTMLMediaElement.prototype, "pause").mockReturnValue(undefined);
});

afterEach(() => {
  clearMocks();
  vi.unstubAllEnvs();
  vi.restoreAllMocks();
});

const voice = {
  narrator: "Natsuki Karin",
  speed: 120,
  pause: 0.2,
  emotion: {},
};

// title: lead 1.0 秒・tail 0.5 秒、move: lead 0.5 秒・tail 1.0 秒。
const script: Script = {
  scenes: [
    { id: "title", name: "表紙", lead: 1.0, tail: 0.5 },
    { id: "move", name: "ムーブ", lead: 0.5, tail: 1.0 },
  ],
  sentences: [
    { id: "t1", scene: "title", text: "表紙の文", say: "ひょうし" },
    {
      id: "m1",
      scene: "move",
      text: "ムーブの文 1",
      say: "ぶんいち",
      pause_after: 0.5,
    },
    { id: "m2", scene: "move", text: "ムーブの文 2", say: "ぶんに" },
  ],
};

// title（0〜105）: t1 は 30 から。move（105〜285）: m1 は 120 から、m2 は 195 から。
// 動画の長さは 285 フレーム（9.5 秒）、最後のフレームは 284。
const durations: Durations = {
  t1: { seconds: 2.0, say: "ひょうし", voice },
  m1: { seconds: 2.0, say: "ぶんいち", voice },
  m2: { seconds: 2.0, say: "ぶんに", voice },
};
const DURATION = 285;

const chapters: Chapter[] = [
  {
    id: "ch04",
    title: "所有権",
    description: "",
    lessons: [
      {
        id: "ch04-02",
        kind: "lesson",
        title: "所有権のムーブ",
        summary: null,
        estimated_minutes: null,
        in_preparation: false,
      },
    ],
  },
];

function Scene() {
  return <p>frame {useCurrentFrame()}</p>;
}

const video: LessonVideoProps = {
  script,
  durations,
  cueReference: {},
  scenes: { title: Scene, move: Scene },
  voiceDir: "/app/Resources/content/lessons/ch04-02/video/voice",
  chapters,
  lesson: {
    id: "ch04-02",
    chapter: "ch04",
    kind: "lesson",
    title: "所有権のムーブ",
    primary_elements: [],
    secondary_elements: [],
  },
};

const settings: VideoSettings = {
  volume: 0.8,
  muted: false,
  playbackRate: 1,
  captions: false,
};

function setup(overrides: Partial<VideoPlayerProps> = {}) {
  const props: VideoPlayerProps = {
    load: { state: "ready", video },
    chapterColor: 4,
    lessonKind: "lesson",
    initialFrame: 0,
    initialSettings: settings,
    endAction: null,
    onSentenceChange: vi.fn(),
    onPause: vi.fn(),
    onEnd: vi.fn(),
    onSettingsChange: vi.fn(),
    onRetry: vi.fn(),
    ...overrides,
  };
  const view = render(<VideoPlayer {...props} />);
  return { props, ...view };
}

function seekBar() {
  return screen.getByRole("slider", { name: "再生位置" });
}

function volumeSlider() {
  return screen.getByRole("slider", { name: "音量" });
}

function captions(container: HTMLElement) {
  const element = container.querySelector(".pg-captions");
  if (element === null) {
    throw new Error("字幕が無い");
  }
  return element;
}

function frameElement(container: HTMLElement) {
  const element = container.querySelector(".pg-video-frame");
  if (element === null) {
    throw new Error("動画の台紙が無い");
  }
  return element;
}

// 現在の再生位置の表示（「0:04」など）。
function currentTime(container: HTMLElement) {
  return container.querySelector(".pg-video-time b")?.textContent;
}

// シークバーの棒の幅を 1000px として、割合の位置を押す。
function pressSeekBar(ratio: number) {
  const bar = seekBar();
  vi.spyOn(bar, "getBoundingClientRect").mockReturnValue(
    DOMRect.fromRect({ x: 0, y: 0, width: 1000, height: 20 }),
  );
  fireEvent.pointerDown(bar, {
    button: 0,
    pointerId: 1,
    clientX: ratio * 1000,
  });
  fireEvent.pointerUp(bar, { button: 0, pointerId: 1, clientX: ratio * 1000 });
}

describe("読み込みが済んだとき", () => {
  it("開いても自動では再生せず、開いたときの位置で止まっている", async () => {
    const { container } = setup({ initialFrame: 120 });
    await act(() => new Promise((resolve) => setTimeout(resolve, 50)));
    expect(screen.getByRole("button", { name: "再生" })).toBeEnabled();
    expect(
      screen.queryByRole("button", { name: "一時停止" }),
    ).not.toBeInTheDocument();
    expect(currentTime(container)).toBe("0:04");
    expect(screen.getByText("frame 15")).toBeInTheDocument();
  });

  it("時間に、現在の位置と動画全体の長さを出す", () => {
    const { container } = setup();
    expect(container.querySelector(".pg-video-time")).toHaveTextContent(
      "0:00 / 0:10",
    );
  });

  it("再生／一時停止と 5 秒の移動のボタンに、キーを title と aria-keyshortcuts で書く", () => {
    setup();
    const play = screen.getByRole("button", { name: "再生" });
    expect(play).toHaveAttribute("title", "再生（Space）");
    expect(play).toHaveAttribute("aria-keyshortcuts", "Space");
    const back = screen.getByRole("button", { name: "5 秒戻る" });
    expect(back).toHaveAttribute("title", "5 秒戻る（←）");
    expect(back).toHaveAttribute("aria-keyshortcuts", "ArrowLeft");
    const forward = screen.getByRole("button", { name: "5 秒進む" });
    expect(forward).toHaveAttribute("title", "5 秒進む（→）");
    expect(forward).toHaveAttribute("aria-keyshortcuts", "ArrowRight");
  });

  it("再生ボタンで再生し、もう一度押すと一時停止して、一時停止した文を知らせる", () => {
    const { props } = setup({ initialFrame: 130 });
    fireEvent.click(screen.getByRole("button", { name: "再生" }));
    const pause = screen.getByRole("button", { name: "一時停止" });
    expect(pause).toHaveAttribute("title", "一時停止（Space）");
    fireEvent.click(pause);
    expect(screen.getByRole("button", { name: "再生" })).toBeInTheDocument();
    expect(props.onPause).toHaveBeenCalledWith("m1");
  });

  it("シークバーに、各シーンの開始位置の印を出す", () => {
    setup();
    const marks = seekBar().querySelectorAll<HTMLElement>(".pg-slider-mark");
    expect(Array.from(marks).map((mark) => mark.style.left)).toEqual([
      "0%",
      `${(105 / DURATION) * 100}%`,
    ]);
  });

  it("シークバーを押した位置に移し、一時停止のままにする", () => {
    const { container } = setup();
    pressSeekBar(120 / DURATION);
    expect(currentTime(container)).toBe("0:04");
    expect(seekBar()).toHaveAttribute("aria-valuenow", "4");
    expect(seekBar()).toHaveAttribute("aria-valuetext", "0 分 4 秒");
    expect(screen.getByRole("button", { name: "再生" })).toBeInTheDocument();
  });

  it("5 秒進む・5 秒戻るのボタンで、5 秒後・5 秒前に移す", () => {
    const { container } = setup({ initialFrame: 30 });
    fireEvent.click(screen.getByRole("button", { name: "5 秒進む" }));
    expect(currentTime(container)).toBe("0:06");
    fireEvent.click(screen.getByRole("button", { name: "5 秒戻る" }));
    expect(currentTime(container)).toBe("0:01");
    fireEvent.click(screen.getByRole("button", { name: "5 秒戻る" }));
    expect(currentTime(container)).toBe("0:00");
  });

  it("→ と ← のキーで、5 秒後・5 秒前に移す", () => {
    const { container } = setup({ initialFrame: 30 });
    fireEvent.keyDown(document.body, { code: "ArrowRight", key: "ArrowRight" });
    expect(currentTime(container)).toBe("0:06");
    fireEvent.keyDown(document.body, { code: "ArrowLeft", key: "ArrowLeft" });
    expect(currentTime(container)).toBe("0:01");
  });

  it("Space のキーで、再生と一時停止を切り替える", () => {
    setup();
    fireEvent.keyDown(document.body, { code: "Space", key: " " });
    expect(
      screen.getByRole("button", { name: "一時停止" }),
    ).toBeInTheDocument();
    fireEvent.keyDown(document.body, { code: "Space", key: " " });
    expect(screen.getByRole("button", { name: "再生" })).toBeInTheDocument();
  });

  it("動画を、章カラーの data-chapter を付けた要素で囲む", () => {
    const { container } = setup();
    const wrapper = container.querySelector('[data-chapter="4"]');
    expect(wrapper).toContainElement(
      container.querySelector('[data-lesson="ch04-02"]') as HTMLElement,
    );
    expect(wrapper).toContainElement(seekBar());
  });

  it("章に属さずに表示するときは、data-chapter を付けない", () => {
    const { container } = setup({ chapterColor: null });
    expect(container.querySelector("[data-chapter]")).toBeNull();
  });
});

describe("字幕", () => {
  it("表示しないときも、動画の枠の外（下）に中身のない字幕の場所を置く", () => {
    const { container } = setup();
    const element = captions(container);
    expect(element).toBeEmptyDOMElement();
    expect(element).toHaveAttribute("aria-hidden", "true");
    expect(frameElement(container)).not.toContainElement(
      element as HTMLElement,
    );
    expect(
      frameElement(container).compareDocumentPosition(element) &
        Node.DOCUMENT_POSITION_FOLLOWING,
    ).toBeTruthy();
  });

  it("表示すると再生中の文の text を出し、文が変わると切り替える", () => {
    const { container } = setup({ initialFrame: 40 });
    const frame = frameElement(container);
    const captionsButton = screen.getByRole("button", { name: "字幕" });
    expect(captionsButton).toHaveAttribute("aria-pressed", "false");

    fireEvent.click(captionsButton);
    expect(captionsButton).toHaveAttribute("aria-pressed", "true");
    expect(captions(container)).toHaveTextContent("表紙の文");
    expect(captions(container)).not.toHaveAttribute("aria-hidden");
    expect(frame).not.toContainElement(captions(container) as HTMLElement);

    pressSeekBar(200 / DURATION);
    expect(captions(container)).toHaveTextContent("ムーブの文 2");

    // 字幕を切り替えても、動画の台紙は同じ要素のまま（描き直さない）。
    fireEvent.click(captionsButton);
    expect(captions(container)).toBeEmptyDOMElement();
    expect(frameElement(container)).toBe(frame);
  });

  it("字幕の初期値が表示なら、開いたときから字幕を出す", () => {
    const { container } = setup({
      initialFrame: 130,
      initialSettings: { ...settings, captions: true },
    });
    expect(captions(container)).toHaveTextContent("ムーブの文 1");
  });
});

describe("呼び出し側に知らせること", () => {
  it("再生中の文が変わったら知らせる。開いたときの文では知らせない", () => {
    const { props } = setup({ initialFrame: 40 });
    expect(props.onSentenceChange).not.toHaveBeenCalled();
    pressSeekBar(130 / DURATION);
    expect(props.onSentenceChange).toHaveBeenLastCalledWith("m1");
    pressSeekBar(140 / DURATION);
    expect(props.onSentenceChange).toHaveBeenCalledTimes(1);
  });

  it("シークで末尾に着いたら、着くたびに 1 回知らせる", () => {
    const { props } = setup();
    pressSeekBar(1);
    expect(props.onEnd).toHaveBeenCalledTimes(1);
    expect(props.onEnd).toHaveBeenCalledWith("m2");
    pressSeekBar(1);
    expect(props.onEnd).toHaveBeenCalledTimes(1);
    pressSeekBar(0.5);
    fireEvent.keyDown(document.body, { code: "ArrowRight", key: "ArrowRight" });
    expect(props.onEnd).toHaveBeenCalledTimes(2);
  });

  it("開いたときに末尾にいても、末尾に着いたとは知らせない", () => {
    const { props } = setup({ initialFrame: DURATION - 1 });
    expect(props.onEnd).not.toHaveBeenCalled();
  });

  it("再生速度・ミュート・字幕を変えたら、変えた後の設定を知らせる", () => {
    const { props } = setup();
    fireEvent.change(screen.getByRole("combobox", { name: "再生速度" }), {
      target: { value: "1.5" },
    });
    expect(props.onSettingsChange).toHaveBeenLastCalledWith({
      ...settings,
      playbackRate: 1.5,
    });
    fireEvent.click(screen.getByRole("button", { name: "ミュート" }));
    expect(props.onSettingsChange).toHaveBeenLastCalledWith({
      ...settings,
      playbackRate: 1.5,
      muted: true,
    });
    fireEvent.click(screen.getByRole("button", { name: "字幕" }));
    expect(props.onSettingsChange).toHaveBeenLastCalledWith({
      ...settings,
      playbackRate: 1.5,
      muted: true,
      captions: true,
    });
  });

  it("ミュートしても、音量の位置は変えない", () => {
    setup();
    const mute = screen.getByRole("button", { name: "ミュート" });
    expect(mute).toHaveAttribute("aria-pressed", "false");
    fireEvent.click(mute);
    expect(mute).toHaveAttribute("aria-pressed", "true");
    expect(volumeSlider()).toHaveAttribute("aria-valuenow", "80");
  });

  it("音量を、押した位置とドラッグした位置に変える。値が変わらないときは知らせない", () => {
    const { props } = setup();
    const slider = volumeSlider();
    vi.spyOn(slider, "getBoundingClientRect").mockReturnValue(
      DOMRect.fromRect({ x: 100, y: 0, width: 72, height: 20 }),
    );
    fireEvent.pointerDown(slider, { button: 0, pointerId: 2, clientX: 118 });
    expect(props.onSettingsChange).toHaveBeenLastCalledWith({
      ...settings,
      volume: 0.25,
    });
    fireEvent.pointerMove(slider, { pointerId: 2, clientX: 136 });
    expect(props.onSettingsChange).toHaveBeenLastCalledWith({
      ...settings,
      volume: 0.5,
    });
    expect(slider).toHaveAttribute("aria-valuenow", "50");
    fireEvent.pointerMove(slider, { pointerId: 2, clientX: 136 });
    fireEvent.pointerUp(slider, { pointerId: 2, clientX: 136 });
    expect(props.onSettingsChange).toHaveBeenCalledTimes(2);
  });

  it("音量にフォーカスがあっても、← / → で 5 秒の移動をし、音量は変えない", () => {
    const { props, container } = setup({ initialFrame: 30 });
    const slider = volumeSlider();
    slider.focus();
    fireEvent.keyDown(slider, { code: "ArrowRight", key: "ArrowRight" });
    expect(currentTime(container)).toBe("0:06");
    expect(slider).toHaveAttribute("aria-valuenow", "80");
    expect(props.onSettingsChange).not.toHaveBeenCalled();
  });
});

describe("右端の行動", () => {
  it("渡された行動を、主ボタンで右端に置く", () => {
    const onSelect = vi.fn();
    setup({ endAction: { label: "演習へ", onSelect } });
    const button = screen.getByRole("button", { name: "演習へ" });
    expect(button).toHaveClass("pg-btn--accent");
    fireEvent.click(button);
    expect(onSelect).toHaveBeenCalledTimes(1);
  });

  it("渡されないときは何も置かない", () => {
    const { container } = setup();
    expect(container.querySelector(".pg-video-end")).toBeNull();
  });
});

describe("読み込み中と、読み込みに失敗したとき", () => {
  const loading: VideoLoad = { state: "loading" };
  const failed: VideoLoad = { state: "failed" };

  it("読み込み中は、動画の枠の中に回る印を出し、再生できない", () => {
    const { container } = setup({
      load: loading,
      endAction: { label: "演習へ", onSelect: vi.fn() },
    });
    const status = screen.getByRole("status", {
      name: "動画を読み込んでいます",
    });
    expect(frameElement(container)).toContainElement(status);
    expect(status.querySelector(".pg-spin")).not.toBeNull();
    // 回る印に、見える文字を添える。
    expect(status).toHaveTextContent("動画を読み込んでいます");
    expect(screen.getByRole("button", { name: "再生" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "5 秒戻る" })).toBeDisabled();
    expect(screen.getByRole("button", { name: "5 秒進む" })).toBeDisabled();
    expect(seekBar()).toHaveAttribute("aria-disabled", "true");
    expect(captions(container)).toHaveAttribute("aria-hidden", "true");
    // 「演習へ」を出す条件は、読み込みの状態によらない（ADR 0173）。
    expect(screen.getByRole("button", { name: "演習へ" })).toBeInTheDocument();
    // キーを押しても何も起きない。
    fireEvent.keyDown(document.body, { code: "Space", key: " " });
    expect(screen.getByRole("button", { name: "再生" })).toBeDisabled();
  });

  it("失敗したら、動画の枠の中に文と「もう一度読み込む」を出す", () => {
    const { container, props } = setup({
      load: failed,
      endAction: { label: "次のレッスンへ", onSelect: vi.fn() },
    });
    const alert = screen.getByRole("alert");
    expect(frameElement(container)).toContainElement(alert);
    expect(alert).toHaveTextContent("動画を読み込めませんでした");
    fireEvent.click(screen.getByRole("button", { name: "もう一度読み込む" }));
    expect(props.onRetry).toHaveBeenCalledTimes(1);
    expect(screen.getByRole("button", { name: "再生" })).toBeDisabled();
    expect(
      screen.getByRole("button", { name: "次のレッスンへ" }),
    ).toBeInTheDocument();
  });

  it("台本と音声の長さが合わず、タイムラインを作れないときも、失敗として表示する", () => {
    setup({
      load: { state: "ready", video: { ...video, durations: {} } },
    });
    expect(screen.getByRole("alert")).toHaveTextContent(
      "動画を読み込めませんでした",
    );
  });

  it("動画のコードが描画中にエラーを出したら、縮めて描く動画の外（台紙の上）に失敗を表示し、再生できない", () => {
    // Remotion と React が、捕まえたエラーをコンソールに書くので、出力を止める。
    vi.spyOn(console, "error").mockImplementation(() => {});
    function Broken(): never {
      throw new Error("シーンのエラー");
    }
    const { container, props } = setup({
      load: {
        state: "ready",
        video: { ...video, scenes: { title: Broken, move: Broken } },
      },
    });
    const alert = screen.getByRole("alert");
    expect(alert).toHaveTextContent("動画を読み込めませんでした");
    expect(frameElement(container)).toContainElement(alert);
    expect(container.querySelector(".video-player-stage")).not.toContainElement(
      alert,
    );
    expect(screen.getByRole("button", { name: "再生" })).toBeDisabled();
    fireEvent.click(screen.getByRole("button", { name: "もう一度読み込む" }));
    expect(props.onRetry).toHaveBeenCalledTimes(1);
  });

  it("読み込み中に変えた設定は、読み込みが済んだ後も使う", () => {
    const { container, rerender, props } = setup({ load: loading });
    fireEvent.click(screen.getByRole("button", { name: "字幕" }));
    rerender(<VideoPlayer {...props} load={{ state: "ready", video }} />);
    expect(screen.getByRole("button", { name: "字幕" })).toHaveAttribute(
      "aria-pressed",
      "true",
    );
    expect(captions(container)).toHaveTextContent("表紙の文");
  });
});
