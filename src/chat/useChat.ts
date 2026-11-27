// 画面を開いている間の質疑応答の会話を持ち、送信・停止・再試行を、コマンドとイベントにつなぐ
// （documents/spec/qa-chat/history.md、documents/spec/qa-chat/screen.md、
// documents/spec/architecture/commands/chat.md、ADR 0025、ADR 0254、ADR 0255）。
// 動画画面と正解のモーダルが使う。会話はメモリにだけ持ち、部品が外れると消える。ファイルにも設定にも残さない。
import { useEffect, useRef, useState } from "react";

import type { AiError } from "../claude/aiErrorMessage";
import {
  cancelChat,
  listenChatEvents,
  sendChat,
  type ChatContext,
  type ChatFinished,
  type ChatTurn,
} from "../commands/chat";
import {
  asCommandError,
  commandErrorMessage,
  type CommandErrorCode,
} from "../commands/error";

// 往復の状態。answering は回答中、completed は回答が終わった、stopped は「停止」で止めた、error は失敗した。
export type ChatExchangeStatus =
  "answering" | "completed" | "stopped" | "error";

// 往復のエラー。ai は回答の途中の失敗（chat_finished の error）で、画面に出す文は aiErrorMessage で決める。
// command は send_chat が拒否されたとき（と、イベントの受け取りを始められなかったとき）で、message をそのまま出す。
export type ChatExchangeError =
  | { kind: "ai"; error: AiError }
  | { kind: "command"; code: CommandErrorCode | null; message: string };

// 会話の 1 往復。
export type ChatExchange = {
  // いまの送信の request_id。再試行で送り直すと変わる。
  requestId: string;
  question: string;
  // 届いた断片を、届いた順につないだもの。止めた往復では、止めるまでの回答。
  answer: string;
  status: ChatExchangeStatus;
  // status が error のときのエラー。ほかでは null。
  error: ChatExchangeError | null;
  // video のとき、質問を送信したときに再生中だった文の ID。explanation では null。再試行でも、この ID で送り直す。
  sentenceId: string | null;
};

export type UseChatOptions = {
  // レッスン ID（生成問題は gen-{generation_id}）。
  lessonId: string;
  context: ChatContext;
  // video のとき、質問を送信したときに再生中の文の ID を返す関数。送信したときに呼ぶ。explanation では呼ばない。
  getSentenceId?: () => string | null;
};

export type UseChat = {
  // 会話の往復。古い順。
  exchanges: ChatExchange[];
  // 回答中の往復があるか。回答中は送信も再試行もできない。
  answering: boolean;
  // 質問を送る。回答中のときは送らずに false を返す。送ったとき（往復を足したとき）は true。
  // 文字数の検査は画面がする。
  send: (question: string) => boolean;
  // 回答中の往復を止める。往復は chat_finished（cancelled）が届いたときに「停止」になる。
  stop: () => void;
  // エラーになった最後の往復を、同じ質問のまま送り直す。往復は足さない。送り直せないときは false を返す。
  retry: () => boolean;
};

// send_chat に渡す会話履歴。完了した往復と、止めた往復（止めるまでの回答）を古い順に入れ、
// エラーの往復と回答中の往復は入れない。10 往復に絞るのはバックエンドがする（documents/spec/qa-chat/history.md）。
export function chatHistory(exchanges: readonly ChatExchange[]): ChatTurn[] {
  return exchanges
    .filter(
      (exchange) =>
        exchange.status === "completed" || exchange.status === "stopped",
    )
    .map((exchange) => ({
      question: exchange.question,
      answer: exchange.answer,
    }));
}

function finishedStatus(event: ChatFinished): ChatExchangeStatus {
  switch (event.status) {
    case "completed":
      return "completed";
    case "cancelled":
      return "stopped";
    case "error":
      return "error";
  }
}

function commandError(error: unknown): ChatExchangeError {
  return {
    kind: "command",
    code: asCommandError(error)?.code ?? null,
    message: commandErrorMessage(error),
  };
}

// 動いている送信。受け取りと send_chat を待つ間に止めたり外れたりしたら、cancelRequested を立てる。
type Active = {
  requestId: string;
  unlisten: (() => void) | null;
  // send_chat が解決したか。解決する前の cancel_chat は、バックエンドで何もしない。
  started: boolean;
  cancelRequested: boolean;
};

export function useChat({
  lessonId,
  context,
  getSentenceId,
}: UseChatOptions): UseChat {
  // 会話の正。送信したときの会話履歴を、レンダーを待たずに作るため、ref に持って state に写す。
  const exchangesRef = useRef<ChatExchange[]>([]);
  const [exchanges, setExchanges] = useState<ChatExchange[]>([]);
  // 動いている送信。送信してから chat_finished が届くか、始められなかったと分かるまである。
  const activeRef = useRef<Active | null>(null);
  // 部品が外れたか。
  const leftRef = useRef(false);
  const optionsRef = useRef({ lessonId, context, getSentenceId });
  useEffect(() => {
    optionsRef.current = { lessonId, context, getSentenceId };
  });

  const update = (
    requestId: string,
    change: (exchange: ChatExchange) => ChatExchange,
  ) => {
    exchangesRef.current = exchangesRef.current.map((exchange) =>
      exchange.requestId === requestId ? change(exchange) : exchange,
    );
    setExchanges(exchangesRef.current);
  };

  const release = (active: Active) => {
    active.unlisten?.();
    active.unlisten = null;
    if (activeRef.current === active) {
      activeRef.current = null;
    }
  };

  // 往復を回答中にした後に、受け取りを始めてから send_chat を呼ぶ。
  const start = async (exchange: ChatExchange, retry: boolean) => {
    const active: Active = {
      requestId: exchange.requestId,
      unlisten: null,
      started: false,
      cancelRequested: false,
    };
    activeRef.current = active;
    const requestId = exchange.requestId;
    // 送る会話履歴は、この往復を除いたもの。
    const history = chatHistory(
      exchangesRef.current.filter((other) => other.requestId !== requestId),
    );

    const failed = (error: unknown) => {
      release(active);
      if (!leftRef.current) {
        update(requestId, (current) =>
          current.status === "answering"
            ? { ...current, status: "error", error: commandError(error) }
            : current,
        );
      }
    };

    let unlisten: () => void;
    try {
      unlisten = await listenChatEvents(requestId, {
        delta: (event) => {
          update(requestId, (current) =>
            current.status === "answering"
              ? { ...current, answer: current.answer + event.text }
              : current,
          );
        },
        finished: (event) => {
          release(active);
          update(requestId, (current) =>
            current.status === "answering"
              ? {
                  ...current,
                  status: finishedStatus(event),
                  error:
                    event.status === "error"
                      ? {
                          kind: "ai",
                          error: event.error ?? {
                            category: "other",
                            detail: null,
                          },
                        }
                      : null,
                }
              : current,
          );
        },
      });
    } catch (error) {
      failed(error);
      return;
    }
    // 受け取りを待つ間に外れたら、送らない。
    if (leftRef.current) {
      unlisten();
      release(active);
      return;
    }
    // 受け取りを待つ間に chat_finished は届かないので、ここではまだ動いている。
    active.unlisten = unlisten;

    const { lessonId: lesson, context: where } = optionsRef.current;
    try {
      await sendChat({
        requestId,
        lessonId: lesson,
        context: where,
        sentenceId: exchange.sentenceId,
        question: exchange.question,
        history,
        retry,
      });
    } catch (error) {
      failed(error);
      return;
    }
    active.started = true;
    // 始まる前に止めたり外れたりしたら、そのときの cancel_chat は、まだ始まっていない ID に届いて何もしなかった。
    // 始まったいま、もう一度止める。chat_finished が既に届いていたら、止めない。
    if (active.cancelRequested && activeRef.current === active) {
      cancelChat(requestId).catch(() => {});
    }
  };

  const send = (question: string): boolean => {
    if (activeRef.current !== null) {
      return false;
    }
    const { context: where, getSentenceId: sentence } = optionsRef.current;
    const exchange: ChatExchange = {
      requestId: crypto.randomUUID(),
      question,
      answer: "",
      status: "answering",
      error: null,
      sentenceId: where === "video" ? (sentence?.() ?? null) : null,
    };
    exchangesRef.current = [...exchangesRef.current, exchange];
    setExchanges(exchangesRef.current);
    void start(exchange, false);
    return true;
  };

  const retry = (): boolean => {
    const last = exchangesRef.current.at(-1);
    if (activeRef.current !== null || last?.status !== "error") {
      return false;
    }
    const exchange: ChatExchange = {
      ...last,
      requestId: crypto.randomUUID(),
      answer: "",
      status: "answering",
      error: null,
    };
    exchangesRef.current = [...exchangesRef.current.slice(0, -1), exchange];
    setExchanges(exchangesRef.current);
    void start(exchange, true);
    return true;
  };

  const cancel = (active: Active) => {
    active.cancelRequested = true;
    if (active.started) {
      cancelChat(active.requestId).catch(() => {});
    }
  };

  const stop = () => {
    const active = activeRef.current;
    if (active !== null) {
      cancel(active);
    }
  };

  // 部品が外れるとき、回答中なら止めて、受け取りをやめる。
  useEffect(() => {
    leftRef.current = false;
    return () => {
      leftRef.current = true;
      const active = activeRef.current;
      if (active !== null) {
        cancel(active);
        active.unlisten?.();
        active.unlisten = null;
      }
    };
  }, []);

  return {
    exchanges,
    answering: exchanges.some((exchange) => exchange.status === "answering"),
    send,
    stop,
    retry,
  };
}
