// Real Chromium rendering against the production Vite build. The browser uses the
// app's built-in mock API; this is NOT a native Tauri/database benchmark.
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { setTimeout as delay } from 'node:timers/promises';
import { chromium } from 'playwright';

const root = process.cwd();
const output = resolve(root, 'qa', 'evidence');
const origin = 'http://127.0.0.1:4173';
const widths = [1440, 1100, 760];
const locales = ['en', 'ar'];
const themes = ['dark', 'system'];
const views = [
  { name: 'dashboard', key: 'd', index: 0 },
  { name: 'research', key: 'r', index: 1 },
  { name: 'jobs', key: 'j', index: 2 },
  { name: 'transcript', key: 't', index: 3 },
  { name: 'ai-studio', key: 'a', index: 4 },
  { name: 'library', key: 'l', index: 5 },
  { name: 'settings', key: ',', index: null }
];
const failures = [];
const results = [];
let screenshotCount = 0;
const allowlist = JSON.parse(await readFile(resolve(root, 'qa', 'native-control-allowlist.json'), 'utf8'));

function recordFailure(scenario, error) {
  const message = error instanceof Error ? error.stack || error.message : String(error);
  failures.push({ scenario, error: message });
  console.error('FAIL [' + scenario + '] ' + message);
}
async function waitForServer() {
  for (let i = 0; i < 80; i += 1) {
    try {
      const response = await fetch(origin, { signal: AbortSignal.timeout(1500) });
      if (response.ok) return;
    } catch { /* Preview may still be booting. */ }
    await delay(250);
  }
  throw new Error('Vite preview did not start at ' + origin);
}
async function screenshot(page, pathname) {
  await page.screenshot({
    path: resolve(output, pathname + '.png'),
    animations: 'disabled',
    caret: 'hide',
    fullPage: false
  });
  screenshotCount += 1;
}
async function routeTo(page, view) {
  // The actual production keyboard routing is tested, not a test-only view setter.
  await page.locator('#workspace-main').focus();
  const start = performance.now();
  await page.keyboard.press(view.key);
  if (view.index === null) {
    await page.locator('.sidebar-utility .settings-link[aria-current="page"]').waitFor({ timeout: 5000 });
  } else {
    await page.waitForFunction((index) => {
      const buttons = document.querySelectorAll('.primary-nav button');
      return buttons[index]?.getAttribute('aria-current') === 'page';
    }, view.index, { timeout: 5000 });
  }
  return performance.now() - start;
}
async function checkOverflow(page, label) {
  const data = await page.evaluate(() => ({
    scrollWidth: document.documentElement.scrollWidth,
    innerWidth: window.innerWidth,
    mainVisible: Boolean(document.querySelector('#workspace-main'))
  }));
  if (!data.mainVisible) recordFailure(label, 'Workspace shell absent');
  if (data.scrollWidth > data.innerWidth + 2) {
    recordFailure(label, 'Horizontal page overflow: ' + data.scrollWidth + ' > ' + data.innerWidth);
  }
}
async function checkNativeControls(page, label) {
  const issues = await page.locator('select, input[type="range"]').evaluateAll((elements, selectors) =>
    elements.flatMap((node) => {
      if (!(node instanceof HTMLElement) || !node.getClientRects().length) return [];
      if (selectors.some((selector) => node.matches(selector))) return [];
      const style = getComputedStyle(node);
      const appearance = style.appearance || style.getPropertyValue('-webkit-appearance');
      if (appearance === 'none') return [];
      const name = node.getAttribute('aria-label') || node.id || node.tagName.toLowerCase();
      return [{ control: name, appearance, html: node.outerHTML.slice(0, 170) }];
    }), allowlist.nativeSelectors);
  for (const issue of issues) recordFailure(label + '/native-control', JSON.stringify(issue));
}
async function checkSwitchesAndPopovers(page, label) {
  const switches = page.locator('input[role="switch"], label.switch input[type="checkbox"]');
  const issues = await switches.evaluateAll((elements) => elements.flatMap((element) => {
    if (!(element instanceof HTMLElement) || !element.getClientRects().length) return [];
    const appearance = getComputedStyle(element).appearance;
    const hit = element.closest('label')?.getBoundingClientRect() ?? element.getBoundingClientRect();
    const problems = [];
    if (appearance !== 'none') problems.push('native switch appearance=' + appearance);
    if (hit.height < 32) problems.push('switch hit target height=' + hit.height);
    return problems.map((problem) => ({
      control: element.getAttribute('aria-label') || element.outerHTML.slice(0, 120), problem
    }));
  }));
  for (const issue of issues) recordFailure(label + '/switch', JSON.stringify(issue));
  const first = switches.first();
  if (await first.count() && await first.isVisible() && await first.isEnabled()) {
    const before = await first.isChecked();
    await first.focus();
    await page.keyboard.press('Space');
    if (await first.isChecked() === before) {
      recordFailure(label + '/switch-keyboard', 'Space did not toggle focused switch');
    }
    await page.keyboard.press('Space');
  }
  const popovers = page.locator('[aria-haspopup="dialog"]');
  for (let index = 0; index < await popovers.count(); index += 1) {
    const trigger = popovers.nth(index);
    if (!await trigger.isVisible() || !await trigger.isEnabled()) continue;
    await trigger.focus();
    await page.keyboard.press('Enter');
    if (await trigger.getAttribute('aria-expanded') !== 'true') {
      recordFailure(label + '/popover', 'Keyboard Enter did not open the popover');
      continue;
    }
    await page.keyboard.press('Escape');
    if (await trigger.getAttribute('aria-expanded') !== 'false') {
      recordFailure(label + '/popover', 'Escape did not close the popover');
    }
    break;
  }
}
async function checkKeyboard(page, label) {
  await page.locator('#workspace-main').focus();
  await page.keyboard.press('Control+k');
  const dialog = page.getByRole('dialog');
  await dialog.waitFor({ timeout: 5000 });
  const focusedInside = await page.evaluate(() => {
    const modal = document.querySelector('[role="dialog"][aria-modal="true"]');
    return Boolean(modal && document.activeElement && modal.contains(document.activeElement));
  });
  if (!focusedInside) recordFailure(label + '/keyboard', 'Command palette did not focus inside its modal');
  await page.keyboard.press('Tab');
  const tabInside = await page.evaluate(() => {
    const modal = document.querySelector('[role="dialog"][aria-modal="true"]');
    return Boolean(modal && document.activeElement && modal.contains(document.activeElement));
  });
  if (!tabInside) recordFailure(label + '/keyboard', 'Tab escaped the command palette');
  await page.keyboard.press('Escape');
  await dialog.waitFor({ state: 'hidden', timeout: 5000 });

  // Actual focus visibility, not a static CSS selector search.
  await page.keyboard.press('Tab');
  const focus = await page.evaluate(() => {
    const el = document.activeElement;
    if (!(el instanceof HTMLElement)) return { present: false, visible: false };
    const style = getComputedStyle(el);
    return {
      present: true,
      visible: el.matches(':focus-visible') &&
        ((style.outlineStyle !== 'none' && parseFloat(style.outlineWidth) > 0) ||
         style.boxShadow !== 'none'),
      tag: el.tagName
    };
  });
  if (!focus.present || !focus.visible) recordFailure(label + '/focus', JSON.stringify(focus));
}
async function runWorkspace(browser, width, locale, theme) {
  const label = [width, locale, theme].join('-');
  const context = await browser.newContext({
    viewport: { width, height: width === 760 ? 820 : 900 },
    deviceScaleFactor: 1,
    colorScheme: theme === 'system' ? 'light' : 'dark',
    reducedMotion: 'reduce',
    locale: locale === 'ar' ? 'ar-AE' : 'en-US'
  });
  await context.addInitScript(({ language, appearance }) => {
    localStorage.setItem('scriptotar.uiLanguage', language);
    localStorage.setItem('scriptotar.appearance', appearance);
  }, { language: locale, appearance: theme });
  const page = await context.newPage();
  const pageErrors = [];
  page.on('pageerror', (error) => pageErrors.push(String(error)));
  try {
    const navigationStart = performance.now();
    await page.goto(origin, { waitUntil: 'domcontentloaded' });
    await page.locator('#workspace-main').waitFor({ timeout: 15000 });
    const shellMs = performance.now() - navigationStart;
    const actualDir = await page.locator('html').getAttribute('dir');
    const expectedDir = locale === 'ar' ? 'rtl' : 'ltr';
    if (actualDir !== expectedDir) recordFailure(label, 'Expected ' + expectedDir + ', got ' + actualDir);
    const activeTheme = await page.locator('html').getAttribute('data-theme');
    if (activeTheme !== theme) recordFailure(label, 'Theme mismatch: ' + activeTheme);
    const samples = [];
    for (const view of views) {
      const navigationMs = await routeTo(page, view);
      await page.locator('#workspace-main h1').first().waitFor({ timeout: 5000 });
      await checkOverflow(page, label + '/' + view.name);
      await checkNativeControls(page, label + '/' + view.name);
      await checkSwitchesAndPopovers(page, label + '/' + view.name);
      await screenshot(page, label + '-' + view.name);
      samples.push({ view: view.name, navigationMs });
      if (view.name === 'library') {
        const search = page.locator('.library-toolbar input').first();
        await search.fill('qa-absent-item-000000000');
        await page.locator('.library-list').waitFor({ state: 'hidden', timeout: 5000 });
        await screenshot(page, label + '-library-empty');
        await search.fill('');
      }
      if (view.name === 'settings') {
        await page.locator('[data-testid="mcp-permissions"]').waitFor({ timeout: 5000 });
        await page.locator('[data-testid="mcp-permissions"]').scrollIntoViewIfNeeded();
        await screenshot(page, label + '-mcp-setup');
      }
    }
    await checkKeyboard(page, label);
    if (pageErrors.length) recordFailure(label + '/pageerrors', pageErrors.join('; '));
    results.push({ label, width, locale, theme, mode: 'browser-mock-client',
      shellMs, navigation: samples, jsHeapBytes: await page.evaluate(() =>
        performance.memory?.usedJSHeapSize ?? null) });
  } catch (error) {
    recordFailure(label, error);
  } finally {
    await context.close();
  }
}
async function runBootState(browser, locale, theme, state) {
  const label = '1100-' + locale + '-' + theme + '-' + state;
  const context = await browser.newContext({
    viewport: { width: 1100, height: 800 },
    colorScheme: theme === 'system' ? 'light' : 'dark',
    reducedMotion: 'reduce'
  });
  await context.addInitScript(({ language, appearance, mode }) => {
    localStorage.setItem('scriptotar.uiLanguage', language);
    localStorage.setItem('scriptotar.appearance', appearance);
    window.__TAURI__ = {
      core: {
        invoke: (command) => {
          if (command === 'bootstrap_app') {
            return mode === 'loading' ? new Promise(() => {}) :
              Promise.reject(new Error('QA injected bootstrap failure'));
          }
          if (command === 'get_watchlist_statuses') return Promise.resolve([]);
          if (command === 'get_migration_status') {
            return Promise.resolve({ state: 'no_legacy_db', message: 'QA fixture', candidates: [] });
          }
          return Promise.reject(new Error('QA fixture only: ' + command));
        }
      }
    };
  }, { language: locale, appearance: theme, mode: state });
  const page = await context.newPage();
  try {
    await page.goto(origin, { waitUntil: 'domcontentloaded' });
    const indicator = state === 'loading'
      ? page.locator('[aria-busy="true"]').first()
      : page.getByText('QA injected bootstrap failure');
    await indicator.waitFor({ timeout: 15000 });
    await screenshot(page, label);
  } catch (error) {
    recordFailure(label, error);
  } finally {
    await context.close();
  }
}
await mkdir(output, { recursive: true });
const server = spawn('npm', ['run', 'preview', '--', '--host', '127.0.0.1',
  '--port', '4173', '--strictPort'], { cwd: root, stdio: 'ignore', detached: true });
let browser;
try {
  await waitForServer();
  browser = await chromium.launch({ headless: true });
  for (const width of widths) for (const locale of locales) for (const theme of themes) {
    await runWorkspace(browser, width, locale, theme);
  }
  for (const locale of locales) for (const theme of themes) {
    for (const state of ['loading', 'error']) await runBootState(browser, locale, theme, state);
  }
} catch (error) {
  recordFailure('infrastructure', error);
} finally {
  if (browser) await browser.close();
  try { process.kill(-server.pid, 'SIGTERM'); } catch { server.kill(); }
}
const report = {
  generatedAt: new Date().toISOString(),
  sha: process.env.GITHUB_SHA || 'local-unpinned',
  baselineSha: 'f741244634b5143602a21a554094a498a0005b11',
  scope: 'Chromium browser / production Vite bundle / built-in mock client; NOT installed Tauri',
  screenshotCount, failures, results
};
await writeFile(resolve(output, 'report.json'), JSON.stringify(report, null, 2) + '\n');
console.log('Rendered screenshots: ' + screenshotCount +
  '; checked combinations: ' + results.length + '; failures: ' + failures.length);
if (failures.length || screenshotCount !== 116) process.exitCode = 1;
