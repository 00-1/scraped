// Headless test of the browser player: it plays, it can be driven by
// keyboard alone, and it passes basic accessibility checks.
//   node tools/smoke/play.cjs   (after tools/build.sh)
const path = require('path');
let chromium;
try { ({ chromium } = require('playwright')); } catch { ({ chromium } = require(process.env.PLAYWRIGHT_PATH || '/opt/node22/lib/node_modules/playwright')); }
(async () => {
  const browser = await chromium.launch();
  const page = await browser.newPage({ acceptDownloads: true });
  const errors = [];
  page.on('pageerror', e => errors.push(e.message));
  await page.goto('file://' + path.resolve(__dirname, '../dist/play.html'));
  await page.waitForSelector('#log article');
  // Keyboard only: focus starts in the command box.
  if (await page.evaluate(() => document.activeElement.id) !== 'input') throw new Error('focus is not on the command box');
  await page.keyboard.type('look');
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => document.querySelectorAll('#log article').length >= 2);
  await page.keyboard.press('ArrowUp');
  if (await page.inputValue('#input') !== 'look') throw new Error('history recall failed');
  await page.keyboard.press('Enter');
  await page.waitForFunction(() => document.querySelectorAll('#log article').length >= 3);
  // Alt+M opens the menu; Tab moves through it; Escape closes it.
  await page.keyboard.press('Alt+m');
  await page.waitForSelector('dialog[open]');
  for (let i = 0; i < 6; i++) await page.keyboard.press('Tab');
  const inside = await page.evaluate(() => !!document.activeElement.closest('dialog'));
  if (!inside) throw new Error('focus escaped the menu');
  await page.keyboard.press('Escape');
  await page.waitForFunction(() => !document.querySelector('dialog').open);
  // Accessibility basics.
  const audit = await page.evaluate(() => {
    const problems = [];
    if (!document.documentElement.lang) problems.push('no lang');
    if (!document.querySelector('[role=log][aria-live]')) problems.push('no live log');
    for (const b of document.querySelectorAll('button')) if (!b.textContent.trim() && !b.getAttribute('aria-label')) problems.push('unnamed button ' + b.id);
    for (const i of document.querySelectorAll('input:not([type=hidden]):not([hidden]), select')) {
      const named = i.labels && i.labels.length || i.getAttribute('aria-label') || i.getAttribute('aria-labelledby');
      if (!named) problems.push('unlabelled ' + i.id);
    }
    if (!document.title) problems.push('no title');
    return problems;
  });
  if (audit.length) throw new Error('accessibility: ' + audit.join(', '));
  // Saves, text size, contrast, a new game from a code, downloads.
  await page.click('#menuBtn');
  await page.click('[data-save="0"]');
  await page.click('#larger');
  const size = await page.evaluate(() => getComputedStyle(document.documentElement).getPropertyValue('--size').trim());
  if (size !== '20px') throw new Error('text size: ' + size);
  await page.check('#contrast');
  if (await page.evaluate(() => document.documentElement.dataset.contrast) !== 'high') throw new Error('contrast');
  const [dl] = await Promise.all([page.waitForEvent('download'), page.click('#transcriptBtn')]);
  if (!(dl.suggestedFilename()).endsWith('.txt')) throw new Error('transcript download');
  const code = (await page.textContent('#worldCode')).split(' ').pop();
  await page.fill('#newCode', code);
  await page.click('#startBtn');
  await page.waitForFunction(() => document.querySelectorAll('#log article').length === 1);
  if ((await page.textContent('#worldCode')).split(' ').pop() !== code) throw new Error('the code gave a different world');
  await page.click('#menuBtn');
  await page.click('[data-load="0"]');
  await page.waitForFunction(() => !document.querySelector('dialog').open);
  await browser.close();
  if (errors.length) throw new Error('page errors: ' + errors.join('; '));
  console.log('browser player smoke test passed');
})().catch(e => { console.error(e.message); process.exit(1); });
