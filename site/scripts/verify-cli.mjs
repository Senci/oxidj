import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import assert from 'node:assert/strict';
const root = resolve('..');
const folder = resolve('.hallmark/final/cli-example');
mkdirSync(`${folder}/audio`, { recursive: true });
// Lawful generated fixture: 40ms of mono PCM16 silence, 44.1kHz.
const frames = 1764;
const wav = Buffer.alloc(44 + frames * 2);
wav.write('RIFF');
wav.writeUInt32LE(wav.length - 8, 4);
wav.write('WAVEfmt ', 8);
wav.writeUInt32LE(16, 16);
wav.writeUInt16LE(1, 20);
wav.writeUInt16LE(1, 22);
wav.writeUInt32LE(44100, 24);
wav.writeUInt32LE(88200, 28);
wav.writeUInt16LE(2, 32);
wav.writeUInt16LE(16, 34);
wav.write('data', 36);
wav.writeUInt32LE(frames * 2, 40);
const audio = `${folder}/audio/example.wav`;
writeFileSync(audio, wav);
const digest = (bytes) => createHash('sha256').update(bytes).digest('hex');
const before = digest(readFileSync(audio));
const db = `${folder}/catalog-${Date.now()}.sqlite`;
function run(args) {
  const result = spawnSync('cargo', ['run', '-p', 'oxidj-cli', '--', ...args], {
    cwd: root,
    encoding: 'utf8',
  });
  console.log(
    `cargo run -p oxidj-cli -- ${args.join(' ')}\n${result.stdout}${result.stderr}exit ${result.status}`,
  );
  assert.equal(result.status, 0);
  return result.stdout;
}
run(['catalog', 'init', '--db', db]);
assert.equal(run(['catalog', 'list', '--db', db]).trim(), 'TrackId\tPath\tSHA-256\tBytes');
assert.match(run(['catalog', 'import', '--db', db, '--dir', `${folder}/audio`]), /Imported/);
const listing = run(['catalog', 'list', '--db', db]);
assert.match(listing, /1\t.*example\.wav/);
assert.ok(listing.includes(before));
assert.equal(digest(readFileSync(audio)), before);
console.log(
  'CLI examples verified: init, empty list, recursive import, saved list; synthetic WAV unchanged.',
);
