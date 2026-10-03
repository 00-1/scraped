// Headless smoke test of the bench: generate, switch seed, open the world.
const path = require('path');
let chromium;
try { ({ chromium } = require('playwright')); } catch { ({ chromium } = require(process.env.PLAYWRIGHT_PATH || '/opt/node22/lib/node_modules/playwright')); }
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage();
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.goto('file://' + path.resolve(__dirname, '../dist/bench.html'));
  await page.waitForFunction(() => document.querySelector('#status').textContent.startsWith('Seed'));
  await page.fill('#seed', '7');
  await page.click('button.primary');
  await page.waitForFunction(() => document.querySelector('#status').textContent.startsWith('Seed 7'));
  // The game: Play is the first tab.
  await page.waitForSelector('#playInput');
  await page.fill('#playInput', 'help');
  await page.press('#playInput', 'Enter');
  await page.waitForFunction(() => document.querySelector('#playLog').textContent.includes('> help'));
  await page.fill('#playInput', 'head north');
  await page.press('#playInput', 'Enter');
  await page.waitForFunction(() => document.querySelector('#playLog').textContent.includes('> head north'));
  await page.check('#spoil');
  await page.waitForSelector('#playMapWrap:not([hidden])');
  await page.fill('#playInput', 'status');
  await page.press('#playInput', 'Enter');
  await page.waitForFunction(() => document.querySelector('#playLog').textContent.includes('[body '));
  // A run that ends: the notebook appears, and the spoiler shows the ending.
  await page.check('#playLegacy');
  for (let i = 0; i < 40 && !(await page.$('#notebook h2')); i++) {
    await page.fill('#playInput', 'wait 3 days');
    await page.press('#playInput', 'Enter');
    await page.waitForTimeout(100);
  }
  await page.waitForSelector('#notebook h2', { timeout: 60000 });
  await page.waitForFunction(() => document.querySelector('#playLog').textContent.includes('[ending: death'));
  await page.click('[data-tab=world]');
  await page.waitForSelector('[data-site]', { timeout: 20000 });
  await page.click('[data-site="0"]');
  const site = await page.textContent('#worldText');
  await page.click('#writingView');
  await page.waitForFunction(() => document.querySelector('#worldText').textContent.startsWith('LIVE CLAIMS'));
  await page.click('#regionsView');
  await page.waitForFunction(() => document.querySelector('#worldText').textContent.startsWith('TRAJECTORY'), null, { timeout: 30000 });
  // D03: the places view: roles, districts, scenes and features.
  await page.click('#placesView');
  await page.waitForFunction(() => document.querySelector('#worldText').textContent.includes('districts:'));
  // D01: the depth metrics for this world.
  await page.click('[data-tab=depth]');
  await page.waitForFunction(() => document.querySelector('#panel').textContent.includes('landmarks_unique_share'), null, { timeout: 60000 });
  await browser.close();
  if (errors.length) throw new Error('page errors: ' + errors.join('; '));
  if (!site.startsWith('SITE 0')) throw new Error('site view missing: ' + site.slice(0, 80));
  console.log('bench smoke test passed');
})().catch(e => { console.error(e.message); process.exit(1); });
