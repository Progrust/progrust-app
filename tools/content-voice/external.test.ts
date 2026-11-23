// VOICEPEAK・ffmpeg・ffprobe の呼び出し（external.ts）のテスト。実物の代わりに、偽の sh のスクリプトを使う。
import {
  chmodSync,
  existsSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { afterEach, describe, expect, it } from "vitest";
import { createTools } from "./external.ts";
import { serializeWav } from "./wav.ts";

const VOICE = {
  narrator: "Natsuki Karin",
  speed: 120,
  pause: 0.2,
  emotion: { hightension: 20 },
};

const dirs: string[] = [];

afterEach(() => {
  for (const dir of dirs.splice(0)) {
    rmSync(dir, { recursive: true, force: true });
  }
});

// `failures` 回だけ失敗してから、用意した WAV を出力先（最後の引数）に写す、偽の VOICEPEAK。
function fakeVoicepeak(failures: number) {
  const dir = mkdtempSync(join(tmpdir(), "progrust-fake-voicepeak-"));
  dirs.push(dir);
  const wav = join(dir, "speech.wav");
  writeFileSync(
    wav,
    serializeWav({
      sampleRate: 8000,
      channels: 1,
      samples: Int16Array.from([1, 2, 3, 4]),
    }),
  );
  const count = join(dir, "count");
  writeFileSync(count, "0");
  const script = join(dir, "voicepeak");
  writeFileSync(
    script,
    [
      "#!/bin/sh",
      `n=$(cat '${count}')`,
      `echo $((n + 1)) > '${count}'`,
      `if [ "$n" -lt ${failures} ]; then echo "failed $n" >&2; exit 3; fi`,
      "for last; do :; done",
      `cp '${wav}' "$last"`,
      "",
    ].join("\n"),
  );
  chmodSync(script, 0o755);
  return {
    script,
    out: join(dir, "out.wav"),
    attempts: () => Number(readFileSync(count, "utf8")),
  };
}

describe("createTools", () => {
  it("VOICEPEAK が失敗したら、3 回まで作り直す", () => {
    const fake = fakeVoicepeak(2);
    const logs: string[] = [];
    const tools = createTools(
      { voicepeak: fake.script, ffmpeg: "ffmpeg", ffprobe: "ffprobe" },
      (line) => logs.push(line),
    );
    tools.synthesize("はい。", VOICE, fake.out);
    expect(fake.attempts()).toBe(3);
    expect(logs).toEqual([
      "VOICEPEAK が失敗した（終了コード 3、failed 0）。作り直す",
      "VOICEPEAK が失敗した（終了コード 3、failed 1）。作り直す",
    ]);
    expect(existsSync(fake.out)).toBe(true);
  });

  it("3 回とも失敗したら、エラーにする", () => {
    const fake = fakeVoicepeak(3);
    const tools = createTools(
      { voicepeak: fake.script, ffmpeg: "ffmpeg", ffprobe: "ffprobe" },
      () => {},
    );
    expect(() => tools.synthesize("はい。", VOICE, fake.out)).toThrow(
      "VOICEPEAK が失敗した（終了コード 3、failed 2）（3 回試した）",
    );
    expect(existsSync(fake.out)).toBe(false);
  });

  it("VOICEPEAK・ffmpeg・ffprobe が見つからないときは、エラーにする", () => {
    const tools = createTools({
      voicepeak: "/nonexistent/voicepeak",
      ffmpeg: "/nonexistent/ffmpeg",
      ffprobe: "/nonexistent/ffprobe",
    });
    expect(() => tools.synthesize("はい。", VOICE, "x.wav")).toThrow(
      "VOICEPEAK が見つからない",
    );
    expect(() => tools.encode("x.wav", "x.m4a")).toThrow(
      "ffmpeg が見つからない",
    );
    expect(() => tools.measureSeconds("x.m4a")).toThrow(
      "ffprobe が見つからない",
    );
  });
});
