// テスト用のレッスンの部品。index.tsx から読み込む。
export function Panel({ text }: { text: string }) {
  return (
    <div className="box" data-part="PART-MARKER-beta">
      {text}
    </div>
  );
}
