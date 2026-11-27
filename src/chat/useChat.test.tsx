import { clearMocks, mockIPC } from "@tauri-apps/api/mocks";
import { emit } from "@tauri-apps/api/event";
import { act, cleanup, renderHook } from "@testing-library/react";
import { afterEach, describe, expect, it } from "vitest";

import {
  CHAT_DELTA,
  CHAT_FINISHED,
  type ChatFinished,
  type ChatTurn,
} from "../commands/chat";
import { chatHistory, useChat, type UseChatOptions } from "./useChat";

type Call = { command: string; args: Record<string, unknown> };

type Options = {
  // コマンドの名前から、拒否するときの値への対応。
  reject?: { [command: string]: unknown };
  // send_chat が、呼ばれてすぐ（戻り値より先に）送るイベント。
  emitOnSend?: (requestId: string) => Promise<void>;
  // send_chat の戻りを、この Promise が解決するまで待たせる。
  hold?: Promise<void>;
};

function setup(
  options: Options = {},
  props: UseChatOptions = { lessonId: "ch04-04", context: "explanation" },
) {
  const calls: Call[] = [];
  mockIPC(
    async (command, args) => {
      const payload = (args ?? {}) as Record<string, unknown>;
      if (command === "send_chat") {
        await options.emitOnSend?.(payload.request_id as string);
        await options.hold;
      }
      if (options.reject?.[command] !== undefined) {
        throw options.reject[command];
      }
      return null;
    },
    { shouldMockEvents: true },
  );
  // イベントの受け取り（plugin:event|listen・unlisten）は mockIPC の関数に届かないので、その手前で、
  // すべての呼び出しを呼んだ順に記録する。
  const internals = (
    window as unknown as {
      __TAURI_INTERNALS__: { invoke: (...args: unknown[]) => unknown };
    }
  ).__TAURI_INTERNALS__;
  const invoke = internals.invoke;
  internals.invoke = (...args: unknown[]) => {
    const [command, payload] = args as [string, Record<string, unknown>?];
    calls.push({ command, args: payload ?? {} });
    return invoke(...args);
  };
  const hook = renderHook((p: UseChatOptions) => useChat(p), {
    initialProps: props,
  });
  return { calls, ...hook };
}

function commands(calls: Call[], name: string) {
  return calls.filter((c) => c.command === name).map((c) => c.args);
}

function lastRequestId(calls: Call[]): string {
  const sent = commands(calls, "send_chat");
  return sent[sent.length - 1].request_id as string;
}

// 非同期の受け取りと send_chat が済むまで待つ。
async function settle() {
  await act(async () => {
    await new Promise((resolve) => setTimeout(resolve, 0));
  });
}

async function delta(requestId: string, text: string) {
  await act(async () => {
    await emit(CHAT_DELTA, { request_id: requestId, text });
  });
}

async function finished(
  requestId: string,
  status: ChatFinished["status"],
  error: ChatFinished["error"] = null,
) {
  await act(async () => {
    await emit(CHAT_FINISHED, { request_id: requestId, status, error });
  });
}

afterEach(() => {
  cleanup();
  clearMocks();
});

describe("useChat", () => {
  it("2 回目の send_chat の history に、1 回目の質問と回答が入る", async () => {
    const { calls, result } = setup();
    act(() => {
      result.current.send("所有者とは何ですか？");
    });
    await settle();
    const first = lastRequestId(calls);
    await delta(first, "値を持っている");
    await delta(first, "変数のことです。");
    await finished(first, "completed");

    act(() => {
      result.current.send("さっきの例をもう少し詳しく");
    });
    await settle();

    const sent = commands(calls, "send_chat");
    expect(sent).toHaveLength(2);
    expect(sent[0].history).toEqual([]);
    expect(sent[1]).toMatchObject({
      lesson_id: "ch04-04",
      context: "explanation",
      sentence_id: null,
      question: "さっきの例をもう少し詳しく",
      retry: false,
    });
    expect(sent[1].history).toEqual([
      {
        question: "所有者とは何ですか？",
        answer: "値を持っている変数のことです。",
      },
    ]);
    expect(result.current.exchanges.map((e) => e.status)).toEqual([
      "completed",
      "answering",
    ]);
  });

  it("止めた往復は止めるまでの回答で history に入り、エラーの往復は入らない", async () => {
    const { calls, result } = setup();
    act(() => {
      result.current.send("質問 1");
    });
    await settle();
    const first = lastRequestId(calls);
    await delta(first, "途中まで");
    act(() => {
      result.current.stop();
    });
    await settle();
    expect(commands(calls, "cancel_chat")).toEqual([{ request_id: first }]);
    // 停止は chat_finished（cancelled）が届いてから。
    expect(result.current.exchanges[0].status).toBe("answering");
    await delta(first, "の回答");
    await finished(first, "cancelled");
    expect(result.current.exchanges[0]).toMatchObject({
      status: "stopped",
      answer: "途中までの回答",
      error: null,
    });

    act(() => {
      result.current.send("質問 2");
    });
    await settle();
    const second = lastRequestId(calls);
    await delta(second, "壊れた回答");
    await finished(second, "error", { category: "rate_limited", detail: null });
    expect(result.current.exchanges[1]).toMatchObject({
      status: "error",
      error: { kind: "ai", error: { category: "rate_limited", detail: null } },
    });

    act(() => {
      result.current.send("質問 3");
    });
    await settle();
    const sent = commands(calls, "send_chat");
    expect(sent[2].history).toEqual([
      { question: "質問 1", answer: "途中までの回答" },
    ]);
  });

  it("止めるまでに回答が無くても、止めた往復は空の回答で history に入る", () => {
    const history: ChatTurn[] = chatHistory([
      {
        requestId: "a",
        question: "質問",
        answer: "",
        status: "stopped",
        error: null,
        sentenceId: null,
      },
      {
        requestId: "b",
        question: "エラー",
        answer: "",
        status: "error",
        error: { kind: "ai", error: { category: "timeout", detail: null } },
        sentenceId: null,
      },
      {
        requestId: "c",
        question: "回答中",
        answer: "途中",
        status: "answering",
        error: null,
        sentenceId: null,
      },
    ]);
    expect(history).toEqual([{ question: "質問", answer: "" }]);
  });

  it("再試行は同じ質問を retry: true で送り、往復が増えない", async () => {
    const { calls, result } = setup(
      {},
      { lessonId: "ch04-04", context: "video", getSentenceId: () => "m5" },
    );
    act(() => {
      result.current.send("ムーブ後に使えないのはなぜ？");
    });
    await settle();
    const first = lastRequestId(calls);
    await delta(first, "途中");
    await finished(first, "error", { category: "other", detail: "失敗" });

    let retried = false;
    act(() => {
      retried = result.current.retry();
    });
    await settle();
    expect(retried).toBe(true);

    const sent = commands(calls, "send_chat");
    expect(sent).toHaveLength(2);
    expect(sent[0]).toMatchObject({ retry: false, sentence_id: "m5" });
    expect(sent[1]).toMatchObject({
      context: "video",
      sentence_id: "m5",
      question: "ムーブ後に使えないのはなぜ？",
      history: [],
      retry: true,
    });
    const second = lastRequestId(calls);
    expect(second).not.toBe(first);
    expect(result.current.exchanges).toHaveLength(1);
    expect(result.current.exchanges[0]).toMatchObject({
      requestId: second,
      status: "answering",
      answer: "",
      error: null,
    });
    expect(result.current.answering).toBe(true);

    await delta(second, "答えです。");
    await finished(second, "completed");
    expect(result.current.exchanges).toEqual([
      expect.objectContaining({ status: "completed", answer: "答えです。" }),
    ]);
  });

  it("エラーでない往復は再試行しない", async () => {
    const { calls, result } = setup();
    act(() => {
      result.current.send("質問");
    });
    await settle();
    const first = lastRequestId(calls);
    let retried = true;
    act(() => {
      retried = result.current.retry();
    });
    expect(retried).toBe(false);
    await finished(first, "completed");
    act(() => {
      retried = result.current.retry();
    });
    expect(retried).toBe(false);
    expect(commands(calls, "send_chat")).toHaveLength(1);
  });

  it("send_chat が拒否されたら往復をエラーにして message を持ち、再試行できる", async () => {
    const rejected = {
      code: "claude_unavailable",
      message: "claude が見つかりません。",
      detail: null,
    };
    const { calls, result } = setup({ reject: { send_chat: rejected } });
    act(() => {
      result.current.send("質問");
    });
    await settle();
    expect(result.current.exchanges[0]).toMatchObject({
      status: "error",
      error: {
        kind: "command",
        code: "claude_unavailable",
        message: "claude が見つかりません。",
      },
    });
    expect(result.current.answering).toBe(false);
    act(() => {
      result.current.retry();
    });
    await settle();
    expect(commands(calls, "send_chat").map((args) => args.retry)).toEqual([
      false,
      true,
    ]);
  });

  it("受け取りを始めてから send_chat を呼び、別の request_id の断片は入らない", async () => {
    const { calls, result } = setup({
      // send_chat の戻り値より先に、断片と終わりが届く。
      emitOnSend: async (requestId) => {
        await emit(CHAT_DELTA, { request_id: "other", text: "ほかの回答" });
        await emit(CHAT_DELTA, { request_id: requestId, text: "所有権は" });
        await emit(CHAT_DELTA, { request_id: requestId, text: "持ち主です。" });
        await emit(CHAT_FINISHED, {
          request_id: requestId,
          status: "completed",
          error: null,
        });
      },
    });
    act(() => {
      result.current.send("所有権とは？");
    });
    await settle();

    const names = calls.map((c) => c.command);
    const sendIndex = names.indexOf("send_chat");
    const listens = calls
      .slice(0, sendIndex)
      .filter((c) => c.command === "plugin:event|listen")
      .map((c) => c.args.event);
    expect(listens).toEqual(
      expect.arrayContaining([CHAT_DELTA, CHAT_FINISHED]),
    );

    expect(result.current.exchanges).toEqual([
      expect.objectContaining({
        status: "completed",
        answer: "所有権は持ち主です。",
      }),
    ]);
    expect(result.current.answering).toBe(false);

    // 終わった後の断片も入らない。
    await delta(lastRequestId(calls), "後の断片");
    await delta("other", "ほかの回答");
    expect(result.current.exchanges[0].answer).toBe("所有権は持ち主です。");
  });

  it("回答中は送信できず、chat_finished で回答中でなくなる", async () => {
    const { calls, result } = setup();
    let accepted: boolean[] = [];
    act(() => {
      accepted = [result.current.send("質問 1"), result.current.send("質問 2")];
    });
    await settle();
    expect(accepted).toEqual([true, false]);
    expect(result.current.answering).toBe(true);
    act(() => {
      accepted.push(result.current.send("質問 3"));
    });
    await settle();
    expect(accepted).toEqual([true, false, false]);
    expect(commands(calls, "send_chat")).toHaveLength(1);
    expect(result.current.exchanges).toHaveLength(1);

    await finished(lastRequestId(calls), "completed");
    expect(result.current.answering).toBe(false);
    act(() => {
      accepted.push(result.current.send("質問 4"));
    });
    await settle();
    expect(accepted[3]).toBe(true);
    expect(commands(calls, "send_chat")).toHaveLength(2);
  });

  it("video の sentence_id は、送信したときに関数を呼んで得る", async () => {
    let sentence = "m1";
    const { calls, result } = setup(
      {},
      {
        lessonId: "ch04-04",
        context: "video",
        getSentenceId: () => sentence,
      },
    );
    sentence = "m3";
    act(() => {
      result.current.send("質問");
    });
    await settle();
    expect(commands(calls, "send_chat")[0]).toMatchObject({
      context: "video",
      sentence_id: "m3",
    });
  });

  it("部品が外れるとき、回答中なら cancel_chat を呼び、受け取りをやめる", async () => {
    const { calls, result, unmount } = setup();
    act(() => {
      result.current.send("質問");
    });
    await settle();
    const id = lastRequestId(calls);
    await delta(id, "途中");
    unmount();
    await settle();
    expect(commands(calls, "cancel_chat")).toEqual([{ request_id: id }]);
    expect(
      calls.filter((c) => c.command === "plugin:event|unlisten"),
    ).toHaveLength(2);
  });

  it("部品が外れるとき、回答中でなければ cancel_chat を呼ばない", async () => {
    const { calls, result, unmount } = setup();
    act(() => {
      result.current.send("質問");
    });
    await settle();
    await finished(lastRequestId(calls), "completed");
    unmount();
    await settle();
    expect(commands(calls, "cancel_chat")).toEqual([]);
  });

  it("send_chat を待つ間に外れたら、始まった後に cancel_chat を呼ぶ", async () => {
    let resume = () => {};
    const hold = new Promise<void>((resolve) => {
      resume = resolve;
    });
    const { calls, result, unmount } = setup({ hold });
    act(() => {
      result.current.send("質問");
    });
    await settle();
    const id = lastRequestId(calls);
    unmount();
    await settle();
    // 始まる前の cancel_chat は、バックエンドで何もしないので、まだ呼ばない。
    expect(commands(calls, "cancel_chat")).toEqual([]);
    resume();
    await settle();
    expect(commands(calls, "cancel_chat")).toEqual([{ request_id: id }]);
  });

  it("send_chat を待つ間に止めたら、始まった後に cancel_chat を呼ぶ", async () => {
    let resume = () => {};
    const hold = new Promise<void>((resolve) => {
      resume = resolve;
    });
    const { calls, result } = setup({ hold });
    act(() => {
      result.current.send("質問");
    });
    await settle();
    const id = lastRequestId(calls);
    act(() => {
      result.current.stop();
    });
    expect(commands(calls, "cancel_chat")).toEqual([]);
    resume();
    await settle();
    expect(commands(calls, "cancel_chat")).toEqual([{ request_id: id }]);
    await finished(id, "cancelled");
    expect(result.current.exchanges[0].status).toBe("stopped");
  });
});
