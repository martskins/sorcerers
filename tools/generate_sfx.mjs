import { mkdirSync, writeFileSync } from 'node:fs';

const rate = 44100;
const out = 'assets/sounds';
mkdirSync(out, { recursive: true });

function wav(name, notes, noise = 0) {
  const seconds = notes.at(-1).at + notes.at(-1).len + 0.12;
  const samples = new Int16Array(Math.ceil(seconds * rate));
  for (let i = 0; i < samples.length; i++) {
    const t = i / rate;
    let value = 0;
    for (const n of notes) {
      const local = t - n.at;
      if (local < 0 || local > n.len) continue;
      const envelope = Math.min(1, local / 0.008) * Math.exp(-local * n.decay);
      value += Math.sin(2 * Math.PI * n.freq * local) * n.gain * envelope;
      value += Math.sin(2 * Math.PI * n.freq * 2.01 * local) * n.gain * 0.18 * envelope;
    }
    if (noise && t < 0.06) value += (Math.random() * 2 - 1) * noise * Math.exp(-t * 55);
    samples[i] = Math.max(-1, Math.min(1, value)) * 32767;
  }
  const header = Buffer.alloc(44);
  header.write('RIFF', 0); header.writeUInt32LE(36 + samples.byteLength, 4);
  header.write('WAVEfmt ', 8); header.writeUInt32LE(16, 16); header.writeUInt16LE(1, 20);
  header.writeUInt16LE(1, 22); header.writeUInt32LE(rate, 24); header.writeUInt32LE(rate * 2, 28);
  header.writeUInt16LE(2, 32); header.writeUInt16LE(16, 34); header.write('data', 36);
  header.writeUInt32LE(samples.byteLength, 40);
  writeFileSync(`${out}/${name}.wav`, Buffer.concat([header, Buffer.from(samples.buffer)]));
}

wav('button_click', [{ at: 0, len: .08, freq: 740, gain: .19, decay: 35 }]);
wav('select', [{ at: 0, len: .12, freq: 520, gain: .18, decay: 18 }, { at: .045, len: .13, freq: 780, gain: .15, decay: 18 }]);
wav('confirm', [{ at: 0, len: .15, freq: 440, gain: .18, decay: 13 }, { at: .07, len: .2, freq: 660, gain: .2, decay: 12 }]);
wav('error', [{ at: 0, len: .18, freq: 180, gain: .2, decay: 12 }, { at: .08, len: .2, freq: 145, gain: .18, decay: 12 }]);
wav('turn_start', [{ at: 0, len: .24, freq: 330, gain: .15, decay: 10 }, { at: .09, len: .25, freq: 495, gain: .18, decay: 10 }, { at: .18, len: .3, freq: 660, gain: .16, decay: 9 }]);
wav('card_play', [{ at: 0, len: .13, freq: 110, gain: .28, decay: 25 }, { at: .035, len: .22, freq: 220, gain: .13, decay: 15 }], .12);
wav('site_play', [{ at: 0, len: .22, freq: 76, gain: .32, decay: 13 }, { at: .035, len: .25, freq: 152, gain: .14, decay: 13 }], .08);
