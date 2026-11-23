import { expect, it } from "vitest";
import { cameraAt } from "./Camera";
import {
  easeInOut,
  easeOut,
  enter,
  exit,
  lerp,
  progress,
  sceneOpacity,
  stagger,
} from "./motion";

it("progress は、開始前は 0、終わった後は 1、間はイージングに従う", () => {
  expect(progress(9, 10, 20)).toBe(0);
  expect(progress(10, 10, 20)).toBe(0);
  expect(progress(20, 10, 20)).toBeCloseTo(0.5);
  expect(progress(30, 10, 20)).toBe(1);
  expect(progress(99, 10, 20)).toBe(1);
  expect(progress(15, 10, 10, easeOut)).toBeGreaterThan(0.5);
});

it("イージングは、端が 0 と 1 で、増え続ける", () => {
  for (const ease of [easeOut, easeInOut]) {
    expect(ease(0)).toBe(0);
    expect(ease(1)).toBe(1);
    let prev = 0;
    for (let i = 1; i <= 20; i++) {
      const v = ease(i / 20);
      expect(v).toBeGreaterThanOrEqual(prev);
      prev = v;
    }
  }
  expect(easeInOut(0.5)).toBeCloseTo(0.5);
});

it("lerp は、端の値と中間の値を返す", () => {
  expect(lerp(10, 20, 0)).toBe(10);
  expect(lerp(10, 20, 1)).toBe(20);
  expect(lerp(10, 20, 0.5)).toBe(15);
});

it("enter は、不透明度 0 → 1 と、16px 上がる動きを返す", () => {
  expect(enter(5, 10)).toEqual({ opacity: 0, translateY: 16 });
  expect(enter(26, 10)).toEqual({ opacity: 1, translateY: 0 });
  const mid = enter(18, 10);
  expect(mid.opacity).toBeGreaterThan(0);
  expect(mid.opacity).toBeLessThan(1);
  expect(mid.translateY).toBeCloseTo(16 * (1 - mid.opacity));
});

it("exit は、不透明度 1 → 0 を返す", () => {
  expect(exit(0, 4).opacity).toBe(1);
  expect(exit(12, 4).opacity).toBe(0);
});

it("stagger は、要素ごとに開始をずらす", () => {
  expect(stagger(10, 0)).toBe(10);
  expect(stagger(10, 3)).toBe(28);
  expect(stagger(10, 2, 4)).toBe(18);
});

it("sceneOpacity は、出入りを 10 フレームでフェードし、最後のシーンは消さない", () => {
  expect(sceneOpacity(0, 100)).toBe(0);
  expect(sceneOpacity(50, 100)).toBe(1);
  expect(sceneOpacity(100, 100)).toBe(0);
  expect(sceneOpacity(100, 100, { last: true })).toBe(1);
});

it("cameraAt は、x・y・倍率を同時に補間する", () => {
  const from = { x: 0, y: 0, scale: 1 };
  const to = { x: 22, y: 10, scale: 1.16 };
  expect(cameraAt(0, from, to, 10)).toEqual(from);
  expect(cameraAt(34, from, to, 10)).toEqual(to);
  const mid = cameraAt(22, from, to, 10);
  expect(mid.x).toBeCloseTo(11);
  expect(mid.y).toBeCloseTo(5);
  expect(mid.scale).toBeCloseTo(1.08);
});

it("同じフレームなら、何度呼んでも、順番を変えても同じ結果になる", () => {
  const from = { x: 0, y: 0, scale: 1 };
  const to = { x: 22, y: 10, scale: 1.16 };
  const frames = [30, 5, 18, 40, 18, 5];
  const first = frames.map((f) => [
    cameraAt(f, from, to, 4),
    enter(f, 4),
    exit(f, 4),
    progress(f, 4, 20, easeInOut),
    sceneOpacity(f, 60),
  ]);
  const second = [...frames]
    .reverse()
    .map((f) => [
      cameraAt(f, from, to, 4),
      enter(f, 4),
      exit(f, 4),
      progress(f, 4, 20, easeInOut),
      sceneOpacity(f, 60),
    ]);
  expect(second.reverse()).toEqual(first);
});
