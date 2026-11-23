// 寄せ（documents/spec/content-format/video/scenes.md「共通部品」、
// design-system/progrust-video/README.md「寄せ方」）。
import type { CSSProperties, ReactNode } from "react";
import { easeInOut, FRAMES, lerp, progress } from "./motion";

// 映す範囲。基準サイズ（1280×720）のうち、左上の位置（基準の px）と倍率で決める。
// 映る幅は 1280 ÷ scale、高さは 720 ÷ scale。
export type CameraView = {
  x: number;
  y: number;
  scale: number;
};

export type CameraProps = Partial<CameraView> & { children: ReactNode };

// 寄せない（1 倍）。
export const NO_ZOOM: CameraView = { x: 0, y: 0, scale: 1 };

// x・y・倍率を同時に、同じ進み具合で補間する（寄せを変える動き。既定は 24 フレーム）。
export function cameraAt(
  frame: number,
  from: CameraView,
  to: CameraView,
  start: number,
  duration: number = FRAMES.camera,
): CameraView {
  const t = progress(frame, start, duration, easeInOut);
  return {
    x: lerp(from.x, to.x, t),
    y: lerp(from.y, to.y, t),
    scale: lerp(from.scale, to.scale, t),
  };
}

// ステージ（.video-stage）を .video-camera で包み、映す範囲を CSS 変数で渡す。
export function Camera({
  x = NO_ZOOM.x,
  y = NO_ZOOM.y,
  scale = NO_ZOOM.scale,
  children,
}: CameraProps) {
  const style = {
    "--camera-x": x,
    "--camera-y": y,
    "--camera-scale": scale,
  } as CSSProperties;
  return (
    <div className="video-camera" style={style}>
      {children}
    </div>
  );
}
