import { render } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { createDraftSaveScheduler, useDraftSaveTrigger } from "./draftSave";
import { useStopOnLeave } from "./useStopOnLeave";

beforeEach(() => {
  vi.useFakeTimers();
});
afterEach(() => {
  vi.useRealTimers();
});

describe("下書きを保存するきっかけ", () => {
  it("最後の編集から 1 秒後に 1 回だけ起きる", () => {
    const save = vi.fn();
    const s = createDraftSaveScheduler(save);
    s.edited();
    vi.advanceTimersByTime(999);
    expect(save).not.toHaveBeenCalled();
    vi.advanceTimersByTime(1);
    expect(save).toHaveBeenCalledTimes(1);
    expect(save).toHaveBeenCalledWith("idle");
    vi.advanceTimersByTime(5000);
    expect(save).toHaveBeenCalledTimes(1);
  });

  it("その間に編集があれば延ばす", () => {
    const save = vi.fn();
    const s = createDraftSaveScheduler(save);
    s.edited();
    vi.advanceTimersByTime(600);
    s.edited();
    vi.advanceTimersByTime(600);
    expect(save).not.toHaveBeenCalled();
    vi.advanceTimersByTime(400);
    expect(save).toHaveBeenCalledTimes(1);
  });

  it("画面を離れるとき、保存していない編集があればすぐ保存し、後で重ねて保存しない", () => {
    const save = vi.fn();
    const s = createDraftSaveScheduler(save);
    s.edited();
    s.leave();
    expect(save).toHaveBeenCalledWith("leave");
    vi.advanceTimersByTime(2000);
    expect(save).toHaveBeenCalledTimes(1);
  });

  it("編集が無ければ、離れても保存しない", () => {
    const save = vi.fn();
    createDraftSaveScheduler(save).leave();
    expect(save).not.toHaveBeenCalled();
  });

  it("フックは、アンマウントで保存する", () => {
    const save = vi.fn();
    function Harness() {
      const s = useDraftSaveTrigger(save);
      s.edited();
      return null;
    }
    const { unmount } = render(<Harness />);
    unmount();
    expect(save).toHaveBeenCalledWith("leave");
  });
});

describe("画面を離れるときに止める", () => {
  function Harness(props: { id: string | null; stop: (id: string) => void }) {
    useStopOnLeave(props.id, props.stop);
    return null;
  }

  it("実行中に画面を離れると、その request_id で止める", () => {
    const stop = vi.fn();
    const { rerender, unmount } = render(<Harness id={null} stop={stop} />);
    rerender(<Harness id="req-1" stop={stop} />);
    expect(stop).not.toHaveBeenCalled();
    unmount();
    expect(stop).toHaveBeenCalledExactlyOnceWith("req-1");
  });

  it("何も動いていなければ止めない", () => {
    const stop = vi.fn();
    render(<Harness id={null} stop={stop} />).unmount();
    expect(stop).not.toHaveBeenCalled();
  });
});
