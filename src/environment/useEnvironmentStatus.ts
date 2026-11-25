// 環境の状態（documents/spec/architecture/commands/environment.md）を読み、environment_updated で更新する。
import { useEffect, useState } from "react";
import {
  getEnvironmentStatus,
  listenEnvironmentUpdated,
} from "../commands/environment";
import type { EnvironmentStatus } from "../commands/environment";

// 環境の状態。読み込むまで、読めなかったときは null。
export function useEnvironmentStatus(): EnvironmentStatus | null {
  const [status, setStatus] = useState<EnvironmentStatus | null>(null);
  useEffect(() => {
    let cancelled = false;
    // イベントで受け取った状態は、後から返る get_environment_status の戻り値より新しい。
    let updated = false;
    let unlisten: (() => void) | null = null;
    listenEnvironmentUpdated((next) => {
      if (!cancelled) {
        updated = true;
        setStatus(next);
      }
    }).then(
      (fn) => {
        if (cancelled) {
          fn();
        } else {
          unlisten = fn;
        }
      },
      () => {},
    );
    getEnvironmentStatus().then(
      (next) => {
        if (!cancelled && !updated && next?.cargo !== undefined) {
          setStatus(next);
        }
      },
      () => {
        // 読めなくても、画面は使える。案内を出さない。
      },
    );
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, []);
  return status;
}

// cargo が見つかっているか（実行・テスト・提出を始められるか）。読み込むまでは、見つかっているものとする。
// version_too_old でも、無効にしない。
export function cargoUsable(status: EnvironmentStatus | null): boolean {
  return status === null || status.cargo.status !== "not_found";
}

// 許可クレートを使う演習（dependencies が空でない）を、外部クレートの準備の状態から始められるか。
// 許可クレートを使わない演習と、読み込むまでは、いつでも始められる。
export function cratesReadyFor(
  status: EnvironmentStatus | null,
  dependencies: readonly string[],
): boolean {
  return (
    dependencies.length === 0 ||
    status === null ||
    status.crates.status === "ready"
  );
}
