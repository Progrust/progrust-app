// 動画の台紙の下に出す字幕（design-system/progrust/components/Captions、
// documents/spec/screens/video-screen.md）。
// 表示しないときも、中身のない字幕を置いて場所を空けておく（ADR 0171）。

export type CaptionsProps = {
  // 再生中の文の text。null のときは表示しない。
  text: string | null;
};

export function Captions({ text }: CaptionsProps) {
  if (text === null) {
    return <p className="pg-captions" aria-hidden="true" />;
  }
  return <p className="pg-captions">{text}</p>;
}
