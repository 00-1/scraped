// Headless test of the Android app's page on an emulated phone (Pixel 7):
// make a world, play by typing and by chips, keep the keyboard focus after
// sending, notebook, menus, appearance, an agent's moves, and back.
//   node tools/smoke/app.cjs   (after tools/build.sh)
const path = require('path');
let pw;
try { pw = require('playwright'); } catch { pw = require(process.env.PLAYWRIGHT_PATH || '/opt/node22/lib/node_modules/playwright'); }
const { chromium, devices } = pw;
(async () => {
  const browser = await chromium.launch();
  const context = await browser.newContext({ ...devices['Pixel 7'] });
  const page = await context.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  page.on('console', m => { if (m.type() === 'error') errors.push(m.text()); });
  await page.goto('file://' + path.resolve(__dirname, '../dist/app.html'));
  await page.waitForSelector('#loading', { state: 'hidden', timeout: 30000 });
  if (!(await page.textContent('#worldsList')).trim()) throw new Error('empty worlds list shows nothing');
  // A new world.
  await page.tap('#newWorld');
  await page.waitForSelector('.sheet [data-diff="gentle"]');
  await page.tap('.sheet [data-diff="gentle"]');
  await page.tap('#beginBtn');
  await page.waitForSelector('#gameScreen:not([hidden]) .game', { timeout: 30000 });
  // Type a command; the box keeps focus after sending.
  await page.tap('#input');
  await page.keyboard.type('look');
  await page.tap('#send');
  await page.waitForFunction(() => document.querySelectorAll('#entries .me').length === 1);
  if (await page.evaluate(() => document.activeElement.id) !== 'input') throw new Error('focus left the composer after send');
  if (!(await page.evaluate(() => document.querySelector('#send').disabled))) throw new Error('send should be disabled when empty');
  // Enter sends too; recall brings it back.
  await page.keyboard.type('inventory');
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => document.querySelectorAll('#entries .me').length === 2);
  await page.tap('#recall');
  if (await page.inputValue('#input') !== 'inventory') throw new Error('recall');
  await page.fill('#input', '');
  // Chips: one-tap commands, and verb + thing composition.
  await page.tap('.chip[data-cmd="status"]');
  await page.waitForFunction(() => document.querySelectorAll('#entries .me').length === 3);
  const verb = await page.$('.chip[data-verb="read"]');
  await verb.scrollIntoViewIfNeeded(); await verb.tap();
  if (!(await page.inputValue('#input')).startsWith('read ')) throw new Error('verb chip');
  await page.fill('#input', ''); await page.dispatchEvent('#input', 'input');
  // The newest text is in view.
  const atEnd = await page.evaluate(() => { const t = document.querySelector('#transcript'); return t.scrollHeight - t.scrollTop - t.clientHeight < 90; });
  if (!atEnd) throw new Error('not scrolled to the newest text');
  // Tap a passage: copy and notebook.
  await page.tap('#entries .game >> nth=-1');
  await page.waitForSelector('[data-pass="note"]');
  await page.tap('[data-pass="note"]');
  await page.tap('#openNotebook');
  await page.waitForSelector('#notebookScreen:not([hidden])');
  if (!(await page.inputValue('#notebookText')).startsWith('> ')) throw new Error('passage not in notebook');
  await page.fill('#notebookText', 'glyph 4 might be ka');
  await page.evaluate(() => window.__back());
  // An agent plays: its move shows as an agent bubble; it reads only text.
  const reply = await page.evaluate(() => window.__agentAct('look'));
  if (typeof reply !== 'string' || !reply.length) throw new Error('agent act returned nothing');
  await page.waitForSelector('#entries .me.agent');
  const read = await page.evaluate(() => window.__agentRead(4));
  if (!read.includes('> look')) throw new Error('agent read: ' + read.slice(0, 80));
  // Menus and appearance.
  await page.tap('#openWorldMenu');
  await page.waitForSelector('.sheet [data-wact="share"]');
  await page.evaluate(() => window.__back());
  // Back to the list: the world is there with its notebook kept.
  await page.evaluate(() => window.__back());
  await page.waitForSelector('#worldsScreen:not([hidden]) .world');
  const saved = await page.evaluate(() => JSON.parse(localStorage.getItem(Object.keys(localStorage).find(k => k.startsWith('app.w_')))));
  if (saved.notebook !== 'glyph 4 might be ka') throw new Error('notebook not saved: ' + saved.notebook);
  if (saved.transcript.filter(e => e.k === 'a').length !== 1) throw new Error('agent move not recorded');
  await page.tap('#openAppearance');
  await page.tap('.sheet [data-val="dark"]');
  if (await page.evaluate(() => document.documentElement.dataset.theme) !== 'dark') throw new Error('theme');
  await page.evaluate(() => window.__back());
  await page.tap('#openIntegrations');
  await page.waitForSelector('#integrations .card');
  await page.evaluate(() => window.__back());
  // Reopen the world: it replays to the same place.
  await page.tap('.world');
  await page.waitForSelector('#gameScreen:not([hidden])');
  const n = await page.evaluate(() => document.querySelectorAll('#entries .me').length);
  if (n !== 4) throw new Error('reopened with ' + n + ' commands');
  await page.screenshot({ path: process.env.SHOT || '/tmp/app.png' });
  await browser.close();
  if (errors.length) throw new Error('page errors: ' + errors.join('; '));
  console.log('app smoke test passed');
})().catch(e => { console.error(e.message); process.exit(1); });
