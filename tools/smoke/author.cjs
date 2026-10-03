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
  await page.waitForFunction(() => document.querySelector('[data-issues]').textContent.includes('nonsense'), null, { timeout: 30000 });
  await page.fill(textarea, '{a stroke} pointing {turn}, at the {spot}');
  await page.waitForFunction(() => document.querySelector('#preview td.out')?.textContent.includes('pointing'), null, { timeout: 30000 });
  const preview = await page.textContent('#preview td.out');
  const status = await page.textContent('#packStatus');
  // A fact slot seen in whole responses, as the attention model assembles them.
  await page.click('[data-slot="place.whole"]');
  await page.waitForSelector('#inContext td.out', { timeout: 30000 });
  // A new storylet, placed in a temple, previewed in a real world.
  await page.fill('#newStoryletId', 'smoke well');
  await page.click('#newStorylet button');
  await page.waitForSelector('#storyletCard');
  await page.selectOption('#s-at', 'structure');
  await page.waitForSelector('[data-sfield="place.structure"][value="temple"]');
  await page.check('[data-sfield="place.structure"][value="temple"]');
  await page.selectOption('#s-ins', 'everyday');
  await page.waitForSelector('#s-ins-about');
  await page.fill('#s-ins-about', 'water');
  await page.fill('textarea[data-field=text]', 'A temple, and {inscription}.');
  await page.click('#placeIt');
  await page.waitForFunction(() => /temple/.test(document.querySelector('#placement').textContent), null, { timeout: 20000 });
  const placement = await page.textContent('#placement');
  if (!/Its writing/.test(placement)) throw new Error('no writing in placement: ' + placement);
  const chips = await page.textContent('#beats');
  if (!chips.includes('opening')) throw new Error('beats missing: ' + chips);
  // Playtest: click a line, see why, edit it, see the run update.
  await page.click('[data-mode=play]');
  await page.waitForSelector('#playLog button.line', { timeout: 30000 });
  await page.fill('#playInput', 'help');
  await page.press('#playInput', 'Enter');
  await page.waitForFunction(() => document.querySelector('#playLog').textContent.includes('> help'));
  const helpLine = await page.$$eval('#playLog .entry:last-child button.line', els => els.map(e => e.dataset.line));
  await page.click(`#playLog .entry:last-child button.line`);
  await page.waitForSelector('#why .whybox');
  const why = await page.textContent('#why');
  if (!why.includes('say.help')) throw new Error('why box: ' + why + ' ' + helpLine);
  await page.waitForSelector('#playEditor .variant textarea');
  await page.fill('#playEditor .variant textarea[data-field=text]', 'HOT RELOADED HELP');
  await page.waitForFunction(() => document.querySelector('#playLog').textContent.includes('HOT RELOADED HELP'), null, { timeout: 30000 });
  // Gaps, voice, inspect, changes.
  await page.click('[data-mode=gaps]');
  await page.fill('#gapSeeds', '1');
  await page.fill('#gapSteps', '20');
  await page.click('#runGaps');
  await page.waitForFunction(() => /runs, .* hours of play/.test(document.querySelector('#gapOut').textContent), null, { timeout: 60000 });
  await page.click('[data-mode=voice]');
  await page.waitForSelector('#modeView h3');
  await page.click('[data-mode=inspect]');
  await page.waitForSelector('.glyphs figure svg', { timeout: 30000 });
  await page.click('[data-inspect=world]');
  await page.waitForSelector('#inspMap', { timeout: 30000 });
  await page.click('[data-inspect=run]');
  await page.waitForFunction(() => document.querySelector('#inspectOut').textContent.includes('texts'));
  await page.click('[data-mode=changes]');
  await page.waitForFunction(() => document.querySelector('#modeView').textContent.includes('HOT RELOADED HELP'));
  await page.click('[data-mode=write]');
  await page.check('#reviewMode');
  const firstFamily = await page.textContent('.family');
  if (!firstFamily.startsWith('story')) throw new Error('review order starts with ' + firstFamily);
  await browser.close();
  if (errors.length) throw new Error('page errors: ' + errors.join('; '));
  if (!status.includes('Unsaved changes')) throw new Error('status did not notice the edit: ' + status);
  console.log('authoring tool smoke test passed:', preview.trim());
})().catch(e => { console.error(e.message); process.exit(1); });
