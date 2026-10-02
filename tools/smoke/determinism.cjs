// WebAssembly determinism: the engine running as WebAssembly must give the
// same transcript hashes as the native test (crates/game/tests/transcripts.txt).
// Run after tools/build.sh:  node tools/smoke/determinism.cjs
const fs = require('fs');
const path = require('path');
(async () => {
  const root = path.resolve(__dirname, '../..');
  const bytes = fs.readFileSync(path.join(root, 'tools/dist/engine.wasm'));
  const engine = (await WebAssembly.instantiate(bytes, {})).instance.exports;
  const api = req => {
    const b = new TextEncoder().encode(JSON.stringify(req));
    const ptr = engine.alloc(b.length);
    new Uint8Array(engine.memory.buffer, ptr, b.length).set(b);
    const out = engine.call(ptr, b.length);
    return JSON.parse(new TextDecoder().decode(new Uint8Array(engine.memory.buffer, out, engine.output_len())));
  };
  const dir = path.join(root, 'content');
  const files = fs.readdirSync(dir).filter(f => f.endsWith('.toml')).sort()
    .map(f => ({ path: f, text: fs.readFileSync(path.join(dir, f), 'utf8').replace(/\r\n/g, '\n') }));
  const want = fs.readFileSync(path.join(root, 'crates/game/tests/transcripts.txt'), 'utf8').trim().split(/\r?\n/);
  for (const line of want) {
    const [seed, difficulty, hash] = line.split(' ');
    const r = api({ cmd: 'transcript_hash', files, seed, difficulty, steps: 40 });
    if (r.hash !== hash) throw new Error(`seed ${seed} ${difficulty}: wasm ${r.hash || JSON.stringify(r)} != native ${hash}`);
  }
  console.log(`wasm determinism passed (${want.length} runs)`);
})().catch(e => { console.error(e.message); process.exit(1); });
