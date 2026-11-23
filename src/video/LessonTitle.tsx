// 表紙のレッスン名（documents/spec/content-format/video/scenes.md「共通部品」）。
// lesson.json の title を、表紙のステージの h1 として出す。
import { useContext, useLayoutEffect } from "react";
import { LessonTitleCountContext, useLesson } from "./hooks";

export function LessonTitle() {
  const { lessonTitle } = useLesson();
  // 教材チェックツールの検査「表紙」が、表示している数を数える。
  const count = useContext(LessonTitleCountContext);
  useLayoutEffect(() => {
    if (count === null) {
      return;
    }
    count(1);
    return () => count(-1);
  }, [count]);
  return <h1>{lessonTitle}</h1>;
}
