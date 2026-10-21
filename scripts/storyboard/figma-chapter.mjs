// The whole chapter as ONE long draft frame in Figma (owner 2026-10-10: "create a new draft frame very long like the one
// there is … just with our updated storyboard", "if there is already luma renders we use them", "the height of the
// frame depends on the amount of images our story board generated and of course on their aspect ratio").
//
// Reads the chapter draft (compile.mjs → _out/<ch>/chapter.draft.json: the order, every shot's height from its format +
// aspect, the texts) and sends it through the SCUTOONS Figma bridge (tools/figma-bridge) as one frame, 390 wide,
// as tall as the shots stacked:
//   storyboard (locked)  per shot its picture – a Luma result if there is one (the chosen image in storyboards/dataset,
//                        else the newest in _out/<ch>/generated/), otherwise the render without texts (_out/<ch>/clean/);
//                        a panorama as its whole strip, one screen high (the frame shows its first screen); an endless
//                        band's seamless panorama cropped to the band's 2340 so it still wraps (band-wrap.ts, #109)
//   guides (hidden)      every screen (844) from the top; per scene group a line at each cut (SLICE_H, 624, from the
//                        GROUP's top – where figma-production.mjs cuts, #107) and "end · +r px" at its end; a band's ends
//   text  the texts as live text · layout  empty, on top
// and the rail beside it (per group its cut "scene-25 · 11 slices + 588 px", per shot its source). Re-runs replace shots
// by id and never touch `layout`. The guides and the rail are drawn by the bridge module commands/slice-guides.js.
// --guides-only: ONLY the guides and the rail of the frame named --name (exactly), through slice-guides – nothing resized,
// re-stacked or replaced, `storyboard` / `text` / `layout` untouched: the way to fix the lines of a frame the owner works
// in (nothing is drawn when a shot isn't where the cut expects it). With --dry: the lines it would draw.
//
//   node scripts/storyboard/figma-chapter.mjs storyboards/catpope/s1c3 --page "S1C3 - BARGAINING" [--x 7400] [--y -109] [--name "…"] [--dry]
//   node scripts/storyboard/figma-chapter.mjs storyboards/catpope/s1c3 --page "S1C3 - BARGAINING" --name "s1c3 – storyboard draft 2026-10-09" --guides-only [--dry]

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { basename, extname, resolve } from 'node:path';
import { ROOT, S, W, chapterLayout, figma, sliceLines, textOf, mediaRoot } from './lib.mjs';

const args = process.argv.slice(2);
const take = (flag, dflt) => { const i = args.indexOf(flag); return i >= 0 ? args.splice(i, 2)[1] : dflt; };
const dry = args.includes('--dry') && args.splice(args.indexOf('--dry'), 1);
const guidesOnly = args.includes('--guides-only') && args.splice(args.indexOf('--guides-only'), 1);
const page = take('--page');
const fx = Number(take('--x', 7400)), fy = Number(take('--y', 0));
const src = args[0];
if (!src || !page) { console.error('usage: figma-chapter.mjs <storyboard folder> --page "<Figma page>" [--x n] [--y n] [--name text] [--dry]'); process.exit(2); }
const ch = basename(resolve(src));
const named = take('--name');
if (guidesOnly && !named) { console.error('--guides-only needs the frame\'s exact name: --name "<frame>"'); process.exit(2); }
const name = named ?? `${ch} – storyboard draft ${new Date().toLocaleDateString('sv')}`;
const OUT = resolve(ROOT, 'storyboards/_out', ch);
const draft = JSON.parse(readFileSync(resolve(OUT, 'chapter.draft.json'), 'utf8'));
const port = Number(process.env.SCUTOONS_FIGMA_PORT ?? 3056);

/** a Luma result for the shot, if there is one: the approved choice, else the newest dropped in */
function luma(id) {
	const ds = resolve(ROOT, 'storyboards/dataset/files', ch, id);
	const chosen = existsSync(ds) && readdirSync(ds).find((f) => f.startsWith('chosen.'));
	if (chosen) return resolve(ds, chosen);
	const gd = resolve(OUT, 'generated');
	const gen = existsSync(gd) ? readdirSync(gd).filter((f) => f.startsWith(`${id}--`)).map((f) => resolve(gd, f)) : [];
	return gen.sort((a, b) => statSync(b).mtimeMs - statSync(a).mtimeMs)[0] ?? null;
}
/** Figma takes PNG / JPEG / GIF up to 4096 px a side: anything else (webp) or bigger goes through PIL */
function bytesOf(file) {
	const ext = extname(file).toLowerCase();
	const py = `import sys,io,base64\nfrom PIL import Image\nim=Image.open(sys.argv[1]);im.load()\nif max(im.size)>4096: im.thumbnail((4096,4096))\nb=io.BytesIO();im.convert('RGB').save(b,'JPEG',quality=88);sys.stdout.write(base64.b64encode(b.getvalue()).decode())`;
	const big = () => { try { return execFileSync('python', ['-I', '-c', 'import sys\nfrom PIL import Image\nprint(max(Image.open(sys.argv[1]).size))', file]).toString().trim() > 4096; } catch { return true; } };
	if (['.png', '.jpg', '.jpeg'].includes(ext) && !big()) return readFileSync(file).toString('base64');
	return execFileSync('python', ['-I', '-c', py, file], { maxBuffer: 1 << 28 }).toString();
}
/**
 * An endless band's picture (#109, owner 2026-10-21: "crop it"): a seamless panorama wider than the band (3:1 = 2532 ×
 * 844) is cropped to the band's 2340 : 844 so it still wraps (band-wrap.ts) – then the FILL crops nothing and the cut's
 * slides are exactly it. → _out/<ch>/band/<id>.png, or the file itself when it needs no crop.
 */
function bandOf(id, file) {
	const out = resolve(OUT, 'band', `${id}.png`);
	mkdirSync(resolve(OUT, 'band'), { recursive: true });
	const said = execFileSync(process.execPath, ['--experimental-transform-types', '--no-warnings', resolve(ROOT, 'scripts/storyboard/band-wrap.ts'), file, out]).toString();
	return said.includes('nothing to crop') ? file : out;
}
const texts = (dialog, x0, top, h) => (dialog ?? []).map((d) => ({
	x: +(x0 + ((d.rect?.x ?? 5) / 100) * W).toFixed(1), y: +(top + ((d.rect?.y ?? 5) / 100) * h).toFixed(1), w: +(((d.rect?.width ?? 60) / 100) * W).toFixed(1),
	text: textOf(d.text), lettering: d.font === 'lettering', variant: d.variant ?? 'narration', size: d.font === 'lettering' ? +((((d.size ?? 15) / 100) * W)).toFixed(1) : undefined, title: !!d.title
}));

// the layout: every group's slices stacked; a horizontal group (panorama) = one screen high (lib.mjs chapterLayout – the
// layout figma-production.mjs cuts; its lines: sliceLines)
const layout = chapterLayout(draft);
const y = layout.reduce((a, G) => a + G.h, 0);
const shots = [], rail = [];
for (const G of layout) {
	if (G.pano) {
		const { id, y: top, slices } = G.shots[0];
		const file = luma(id) ?? resolve(OUT, 'clean', `${id}.png`);
		const w = slices.reduce((a, s) => a + s.width, 0);
		const endless = !!(G.g.renderer?.params?.loop || G.g.renderer?.params?.endless);
		shots.push({ id, file, from: file.includes('clean') ? 'render' : 'luma', endless, x: 0, y: top, w, h: S, texts: slices.flatMap((s, i) => texts(s.dialog, i * W, top, S)) });
		rail.push({ y: top + 16, label: `${id}  panorama ${slices.length} screens · ${shots.at(-1).from}`, shot: id });
		continue;
	}
	for (const { id, y: top, slice: s } of G.shots) {
		const own = /\/storyboard\//.test(s.image);
		const file = own ? luma(id) ?? resolve(OUT, 'clean', `${id}.png`) : resolve(mediaRoot(), 'catpope', s.image);
		const from = !own ? 'existing' : file.includes(`${'clean'}`) ? 'render' : 'luma';
		shots.push({ id, file, from, x: 0, y: top, w: s.width, h: s.height, texts: texts(s.dialog, 0, top, s.height) });
		rail.push({ y: top + 16, label: `${id}  ${from}`, shot: id });
	}
}
const lines = sliceLines(layout);
rail.unshift(...lines.rail);
const guides = lines.guides;
const frame = { name, x: fx, y: fy, w: W, h: y };
const missing = shots.filter((s) => !existsSync(s.file));
const count = (k) => shots.filter((s) => s.from === k).length;
console.log(`${ch}: ${shots.length} pictures, ${y} px tall (${(y / S).toFixed(1)} screens) – luma ${count('luma')} · render ${count('render')} · existing ${count('existing')}${missing.length ? ` · MISSING ${missing.map((s) => s.id).join(', ')}` : ''}`);
if (dry) {
	if (guidesOnly) for (const l of [...guides.filter((g) => g.kind !== 'screen'), ...lines.rail].sort((a, b) => a.y - b.y)) console.log(`  ${String(l.y).padStart(6)}  ${l.kind ?? 'rail'}${l.label ? `  ${l.label}` : ''}`);
	else for (const s of shots.filter((s) => s.from !== "render")) console.log(`  ${s.id} ${s.from} ${s.file}`);
	process.exit(0);
}
/** the guides and the rail through the bridge module slice-guides – only those, the frame found by its exact name */
async function drawGuides() {
	const r = await figma('slice-guides', { page, frame: name, guides, rail, shots: shots.map((s) => ({ id: s.id, y: s.y })) }, { client: `figma-chapter ${ch}` });
	if (!r.drawn) { console.error(`NOTHING DRAWN – shots are not where the cut expects them:\n  ${r.moved.join('\n  ')}`); process.exit(1); }
	console.log(`guides: ${r.guides} lines (${r.labels} labels${r.outside ? `, ${r.outside} outside the frame` : ''}) · rail: ${r.rail} labels at x ${r.railX}${r.missing.length ? ` · not in the frame: ${r.missing.join(', ')}` : ''}${r.hidden ? ' · the guides layer is hidden (show it to see them)' : ''}`);
}
if (guidesOnly) { await drawGuides(); process.exit(0); }

async function send(spec) {
	const r = await fetch(`http://127.0.0.1:${port}/cmd`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ cmd: 'draft', args: { spec } }) }).catch(() => null);
	if (!r) throw new Error(`no relay on ${port} – node tools/figma-bridge/relay.mjs`);
	const j = await r.json();
	if (!j.ok) throw new Error(j.error);
	return j.result;
}
await send({ page, frame, guides: [], rail: [], shots: [] }); // the frame and its layers (guides cleared, hidden) – the lines come last
for (const [i, s] of shots.entries()) {
	if (!existsSync(s.file)) continue;
	const { file, from, endless, ...shot } = s;
	await send({ page, frame, shots: [{ ...shot, image: bytesOf(endless ? bandOf(s.id, file) : file) }] });
	process.stdout.write(`\r${i + 1}/${shots.length} ${s.id}            `);
}
console.log();
await drawGuides();
console.log(`→ Figma "${page}" / "${name}" at x ${fx}, y ${fy}`);
