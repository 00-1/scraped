// Writes the Android app's strings.xml from Jb's app.label slot, so even
// the name under the icon is his text. Run by android/build.sh.
//   node tools/smoke/labels.cjs out/strings.xml
const fs = require('fs');
const path = require('path');
(async () => {
  const root = path.resolve(__dirname, '../..');
  const engine = (await WebAssembly.instantiate(fs.readFileSync(path.join(root, 'tools/dist/engine.wasm')), {})).instance.exports;
  const api = req => {
    const b = new TextEncoder().encode(JSON.stringify(req));
    const ptr = engine.alloc(b.length);
    new Uint8Array(engine.memory.buffer, ptr, b.length).set(b);
    const out = engine.call(ptr, b.length);
    return JSON.parse(new TextDecoder().decode(new Uint8Array(engine.memory.buffer, out, engine.output_len())));
  };
  const dir = path.join(root, 'content');
  const files = fs.readdirSync(dir).filter(f => f.endsWith('.toml')).sort().map(f => ({ path: f, text: fs.readFileSync(path.join(dir, f), 'utf8') }));
  const L = api({ cmd: 'app_labels', files });
  const xml = s => s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/'/g, "\\'").replace(/"/g, '\\"');
  fs.writeFileSync(process.argv[2], `<?xml version="1.0" encoding="utf-8"?>\n<resources>\n    <string name="app_name">${xml(L.app_name)}</string>\n</resources>\n`);
})().catch(e => { console.error(e.message); process.exit(1); });
