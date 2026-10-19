// Screenshots of the drafts (headless Chromium, playwright-core from the repo): node agents/research/strip-editor/shot.mjs <abs dir> "phone.html#b-open:390x844:out.png" …
import { chromium } from 'playwright-core';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
const [dir, ...jobs] = process.argv.slice(2);
const browser = await chromium.launch();
for (const job of jobs) {
	const [file, size, out] = job.split(':');
	const [w, h] = size.split('x').map(Number);
	const page = await browser.newPage({ viewport: { width: w, height: h }, deviceScaleFactor: 1 });
	const [f, hash] = file.split('#');
	await page.goto(pathToFileURL(path.join(dir, f)).href + (hash ? '#' + hash : ''));
	await page.waitForLoadState('networkidle');
	await page.evaluate(() => document.fonts.ready);
	await page.screenshot({ path: path.join(dir, out) });
	const over = await page.evaluate(() => document.documentElement.scrollWidth - innerWidth);
	console.log(out, over > 0 ? 'OVERFLOW ' + over : 'ok');
	await page.close();
}
await browser.close();
