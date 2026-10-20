#!/usr/bin/env node
// The strip editor (agents/research/TEXT-ACROSS-SLICES.md §4, step 2) in the creator, through Playwright (chromium;
// playwright-core – see endless-engines-check.mjs). Signed in as the test user (npm run dev:test-user); nothing is saved
// (edits stay in the page and are dropped).
//   desktop 1440 × 900, a vertical group (default catpope ch2 group 6, slice 20):
//     1. the group is ONE strip: a slot per slice, the open slice in the view's centre, only a window of slices mounted
//     2. a click on a box of the next slice: that slice opens (URL, ‹ n / m ›), the box selected – no jump
//     3. Esc, then scrolling: the slice in the view's centre opens
//     4. ‹ › of the action bar: the strip scrolls the slice to the centre
//     5. a box pushed over the seam (Shift + ↓): its crossing part lies over the next slice and takes the pointer there
//     6. the seams toggle: a hairline at every seam, remembered
//     7. a zoom (1:1, fit) moves the scroll but never opens another slice
//     8. a box dragged over the seam and let go there moves to that slice (id kept, selected); one undo puts it back
//   a horizontal group (default group 2): the slices side by side at the area's height
//   phone 390 × 844 (touch): the strip at full width, a tap on a box selects it
//   node scripts/dev/creator-strip-check.mjs [--toon=catpope] [--chapter=2] [--group=6] [--slice=20] [--hgroup=2]
//        (creator :10000, API :9000 – local only)
import { chromium } from 'playwright-core';
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '../..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const CREATOR = String(opt.creator || local('creator')).replace(/\/$/, '');
if (!/^https?:\/\/(localhost|127\.0\.0\.1|\[::1\])(:\d+)?$/.test(CREATOR)) { console.error('creator-strip-check: local servers only'); process.exit(2); }
const TOON = String(opt.toon || 'catpope');
const CHAPTER = Number(opt.chapter || 2);
const GROUP = Number(opt.group || 6);
const SLICE = Number(opt.slice || 20);
const HGROUP = Number(opt.hgroup || 2);
const sessionFile = join(ROOT, 'logs/dev/test-user.json');
const token = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')).token : null;
if (!token) { console.error('no test-user session (run `npm run dev:test-user`)'); process.exit(2); }

const results = [];
const check = (name, ok, detail = '') => { results.push(!!ok); console.log(`${ok ? 'ok  ' : 'FAIL'} ${name}${!ok && detail ? `  — ${detail}` : ''}`); };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function open(browser, { w, h, phone = false, zoom = null }, group, slice) {
	const context = await browser.newContext({ viewport: { width: w, height: h }, ...(phone ? { isMobile: true, hasTouch: true, deviceScaleFactor: 2 } : {}) });
	await context.addInitScript(([t, z]) => {
		try {
			localStorage.setItem('scutoons_session', t);
			localStorage.removeItem('scutoons_creator_seams');
			localStorage.setItem('scutoons_creator_zoom', JSON.stringify(z));
		} catch {}
	}, [token, zoom]);
	const page = await context.newPage();
	const errors = [];
	page.on('pageerror', (e) => errors.push(e.message));
	await page.goto(`${CREATOR}/${TOON}/chapter/${CHAPTER}?group=${group}&slice=${slice}`, { waitUntil: 'domcontentloaded' });
	await page.waitForSelector('[data-testid="strip"] [data-testid="slice-canvas"]', { timeout: 40000 });
	await sleep(1500);
	return { context, page, errors };
}
const sliceOf = (page) => page.evaluate(() => Number(new URLSearchParams(location.search).get('slice')));
/** the slot in the area's centre and the open one */
const centred = (page) =>
	page.evaluate(() => {
		const area = document.querySelector('[data-testid="canvas-area"]').getBoundingClientRect();
		const axis = document.querySelector('[data-testid="strip"]').dataset.axis;
		const mid = axis === 'vertical' ? (area.top + area.bottom) / 2 : (area.left + area.right) / 2;
		const slot = [...document.querySelectorAll('[data-strip]')].find((s) => { const r = s.getBoundingClientRect(); return axis === 'vertical' ? r.top <= mid && r.bottom >= mid : r.left <= mid && r.right >= mid; });
		return { centre: slot ? Number(slot.dataset.strip) : -1, open: Number(document.querySelector('.slot.active')?.dataset.strip ?? -2) };
	});

const browser = await chromium.launch();
try {
	// ── desktop, a vertical group ──
	{
		// at 50 %: the neighbours' boxes are on screen without scrolling (a scroll would open the slice in the centre)
		const { context, page, errors } = await open(browser, { w: 1440, h: 900, zoom: 0.5 }, GROUP, SLICE);
		const s1 = await page.evaluate(() => {
			const strip = document.querySelector('[data-testid="strip"]');
			const slots = [...strip.querySelectorAll('[data-strip]')];
			return { axis: strip.dataset.axis, n: slots.length, mounted: slots.filter((s) => s.dataset.mounted === '1').length, open: document.querySelectorAll('[data-testid="slice-canvas"]').length };
		});
		const c1 = await centred(page);
		check(`1440: the group is one strip – ${s1.n} slots, ${s1.mounted} mounted, slice ${SLICE} open in the centre`, s1.axis === 'vertical' && s1.n > 4 && s1.mounted < s1.n && s1.mounted >= 2 && s1.open === 1 && c1.centre === SLICE - 1 && c1.open === SLICE - 1, JSON.stringify({ ...s1, ...c1 }));

		// 2 – a box of a following slice, on screen
		const target = await page.evaluate(() => {
			const area = document.querySelector('[data-testid="canvas-area"]').getBoundingClientRect();
			for (const b of document.querySelectorAll('[data-testid^="strip-box-dialog-"]')) {
				const r = b.getBoundingClientRect();
				const slot = Number(b.closest('[data-strip]').dataset.strip);
				if (slot > Number(document.querySelector('.slot.active').dataset.strip) && r.top > area.top && r.top + 8 < area.bottom) return { slot, id: b.dataset.testid.replace('strip-', '') };
			}
			return null;
		});
		const next = target?.id;
		if (!target) check('1440: a box on a following slice to click', false, 'none on screen');
		else {
			const before = await page.evaluate(() => document.querySelector('[data-testid="canvas-area"]').scrollTop);
			const box = await page.locator(`[data-strip="${target.slot}"] [data-testid="strip-${next}"]`).boundingBox();
			await page.mouse.click(box.x + box.width / 2, box.y + Math.min(box.height / 2, 6));
			await sleep(400);
			const after = await page.evaluate((t) => ({ top: document.querySelector('[data-testid="canvas-area"]').scrollTop, sel: !!document.querySelector(`[data-testid="slice-canvas"] [data-testid="${t}"].selected`) }), next);
			check(`1440: a click on ${next} of slice ${target.slot + 1} opens that slice and selects the box, no jump`, (await sliceOf(page)) === target.slot + 1 && after.sel && Math.abs(after.top - before) < 2, JSON.stringify({ slice: await sliceOf(page), ...after, before }));

			// 5 – pushed over the seam: Shift + ↓ until the box's bottom passes its slice's bottom
			let crossing = null;
			for (let n = 0; n < 6 && !crossing; n++) {
				await page.keyboard.press('Shift+ArrowDown');
				await sleep(150);
				crossing = await page.evaluate((t) => {
					const b = document.querySelector(`[data-testid="slice-canvas"] [data-testid="${t}"]`);
					const own = b.closest('[data-strip]').getBoundingClientRect();
					const r = b.getBoundingClientRect();
					if (r.bottom <= own.bottom + 4) return null;
					b.scrollIntoView({ block: 'center' });
					const r2 = b.getBoundingClientRect(), own2 = b.closest('[data-strip]').getBoundingClientRect();
					const y = (Math.max(r2.top, own2.bottom) + r2.bottom) / 2; // the middle of its part past the seam
					const e = document.elementFromPoint(r2.left + r2.width / 2, y);
					return { onTop: !!e && b.contains(e), what: e?.className?.toString?.().slice(0, 40), next: !!e?.closest('[data-strip]') && e.closest('[data-strip]') !== b.closest('[data-strip]'), box: [Math.round(r2.top), Math.round(r2.bottom), Math.round(r2.left), Math.round(r2.right)], own: Math.round(own2.bottom), y: Math.round(y), z: getComputedStyle(b).zIndex, pe: getComputedStyle(b).pointerEvents, cls: b.className };
				}, next);
			}
			check('1440: a box pushed over the seam lies over the next slice and takes the pointer there', crossing?.onTop === true, JSON.stringify(crossing));
			await page.keyboard.press('Control+z');
		}

		// 3 – Esc, then the user's scroll (the wheel) opens the slice in the centre
		await page.keyboard.press('Escape');
		const mid = await page.evaluate(() => { const r = document.querySelector('[data-testid="canvas-area"]').getBoundingClientRect(); return [r.x + r.width / 2, r.y + r.height / 2]; });
		await page.mouse.move(...mid);
		for (let n = 0; n < 10; n++) await page.mouse.wheel(0, 200), await sleep(30);
		await sleep(700);
		const c3 = await centred(page);
		check(`1440: scrolled – the slice in the centre (${c3.centre + 1}) opens`, c3.centre === c3.open && (await sliceOf(page)) === c3.open + 1 && c3.open > SLICE, JSON.stringify(c3));

		// 4 – ‹ › scroll the strip
		const at = await sliceOf(page);
		const top0 = await page.evaluate(() => document.querySelector('[data-testid="canvas-area"]').scrollTop);
		await page.click('[data-testid="prev-slice"]');
		await sleep(500);
		const c4 = await centred(page);
		const top1 = await page.evaluate(() => document.querySelector('[data-testid="canvas-area"]').scrollTop);
		check(`1440: ‹ opens slice ${at - 1} and the strip scrolls it to the centre`, (await sliceOf(page)) === at - 1 && c4.centre === at - 2 && top1 < top0, JSON.stringify({ at, c4, top0, top1 }));

		// 8 – dragged over the seam and let go there: the box moves to that slice (its id, its place on screen), one undo
		{
			const before = await page.evaluate(() => {
				const open = Number(document.querySelector('.slot.active').dataset.strip);
				const b = document.querySelector('[data-testid="slice-canvas"] [data-testid^="box-dialog-"]');
				if (!b) return null;
				b.scrollIntoView({ block: 'center' });
				return { open, id: b.querySelector('.tag')?.textContent, testid: b.dataset.testid, count: document.querySelectorAll('[data-testid="slice-canvas"] [data-testid^="box-dialog-"]').length };
			});
			if (!before) check('1440: a text box on the open slice to drag', false);
			else {
				await sleep(500);
				const at = await page.evaluate((t) => {
					const b = document.querySelector(`[data-testid="slice-canvas"] [data-testid="${t}"]`).getBoundingClientRect();
					const slot = document.querySelector('.slot.active').getBoundingClientRect();
					return { x: b.x + b.width / 2, y: b.y + Math.min(6, b.height / 2), cy: b.y + b.height / 2, bottom: slot.bottom };
				}, before.testid);
				const open0 = await sliceOf(page);
				const dy = at.bottom - at.cy + 30;
				await page.mouse.move(at.x, at.y);
				await page.mouse.down();
				for (let n = 1; n <= 8; n++) await page.mouse.move(at.x, at.y + (dy * n) / 8);
				await page.mouse.up();
				await sleep(500);
				const after = await page.evaluate(() => {
					const sel = document.querySelector('[data-testid="slice-canvas"] .box.selected');
					return { id: sel?.querySelector('.tag')?.textContent, slot: Number(document.querySelector('.slot.active').dataset.strip) };
				});
				const left = await page.evaluate((o) => document.querySelectorAll(`[data-strip="${o}"] [data-testid^="strip-box-dialog-"]`).length, before.open);
				check(`1440: ${before.id} dragged over the seam is re-anchored on slice ${open0 + 1}, selected, its id kept`, (await sliceOf(page)) === open0 + 1 && after.id === before.id && after.slot === before.open + 1 && left === before.count - 1, JSON.stringify({ before, after, left, slice: await sliceOf(page) }));
				await page.keyboard.press('Control+z');
				await sleep(400);
				const back = await page.evaluate((o) => document.querySelectorAll(`[data-strip="${o}"] [data-testid*="box-dialog-"]`).length, before.open);
				check('1440: one undo puts it back on its slice', back === before.count, String(back));
				await page.keyboard.press('Escape');
			}
		}

		// 7 – a zoom moves the scroll, never the open slice
		const z0 = await sliceOf(page);
		await page.click('[data-testid="canvas-zoom"]');
		await page.click('[data-testid="canvas-zoom-1x"]');
		await sleep(400);
		await page.click('[data-testid="canvas-zoom-fit"]');
		await page.keyboard.press('Escape');
		await sleep(400);
		check(`1440: zooming (1:1, fit) keeps slice ${z0} open`, (await sliceOf(page)) === z0, String(await sliceOf(page)));

		// 6 – the seams toggle
		const seams = async () => page.evaluate(() => [...document.querySelectorAll('[data-testid="strip"] .seam')].filter((s) => getComputedStyle(s).display !== 'none').length);
		const off = await seams();
		await page.click('[data-testid="seams-toggle"]');
		await sleep(200);
		const on = await seams();
		const kept = await page.evaluate(() => localStorage.getItem('scutoons_creator_seams'));
		check(`1440: the seams toggle draws a hairline at every seam (${on}), remembered`, off === 0 && on === s1.n - 1 && kept === 'true', JSON.stringify({ off, on, kept }));
		await page.click('[data-testid="seams-toggle"]');

		check('1440: no page errors', !errors.length, errors.join(' | ').slice(0, 300));
		await context.close();
	}

	// ── desktop, a horizontal group ──
	{
		const { context, page, errors } = await open(browser, { w: 1440, h: 900 }, HGROUP, 2);
		const h = await page.evaluate(() => {
			const strip = document.querySelector('[data-testid="strip"]');
			const area = document.querySelector('[data-testid="canvas-area"]').getBoundingClientRect();
			const r = [...strip.querySelectorAll('[data-strip]')].slice(0, 3).map((s) => s.getBoundingClientRect());
			return { axis: strip.dataset.axis, side: r.length > 1 && Math.abs(r[0].top - r[1].top) < 1 && Math.abs(r[0].right - r[1].left) < 1, height: Math.round(r[0].height), area: Math.round(area.height) };
		});
		const c = await centred(page);
		check(`1440 horizontal: the slices side by side at the area's height, slice 2 open in the centre`, h.axis === 'horizontal' && h.side && Math.abs(h.height - h.area) <= 2 && c.open === 1 && c.centre === 1, JSON.stringify({ ...h, ...c }));
		check('1440 horizontal: no page errors', !errors.length, errors.join(' | ').slice(0, 300));
		await context.close();
	}

	// ── phone ──
	{
		const { context, page, errors } = await open(browser, { w: 390, h: 844, phone: true }, GROUP, SLICE);
		const p = await page.evaluate(() => {
			const strip = document.querySelector('[data-testid="strip"]').getBoundingClientRect();
			const area = document.querySelector('[data-testid="canvas-area"]').getBoundingClientRect();
			return { strip: Math.round(strip.width), area: Math.round(area.width) };
		});
		check(`390: the strip at the area's full width (${p.strip} / ${p.area})`, Math.abs(p.strip - p.area) <= 1, JSON.stringify(p));
		const t = await page.evaluate(() => { const b = document.querySelector('[data-testid="slice-canvas"] [data-testid^="box-dialog-"]'); if (!b) return null; b.scrollIntoView({ block: 'center' }); return b.dataset.testid; });
		if (t) {
			await sleep(300);
			const box = await page.locator(`[data-testid="slice-canvas"] [data-testid="${t}"]`).boundingBox();
			await page.touchscreen.tap(box.x + box.width / 2, box.y + Math.min(box.height / 2, 6));
			await sleep(400);
			check(`390: a tap on ${t} selects it`, await page.evaluate((x) => !!document.querySelector(`[data-testid="${x}"].selected`), t));
		}
		check('390: no page errors', !errors.length, errors.join(' | ').slice(0, 300));
		await context.close();
	}
} finally {
	await browser.close();
}
const failed = results.filter((r) => !r).length;
console.log(`\n${results.length - failed} / ${results.length} ok`);
process.exit(failed ? 1 : 0);
