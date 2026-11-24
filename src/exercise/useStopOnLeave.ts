// 画面を離れるときに、動いている実行・テスト実行・提出を止める
// （documents/spec/screens/exercise-screen.md、ADR 0202）。確認は出さない。
import { useEffect, useRef } from "react";

// activeRequestId: 動いている要求の ID（無いときは null）。stop: stop_run を呼ぶ関数。
// 演習画面を離れる（アンマウントする）ときの最新の ID で止める。
export function useStopOnLeave(
  activeRequestId: string | null,
  stop: (requestId: string) => void,
): void {
  const idRef = useRef(activeRequestId);
  const stopRef = useRef(stop);
  useEffect(() => {
    idRef.current = activeRequestId;
    stopRef.current = stop;
  });
  useEffect(
    () => () => {
      if (idRef.current !== null) {
        stopRef.current(idRef.current);
      }
    },
    [],
  );
}
