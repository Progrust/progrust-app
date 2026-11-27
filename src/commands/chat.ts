// 質疑応答のコマンドとイベント（documents/spec/architecture/commands/chat.md）。
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import type { AiError } from "../claude/aiErrorMessage";

// 質問の場面。video は動画の再生中、explanation は演習の解説表示中。
export type ChatContext = "video" | "explanation";

// 会話履歴の 1 往復。止めた往復は、止めるまでに表示した回答を answer に入れる。エラーになった往復は入れない
// （documents/spec/qa-chat/history.md）。
export type ChatTurn = {
  question: string;
  answer: string;
};

// send_chat の引数。
export type SendChatArgs = {
  // この質問の ID（crypto.randomUUID()）。
  requestId: string;
  lessonId: string;
  context: ChatContext;
  // video のとき、質問を送信したときに再生中だった文の ID。explanation では null。
  sentenceId: string | null;
  // 2,000 文字まで。
  question: string;
  // フロントエンドが持つ会話履歴。古い順。バックエンドが末尾の 10 往復までを AI に渡す。
  history: ChatTurn[];
  // エラーの後の「再試行」で送り直すときは true。試行ログに記録しない。
  retry: boolean;
};

// イベントの名前。受け取り（listen）を始めてから、send_chat を呼ぶ。
export const CHAT_DELTA = "chat_delta";
export const CHAT_FINISHED = "chat_finished";

// イベント chat_delta の内容。text は前の断片に続けてつなぐ。
export type ChatDelta = {
  request_id: string;
  text: string;
};

// completed は回答が終わった、cancelled は cancel_chat で止めた、error は失敗した。
export type ChatStatus = "completed" | "cancelled" | "error";

// イベント chat_finished の内容。質問ごとに 1 回届く。回答の全文は chat_delta の断片をつないだもの。
export type ChatFinished = {
  request_id: string;
  status: ChatStatus;
  // status が error のときのエラー。ほかでは null。
  error: AiError | null;
};

// 質問を送る。claude の呼び出しを始めたら null で解決する。回答は chat_delta と chat_finished で届く。
// 空の質問・2,000 文字を超える質問・video で sentenceId が null のときは invalid_argument、その場面で質問できない
// とき（正解していない演習の explanation など）は invalid_state、回答中の質問があるときは busy、claude が
// 見つからないときは claude_unavailable で拒否される。
export function sendChat(args: SendChatArgs): Promise<null> {
  return invoke<null>("send_chat", {
    request_id: args.requestId,
    lesson_id: args.lessonId,
    context: args.context,
    sentence_id: args.sentenceId,
    question: args.question,
    history: args.history,
    retry: args.retry,
  });
}

// 回答中の「停止」と、回答中に画面を離れるときに、質問を止める。止めた結果は、claude のプロセスが終わってから
// chat_finished（cancelled）で届く（最大で 3 秒ほどかかる）。回答中でない ID のときは、何もせずに null で解決する。
export function cancelChat(requestId: string): Promise<null> {
  return invoke<null>("cancel_chat", { request_id: requestId });
}

// 1 つの質問のイベントを受け取る関数。
export type ChatEventHandlers = {
  delta: (event: ChatDelta) => void;
  finished: (event: ChatFinished) => void;
};

// requestId のイベント（chat_delta・chat_finished）の受け取りを始める。受け取りを始めたら、受け取りをやめる関数で
// 解決する。この後に send_chat を呼ぶ（documents/spec/architecture/commands/conventions.md「要求を見分ける ID」）。
// 受け取りを始められなかったイベントがあれば、始めた分をやめてから、拒否する。
export async function listenChatEvents(
  requestId: string,
  handlers: ChatEventHandlers,
): Promise<() => void> {
  const mine =
    <T extends { request_id: string }>(handler: (payload: T) => void) =>
    (event: { payload: T }) => {
      if (event.payload.request_id === requestId) {
        handler(event.payload);
      }
    };
  const results = await Promise.allSettled([
    listen<ChatDelta>(CHAT_DELTA, mine(handlers.delta)),
    listen<ChatFinished>(CHAT_FINISHED, mine(handlers.finished)),
  ]);
  const unlisteners = results.flatMap((result) =>
    result.status === "fulfilled" ? [result.value] : [],
  );
  const unlistenAll = () => {
    for (const unlisten of unlisteners.splice(0)) {
      unlisten();
    }
  };
  const failed = results.find((result) => result.status === "rejected");
  if (failed !== undefined) {
    unlistenAll();
    throw failed.reason;
  }
  return unlistenAll;
}
