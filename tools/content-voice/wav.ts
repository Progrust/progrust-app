// 音声を作るツール（documents/spec/content-format/video/voice.md）が、VOICEPEAK の出力の WAV を読み書きする。
// 扱うのは、VOICEPEAK が出力する形（リニア PCM、16 ビット）だけとする。

export interface Wav {
  sampleRate: number;
  channels: number;
  // チャンネルを交互に並べたサンプル（-32768〜32767）。
  samples: Int16Array;
}

function chunkId(buffer: Buffer, offset: number): string {
  return buffer.toString("ascii", offset, offset + 4);
}

export function parseWav(buffer: Buffer): Wav {
  if (
    buffer.length < 12 ||
    chunkId(buffer, 0) !== "RIFF" ||
    chunkId(buffer, 8) !== "WAVE"
  ) {
    throw new Error("WAV のファイルでない");
  }
  let format: { channels: number; sampleRate: number } | undefined;
  let offset = 12;
  while (offset + 8 <= buffer.length) {
    const id = chunkId(buffer, offset);
    const size = buffer.readUInt32LE(offset + 4);
    const body = offset + 8;
    if (id === "fmt ") {
      const audioFormat = buffer.readUInt16LE(body);
      const bitsPerSample = buffer.readUInt16LE(body + 14);
      if (audioFormat !== 1 || bitsPerSample !== 16) {
        throw new Error(
          `WAV の形式が 16 ビットのリニア PCM でない（形式 ${audioFormat}、${bitsPerSample} ビット）`,
        );
      }
      format = {
        channels: buffer.readUInt16LE(body + 2),
        sampleRate: buffer.readUInt32LE(body + 4),
      };
    } else if (id === "data") {
      if (format === undefined)
        throw new Error("WAV の fmt が data より後にある");
      const end = Math.min(body + size, buffer.length);
      const count = Math.floor((end - body) / 2);
      const samples = new Int16Array(count);
      for (let i = 0; i < count; i++)
        samples[i] = buffer.readInt16LE(body + i * 2);
      return { ...format, samples };
    }
    // チャンクの大きさが奇数のときは、1 バイトの詰め物が続く。
    offset = body + size + (size % 2);
  }
  throw new Error("WAV に data がない");
}

export function serializeWav(wav: Wav): Buffer {
  const dataSize = wav.samples.length * 2;
  const buffer = Buffer.alloc(44 + dataSize);
  buffer.write("RIFF", 0, "ascii");
  buffer.writeUInt32LE(36 + dataSize, 4);
  buffer.write("WAVE", 8, "ascii");
  buffer.write("fmt ", 12, "ascii");
  buffer.writeUInt32LE(16, 16);
  buffer.writeUInt16LE(1, 20);
  buffer.writeUInt16LE(wav.channels, 22);
  buffer.writeUInt32LE(wav.sampleRate, 24);
  buffer.writeUInt32LE(wav.sampleRate * wav.channels * 2, 28);
  buffer.writeUInt16LE(wav.channels * 2, 32);
  buffer.writeUInt16LE(16, 34);
  buffer.write("data", 36, "ascii");
  buffer.writeUInt32LE(dataSize, 40);
  for (let i = 0; i < wav.samples.length; i++) {
    buffer.writeInt16LE(wav.samples[i], 44 + i * 2);
  }
  return buffer;
}

// 1 チャンネルあたりのサンプル数（フレーム数）。
export function frameCount(wav: Wav): number {
  return Math.floor(wav.samples.length / wav.channels);
}

// 秒で表した長さ。
export function wavSeconds(wav: Wav): number {
  return frameCount(wav) / wav.sampleRate;
}
