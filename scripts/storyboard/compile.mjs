// Storyboard → chapter draft (STORYBOARD.md §2 "The compile", phase 2): the chapter's one language-neutral document
// as save_draft takes it, with the schematic shots as slice images. One shot = one slice (390 × its height); a loop360
// = a horizontal group (loop + freeMode) of screen-high slides; dialog / items / events / audio keep the chapter's types,
// rects stay in % (a shot is its slice). The chapter's other groups (side quests like mama-*) are kept, the
// placeholder group "chapter-<n>" is replaced. Images are rasterized from the schematic SVGs (headless Edge / Chrome).
//
//   node scripts/storyboard/compile.mjs storyboards/catpope/s1c3 [--chapter catpope:3] [--no-png] [--out storyboards/_out]
//
// Output: _out/<name>/chapter.draft.json (save with the MCP: save_draft key chapter:catpope:3, base = the version read)
// and _out/<name>/png/<file> with the CDN paths to upload them to (upload_asset path "catpope/s1/c3/storyboard/…").
// Nothing is saved or uploaded from here.

import { execFileSync } from 'node:child_process';
import { copyFileSync, existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { ROOT, S, W, readEntity } from './lib.mjs';
import { shotSvg } from './schematic.mjs';
import { loadStoryboards } from './storyboard.mjs';

const args = process.argv.slice(2);
const flag = (f) => { const i = args.indexOf(f); if (i < 0) return false; args.splice(i, 1); return true; };
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const noPng = flag('--no-png');
const outRoot = resolve(ROOT, take('--out', 'storyboards/_out'));
const chapterRef = take('--chapter', null);
const src = args[0];
if (!src) { console.error('usage: compile.mjs <storyboard folder>'); process.exit(2); }

const list = loadStoryboards(src);
const name = basename(resolve(src)); // s1c3
const first = list[0]?.sb;
const toon = first?.toon ?? 'catpope';
const number = first?.chapter;
const m = /^s(\d+)c(\d+)$/.exec(name);
const folder = m ? `s${m[1]}/c${m[2]}/storyboard` : `${name}/storyboard`; // relative to the toon folder
const out = resolve(outRoot, name);
mkdirSync(resolve(out, 'png'), { recursive: true });

const assets = []; // { svg, file, w, h, path, frame?, crop? }
const frames = resolve(out, 'frames'); // tools/storyboard-render/render-shots.mjs output – the images win over the SVG schematics
function asset(svg, w, h, id, frameOf = id, crop = null) {
	const file = `${id}.png`;
	const frame = resolve(frames, `${frameOf}.png`);
	assets.push({ svg, file, w, h, path: `${folder}/${file}`, frame: existsSync(frame) ? frame : null, crop });
	return `${folder}/${file}`;
}

const groups = [];
for (const { sb } of list) {
	for (const g of sb.groups ?? []) {
		const loop = (g.shots ?? []).some((s) => s.format === 'loop360');
		const group = { label: g.label, renderer: g.renderer ?? { type: 'vertical' }, interaction: 'webtoon' };
		if (g.continues) group.continues = true;
		group.slices = [];
		for (const shot of g.shots ?? []) {
			const { svg, w, h } = shotSvg(shot, sb, { noText: true }); // the text is live (slice.dialog), not in the image
			if (shot.format === 'loop360') {
				// cut the panorama into screen-high slides of 390
				const n = Math.max(1, Math.round(w / W));
				for (let k = 0; k < n; k++) {
					const part = svg.replace(/<svg ([^>]*?)width="\d+" height="\d+" viewBox="[^"]*"/, `<svg $1width="${W}" height="${S}" viewBox="${k * W} 0 ${W} ${S}"`);
					const slice = { image: asset(part, W, S, `${shot.id}-${k + 1}`, shot.id, [k * W, 0, (k + 1) * W, S]), width: W, height: S };
					const inSlide = (pct) => pct * n - k * 100;
					const dialog = (shot.dialog ?? []).filter((d) => Math.floor(((d.rect?.x ?? 0) / 100) * n) === k)
						.map((d, j) => {
							const width = Math.min(96, (d.rect.width ?? 30) * n);
							const x = Math.max(2, Math.min(100 - width - 2, inSlide(d.rect.x))); // moved inside its slide
							return { ...dialogOf(d, `${shot.id}-${k + 1}-${j + 1}`), rect: { ...dialogOf(d, '').rect, x: +x.toFixed(2), width: +width.toFixed(2) } };
						});
					if (dialog.length) slice.dialog = dialog;
					group.slices.push(slice);
				}
				if (loop) group.renderer = { type: 'horizontal', params: { ...(g.renderer?.params ?? {}), loop: true, freeMode: true } };
				continue;
			}
			const slice = { image: asset(svg, w, h, shot.id), width: W, height: h };
			const dialog = (shot.dialog ?? []).map((d, j) => dialogOf(d, `${shot.id}-${j + 1}`));
			if (dialog.length) slice.dialog = dialog;
			if (shot.items?.length) slice.items = shot.items;
			if (shot.audio) slice.audio = shot.audio;
			if (shot.video) slice.video = shot.video;
			group.slices.push(slice);
			for (const e of shot.events ?? []) (group.events ??= []).push(e);
		}
		for (const e of g.events ?? []) (group.events ??= []).push(e);
		if (g.audio) group.audio = g.audio;
		groups.push(group);
	}
}

function dialogOf(d, id) {
	const o = { id: `${name}-${id}`, rect: { x: d.rect?.x ?? 5, y: d.rect?.y ?? 5, width: d.rect?.width ?? 60, ...(d.rect?.height ? { height: d.rect.height } : {}) }, text: d.text, variant: d.variant ?? 'narration' };
	if (d.font) o.font = d.font;
	if (d.variant === 'narration' && !d.font) o.font = 'mono';
	for (const k of ['size', 'align', 'valign', 'tail', 'uppercase', 'weight']) if (d[k] != null) o[k] = d[k];
	return o;
}

// the chapter: the stored one (local dev DB, read-only) with its placeholder group replaced, or a bare one
let chapter, base = null;
const key = `chapter:${chapterRef ?? `${toon}:${number}`}`;
try {
	chapter = readEntity(key, 'any');
} catch (e) {
	console.warn(`(${e.message.split('\n')[0]} – a bare chapter)`);
	chapter = { number, groups: [] };
}
const kept = (chapter.groups ?? []).filter((g) => !/^chapter-\d+$/.test(g.label ?? ''));
const draft = { ...chapter, groups: [...groups, ...kept] };
writeFileSync(resolve(out, 'chapter.draft.json'), JSON.stringify(draft, null, '\t'));

// rasterize (headless Edge / Chrome screenshot of each SVG)
let png = 0;
if (!noPng) {
	const browsers = [process.env.CHROME, 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', 'C:/Program Files/Google/Chrome/Application/chrome.exe', '/usr/bin/chromium', '/usr/bin/google-chrome'].filter(Boolean);
	const browser = browsers.find((b) => existsSync(b));
	if (!browser) console.warn('no Edge / Chrome found – PNGs skipped (--no-png)');
	else {
		mkdirSync(resolve(out, 'svg-slices'), { recursive: true });
		for (const a of assets) {
			if (a.frame) {
				// the rendered frame: copied, a panorama's slide cropped out of it (Pillow, as the cdn/ tools)
				const dst = resolve(out, 'png', a.file);
				if (!a.crop) copyFileSync(a.frame, dst);
				else execFileSync('python', ['-c', 'import sys; from PIL import Image; Image.open(sys.argv[1]).crop(tuple(map(int, sys.argv[3:7]))).save(sys.argv[2])', a.frame, dst, ...a.crop.map(String)]);
				png++;
				continue;
			}
			const svgFile = resolve(out, 'svg-slices', a.file.replace(/\.png$/, '.svg'));
			writeFileSync(svgFile, a.svg);
			execFileSync(browser, ['--headless=new', '--disable-gpu', '--hide-scrollbars', '--force-device-scale-factor=1', `--window-size=${a.w},${a.h}`, `--screenshot=${resolve(out, 'png', a.file)}`, pathToFileURL(svgFile).href], { stdio: 'ignore' });
			png++;
		}
	}
}
const slices = groups.reduce((a, g) => a + g.slices.length, 0);
writeFileSync(resolve(out, 'uploads.json'), JSON.stringify(assets.map((a) => ({ file: `png/${a.file}`, path: `${toon}/${a.path}` })), null, '\t'));
console.log(`${key}: ${groups.length} storyboard groups, ${slices} slices (+ ${kept.length} kept: ${kept.map((g) => g.label).join(', ') || '–'})`);
console.log(`→ ${resolve(out, 'chapter.draft.json')}\n→ ${png} PNGs in ${resolve(out, 'png')} (upload list: uploads.json)`);
console.log('Nothing saved: save_draft + upload_asset through the SCUTOONS MCP, by the owner or with the owner\'s go.');
