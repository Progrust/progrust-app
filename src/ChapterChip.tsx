// 画面上部の文脈に置く、章の番号と名前のチップ（design-system/progrust/components/ChapterBadge/README.md）。
// Rust の章は、番号の丸（「4」）と章名を並べる。C の章は、「C 第1章 C の基本」の形の文字にする。
// 章の色は、置いた範囲の data-chapter で決まる。
import { chapterBadge, chapterLabel } from "./chapterNumbers";
import type { Language } from "./commands/content";
import "./chapterChip.css";

export type ChipChapter = {
  language: Language;
  number: number;
  title: string;
};

export function ChapterChip({ chapter }: { chapter: ChipChapter }) {
  if (chapter.language === "c") {
    return (
      <span className="pg-chip pgx-chip-text">
        {chapterLabel(chapter.language, chapter.number)} {chapter.title}
      </span>
    );
  }
  return (
    <span className="pg-chip">
      <span className="pg-chip-num">
        {chapterBadge(chapter.language, chapter.number)}
      </span>
      {chapter.title}
    </span>
  );
}
