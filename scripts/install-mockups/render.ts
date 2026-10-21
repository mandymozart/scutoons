// The install tutorial's pictures (#159): mockups.html → viewer/static/app/<name>.<lang>.png (1x) + .2x.png, per
// language (the system UI speaks the reader's language). Headless Chrome through playwright-core.
//
//   node --experimental-transform-types --no-warnings scripts/install-mockups/render.ts [chrome.exe]

import { mkdirSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { chromium } from 'playwright-core';

const here = dirname(fileURLToPath(import.meta.url));
const out = resolve(here, '../../viewer/static/app');
const chrome = process.argv[2] ?? 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe';

mkdirSync(out, { recursive: true });
const browser = await chromium.launch({ executablePath: chrome });
try {
	for (const lang of ['de', 'en']) {
		for (const [scale, suffix] of [[1, ''], [2, '.2x']] as const) {
			const page = await browser.newPage({ deviceScaleFactor: scale, viewport: { width: 1100, height: 900 } });
			await page.goto(`${pathToFileURL(join(here, 'mockups.html')).href}?lang=${lang}`);
			await page.waitForSelector('body[data-ready]');
			for (const frame of await page.$$('.frame')) {
				const name = await frame.getAttribute('data-name');
				await frame.screenshot({ path: join(out, `${name}.${lang}${suffix}.png`) });
			}
			await page.close();
		}
	}
} finally {
	await browser.close();
}
console.log(`written to ${out}`);
