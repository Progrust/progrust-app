// ch04-04 所有権のムーブ（見本の教材）の動画の入口。
import { ErrorScene } from "./ErrorScene";
import { MoveScene } from "./MoveScene";
import { SummaryScene } from "./SummaryScene";
import { TitleScene } from "./TitleScene";

export default {
  title: TitleScene,
  move: MoveScene,
  error: ErrorScene,
  summary: SummaryScene,
};
