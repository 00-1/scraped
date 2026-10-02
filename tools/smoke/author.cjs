// Headless smoke test of the authoring tool: load, list slots, edit a
// variant, see lint and preview react. Run after tools/build.sh:
//   node tools/smoke/author.cjs
const path = require('path');
let chromium;
try { ({ chromium } = require('playwright')); } catch { ({ chromium } = require(process.env.PLAYWRIGHT_PATH || '/opt/node22/lib/node_modules/playwright')); }

(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.goto('file://' + path.resolve(__dirname, '../dist/author.html'));
  await page.waitForSelector('.slotlink');
  const slots = await page.$$eval('.slotlink', els => els.map(e => e.dataset.slot));
  if (!slots.includes('glyph.stroke')) throw new Error('slot list lacks glyph.stroke: ' + slots);
  await page.click('[data-slot="glyph.stroke"]');
  await page.waitForSelector('#preview tr');
  const textarea = 'textarea[data-field=text]';
  await page.fill(textarea, '{a stroke} pointing {turn}, at the {spot} {nonsense}');
  await page.waitForFunction(() => document.querySelector('[data-issues]').textContent.includes('nonsense'), null, { timeout: 5000 });
  await page.fill(textarea, '{a stroke} pointing {turn}, at the {spot}');
  await page.waitForFunction(() => document.querySelector('#preview td.out')?.textContent.includes('pointing'), null, { timeout: 5000 });
  const preview = await page.textContent('#preview td.out');
  const status = await page.textContent('#packStatus');
  await browser.close();
  if (errors.length) throw new Error('page errors: ' + errors.join('; '));
  if (!status.includes('Unsaved changes')) throw new Error('status did not notice the edit: ' + status);
  console.log('authoring tool smoke test passed:', preview.trim());
})().catch(e => { console.error(e.message); process.exit(1); });
