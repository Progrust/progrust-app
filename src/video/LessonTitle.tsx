// 表紙のレッスン名（documents/spec/content-format/video/scenes.md「共通部品」）。
// lesson.json の title を、表紙のステージの h1 として出す。
import { useLesson } from "./hooks";

export function LessonTitle() {
  const { lessonTitle } = useLesson();
  return <h1>{lessonTitle}</h1>;
}
