// 演習画面のコンソールに残す出力（documents/spec/screens/exercise-screen.md、ADR 0103、ADR 0224、ADR 0226）。
// 実行・提出の出力の断片と、学習者が入力した行を、受け取った順に足していく。
// ANSI エスケープは足すときに読み、色の付いた文字列の並びにしておく。残すのは末尾の 64K 文字までで、
// 超えたら古いほうから消し、消したことを覚えておく（画面の先頭に「古い出力を省略しました」を出す）。
import type { RunOutput } from "../commands/run";
import {
  INITIAL_ANSI_STATE,
  PLAIN_STYLE,
  parseAnsi,
  sameStyle,
  type AnsiState,
  type AnsiStyle,
} from "./ansi";

// コンソールに残す文字数の上限（ADR 0103）。数えるのは、エスケープを除いた、表示する文字。
export const CONSOLE_TAIL_LIMIT = 64 * 1024;

// 出力の種類。stdout・stderr・system は run_output の断片の stream、input は学習者が入力した行。
export type ConsoleKind = "stdout" | "stderr" | "system" | "input";

// 種類と色が同じ一続きの文字列。id は、画面で描き直さずに済むように付ける、消えるまで変わらない番号。
export type ConsoleSegment = {
  id: number;
  kind: ConsoleKind;
  style: AnsiStyle;
  text: string;
};

type Streams = Record<"stdout" | "stderr" | "system", AnsiState>;

export type ConsoleLog = {
  segments: readonly ConsoleSegment[];
  // segments の文字数の合計。
  length: number;
  // 上限を超えて、古い出力を消したことがあるか。
  truncated: boolean;
  // 次の segment の id。
  nextId: number;
  // 最後に出力を受け取った実行・提出の ID。
  requestId: string | null;
  // 種類ごとの、エスケープの読みかけと、いまの色。
  streams: Streams;
};

const INITIAL_STREAMS: Streams = {
  stdout: INITIAL_ANSI_STATE,
  stderr: INITIAL_ANSI_STATE,
  system: INITIAL_ANSI_STATE,
};

export const EMPTY_CONSOLE_LOG: ConsoleLog = {
  segments: [],
  length: 0,
  truncated: false,
  nextId: 0,
  requestId: null,
  streams: INITIAL_STREAMS,
};

type Piece = { kind: ConsoleKind; style: AnsiStyle; text: string };

// 末尾に文字列を足し、上限を超えた分を古いほうから消す。
function appendPieces(
  log: ConsoleLog,
  pieces: readonly Piece[],
  changes: Pick<ConsoleLog, "requestId" | "streams">,
): ConsoleLog {
  const segments = log.segments.slice();
  let { length, nextId } = log;
  for (const piece of pieces) {
    if (piece.text === "") {
      continue;
    }
    length += piece.text.length;
    const last = segments[segments.length - 1];
    if (
      last !== undefined &&
      last.kind === piece.kind &&
      sameStyle(last.style, piece.style)
    ) {
      segments[segments.length - 1] = { ...last, text: last.text + piece.text };
    } else {
      segments.push({ id: nextId++, ...piece });
    }
  }

  let truncated = log.truncated;
  let first = 0;
  if (length > CONSOLE_TAIL_LIMIT) {
    truncated = true;
    let excess = length - CONSOLE_TAIL_LIMIT;
    while (segments[first].text.length <= excess) {
      excess -= segments[first].text.length;
      length -= segments[first].text.length;
      first++;
    }
    if (excess > 0) {
      const { text } = segments[first];
      // サロゲートペアの途中で切らない。
      const low = text.charCodeAt(excess);
      const cut = low >= 0xdc00 && low <= 0xdfff ? excess + 1 : excess;
      length -= cut;
      if (cut < text.length) {
        segments[first] = { ...segments[first], text: text.slice(cut) };
      } else {
        first++;
      }
    }
  }
  return {
    segments: first > 0 ? segments.slice(first) : segments,
    length,
    truncated,
    nextId,
    ...changes,
  };
}

// run_output の断片を足す。ANSI の色の状態は、実行・提出ごと、種類ごとに持つ。
export function appendRunOutput(
  log: ConsoleLog,
  output: RunOutput,
): ConsoleLog {
  // 別の実行・提出の出力になったら、前の読みかけのエスケープと色を持ち越さない。
  const streams: Streams =
    output.request_id === log.requestId
      ? { ...log.streams }
      : { ...INITIAL_STREAMS };
  const pieces: Piece[] = [];
  for (const chunk of output.chunks) {
    const parsed = parseAnsi(streams[chunk.stream], chunk.text);
    streams[chunk.stream] = parsed.state;
    for (const styled of parsed.pieces) {
      pieces.push({
        kind: chunk.stream,
        style: styled.style,
        text: styled.text,
      });
    }
  }
  return appendPieces(log, pieces, { requestId: output.request_id, streams });
}

// 学習者が入力した行（send_stdin で送った文字列）を足す。エスケープは読まず、そのまま表示する。
export function appendInput(log: ConsoleLog, text: string): ConsoleLog {
  return appendPieces(log, [{ kind: "input", style: PLAIN_STYLE, text }], {
    requestId: log.requestId,
    streams: log.streams,
  });
}
