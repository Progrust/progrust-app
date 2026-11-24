// 判定結果のカード（documents/spec/screens/exercise-screen.md「判定結果」、
// design-system/progrust/components/JudgeResult、design-system/ui-design/Exercise.dc.html）。
// 左の列の問題文の上に置く。不正解では見出しだけを出し、「詳細を見る」で失敗したテストケースと、
// 最初に失敗したテストケースの detail を開閉する。次の提出では作り直すので（key を変える）、畳んだ状態に戻る。
import { Check, ChevronDown, Clock, TriangleAlert, X } from "lucide-react";
import { useId, useState } from "react";
import type { CaseResult } from "../commands/run";
import {
  firstFailedCase,
  isStdoutFailDetail,
  visibleWhitespace,
  wrongSubText,
} from "./judgeResult";
import type { JudgeCardState } from "./judgeResult";

export function JudgeCard({ card }: { card: JudgeCardState }) {
  const { result } = card;
  if (result.status === "compile_error") {
    return (
      <section
        className="pg-judge pg-judge--warn exercise-judge"
        aria-label="判定結果"
      >
        <div className="pg-judge-head">
          <span className="pg-judge-mark">
            <TriangleAlert
              className="pg-icon"
              aria-hidden="true"
              size={18}
              strokeWidth={2.2}
            />
          </span>
          <div>
            <p className="pg-judge-title">ビルドに失敗しました</p>
            <p className="pg-judge-sub">
              テストは実行していません。エラーはコンソールに表示しています。
            </p>
          </div>
        </div>
      </section>
    );
  }
  return <WrongCard card={card} />;
}

function WrongCard({ card }: { card: JudgeCardState }) {
  const [open, setOpen] = useState(false);
  const detailId = useId();
  const { result } = card;
  const failed = firstFailedCase(result);
  return (
    <section className="pg-judge exercise-judge" aria-label="判定結果">
      <div className="pg-judge-head">
        <span className="pg-judge-mark">
          <X className="pg-icon" aria-hidden="true" size={20} strokeWidth={3} />
        </span>
        <div>
          <p className="pg-judge-title">不正解</p>
          <p className="pg-judge-sub">
            {wrongSubText(result, card.submittedAt)}
          </p>
        </div>
        <button
          type="button"
          className="pg-judge-toggle"
          aria-expanded={open}
          aria-controls={detailId}
          onClick={() => setOpen((current) => !current)}
        >
          {open ? "詳細を閉じる" : "詳細を見る"}
          <ChevronDown
            className="pg-icon"
            aria-hidden="true"
            size={14}
            strokeWidth={2.4}
          />
        </button>
      </div>
      {open && (
        <div id={detailId} className="pg-judge-detail">
          <ul className="pg-cases" aria-label="テストケース">
            {result.cases.map((item, index) => (
              <CaseItem key={index} item={item} />
            ))}
          </ul>
          {failed !== null && <FailedCase item={failed} />}
        </div>
      )}
    </section>
  );
}

const CASE_STATUS_LABEL: Record<CaseResult["status"], string> = {
  pass: "通過",
  fail: "失敗",
  timeout: "時間切れ",
  runtime_error: "実行時エラー",
};

// テストの一覧の 1 行。色だけで区別しないよう、通ったものに check、時間切れに clock、ほかの失敗に x を付ける
// （失敗したものは太字になる）。
function CaseItem({ item }: { item: CaseResult }) {
  const iconProps = {
    className: "pg-icon",
    "aria-hidden": true,
    size: 12,
    strokeWidth: 3,
  } as const;
  return (
    <li className={item.status === "pass" ? "ok" : "ng"}>
      {item.status === "pass" ? (
        <Check {...iconProps} />
      ) : item.status === "timeout" ? (
        <Clock {...iconProps} />
      ) : (
        <X {...iconProps} />
      )}
      {item.name}
    </li>
  );
}

// 最初に失敗したテストケースの detail。stdout 方式の fail は入力・期待する出力・実際の出力、ほかは文字列。
function FailedCase({ item }: { item: CaseResult }) {
  const { detail } = item;
  if (isStdoutFailDetail(detail)) {
    return (
      <>
        <div className="pg-case">
          <p className="pg-case-title">{item.name}</p>
          <dl>
            <dt>入力</dt>
            <dd>
              <Visible text={detail.input} />
            </dd>
            <dt>期待する出力</dt>
            <dd>
              <Visible text={detail.expected} />
            </dd>
            <dt>実際の出力</dt>
            <dd className="diff">
              <Visible text={detail.actual} />
            </dd>
          </dl>
        </div>
        <p className="pg-judge-time">空白は ␣ で表示しています</p>
      </>
    );
  }
  return (
    <div className="pg-case">
      <p className="pg-case-title">{item.name}</p>
      {detail !== null && detail !== "" && (
        <dl>
          <dt>{CASE_STATUS_LABEL[item.status]}</dt>
          <dd>{detail}</dd>
        </dl>
      )}
    </div>
  );
}

// 行頭と行末の空白を ␣ で見えるようにした文字列。空のときは「（空）」と書く。
function Visible({ text }: { text: string }) {
  if (text === "") {
    return <span className="pg-ws">（空）</span>;
  }
  return (
    <>
      {visibleWhitespace(text).map((part, index) =>
        part.ws ? (
          <span key={index} className="pg-ws">
            {part.text}
          </span>
        ) : (
          part.text
        ),
      )}
    </>
  );
}
