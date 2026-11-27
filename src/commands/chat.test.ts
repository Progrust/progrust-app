import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { afterEach, describe, expect, it } from "vitest";

import {
  CHAT_DELTA,
  CHAT_FINISHED,
  type ChatDelta,
  type ChatFinished,
  cancelChat,
  listenChatEvents,
  sendChat,
} from "./chat";

afterEach(() => {
  clearMocks();
});

describe("sendChat・cancelChat", () => {
  it("引数を snake_case の名前で渡す", async () => {
    const calls: [string, unknown][] = [];
    mockIPC((command, args) => {
      calls.push([command, args]);
      return null;
    });
    await sendChat({
      requestId: "r1",
      lessonId: "ch04-04",
      context: "video",
      sentenceId: "m5",
      question: "ムーブ後に使えないのはなぜ？",
      history: [{ question: "所有者とは？", answer: "値を持つ変数です。" }],
      retry: false,
    });
    await cancelChat("r1");
    expect(calls).toEqual([
      [
        "send_chat",
        {
          request_id: "r1",
          lesson_id: "ch04-04",
          context: "video",
          sentence_id: "m5",
          question: "ムーブ後に使えないのはなぜ？",
          history: [{ question: "所有者とは？", answer: "値を持つ変数です。" }],
          retry: false,
        },
      ],
      ["cancel_chat", { request_id: "r1" }],
    ]);
  });
});

describe("listenChatEvents", () => {
  it("自分の requestId のイベントだけを受け取り、やめた後は受け取らない", async () => {
    mockIPC(() => null, { shouldMockEvents: true });
    const deltas: ChatDelta[] = [];
    const finished: ChatFinished[] = [];
    const unlisten = await listenChatEvents("r1", {
      delta: (event) => deltas.push(event),
      finished: (event) => finished.push(event),
    });

    await emit(CHAT_DELTA, { request_id: "r1", text: "所有権は" });
    await emit(CHAT_DELTA, { request_id: "other", text: "ほかの回答" });
    await emit(CHAT_DELTA, { request_id: "r1", text: "値の持ち主です。" });
    await emit(CHAT_FINISHED, {
      request_id: "other",
      status: "cancelled",
      error: null,
    });
    await emit(CHAT_FINISHED, {
      request_id: "r1",
      status: "completed",
      error: null,
    });

    expect(deltas.map((delta) => delta.text).join("")).toBe(
      "所有権は値の持ち主です。",
    );
    expect(finished).toEqual([
      { request_id: "r1", status: "completed", error: null },
    ]);

    unlisten();
    await emit(CHAT_DELTA, { request_id: "r1", text: "後の断片" });
    expect(deltas).toHaveLength(2);
  });
});
