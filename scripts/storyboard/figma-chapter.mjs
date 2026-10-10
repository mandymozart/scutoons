// The whole chapter as ONE long draft frame in Figma (owner 2026-10-10: "create a new draft frame very long like the one
// there is … just with our updated storyboard", "if there is already luma renders we use them", "the height of the
// frame depends on the amount of images our story board generated and of course on their aspect ratio").
//
// Reads the chapter draft (compile.mjs → _out/<ch>/chapter.draft.json: the order, every shot's height from its format +
// aspect, the texts) and sends it through the SCUTOONS Figma bridge (tools/figma-bridge) as one frame, 390 wide,
// as tall as the shots stacked:
//   storyboard (locked)  per shot its picture – a Luma result if there is one (the chosen image in storyboards/dataset,
//                        else the newest in _out/<ch>/generated/), otherwise the render without texts (_out/<ch>/clean/);
//                        a panorama as its whole strip, one screen high (the frame shows its first screen)
//   guides (hidden)      every screen (844) and slice (625) · text  the texts as live text · layout  empty, on top
// and the rail beside it (shot · source). Re-runs replace shots by id and never touch `layout`.
//
//   node scripts/storyboard/figma-chapter.mjs storyboards/catpope/s1c3 --page "S1C3 - BARGAINING" [--x 7400] [--y -109] [--name "…"] [--dry]

import { execFileSync } from 'node:child_process';
import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { basename, extname, resolve } from 'node:path';
import { ROOT, S, SLICE_H, W, textOf, mediaRoot } from './lib.mjs';

const args = process.argv.slice(2);
const take = (flag, dflt) => { const i = args.indexOf(flag); return i >= 0 ? args.splice(i, 2)[1] : dflt; };
const dry = args.includes('--dry') && args.splice(args.indexOf('--dry'), 1);
const page = take('--page');
const fx = Number(take('--x', 7400)), fy = Number(take('--y', 0));
const src = args[0];
if (!src || !page) { console.error('usage: figma-chapter.mjs <storyboard folder> --page "<Figma page>" [--x n] [--y n] [--name text] [--dry]'); process.exit(2); }
const ch = basename(resolve(src));
const name = take('--name', `${ch} – storyboard draft ${new Date().toLocaleDateString('sv')}`);
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
const texts = (dialog, x0, top, h) => (dialog ?? []).map((d) => ({
	x: +(x0 + ((d.rect?.x ?? 5) / 100) * W).toFixed(1), y: +(top + ((d.rect?.y ?? 5) / 100) * h).toFixed(1), w: +(((d.rect?.width ?? 60) / 100) * W).toFixed(1),
	text: textOf(d.text), lettering: d.font === 'lettering', variant: d.variant ?? 'narration', size: d.font === 'lettering' ? +((((d.size ?? 15) / 100) * W)).toFixed(1) : undefined, title: !!d.title
}));

// the layout: every group's slices stacked; a horizontal group (panorama) = one screen high
let y = 0;
const shots = [], rail = [];
for (const g of draft.groups) {
	rail.push({ y, label: `— ${g.label}` });
	const pano = g.renderer?.type === 'horizontal';
	if (pano) {
		const id = basename(g.slices[0].image).replace(/-\d+\.png$/, '');
		const file = luma(id) ?? resolve(OUT, 'clean', `${id}.png`);
		const w = g.slices.reduce((a, s) => a + s.width, 0);
		shots.push({ id, file, from: file.includes('clean') ? 'render' : 'luma', x: 0, y, w, h: S, texts: g.slices.flatMap((s, i) => texts(s.dialog, i * W, y, S)) });
		rail.push({ y: y + 16, label: `${id}  panorama ${g.slices.length} screens · ${shots.at(-1).from}` });
		y += S;
		continue;
	}
	for (const s of g.slices) {
		const own = /\/storyboard\//.test(s.image);
		const id = basename(s.image).replace(/\.\w+$/, '');
		const file = own ? luma(id) ?? resolve(OUT, 'clean', `${id}.png`) : resolve(mediaRoot(), 'catpope', s.image);
		const from = !own ? 'existing' : file.includes(`${'clean'}`) ? 'render' : 'luma';
		shots.push({ id, file, from, x: 0, y, w: s.width, h: s.height, texts: texts(s.dialog, 0, y, s.height) });
		rail.push({ y: y + 16, label: `${id}  ${from}` });
		y += s.height;
	}
}
const guides = [];
for (let gy = S; gy < y; gy += S) guides.push({ y: gy, kind: 'screen' });
for (let gy = SLICE_H; gy < y; gy += SLICE_H) guides.push({ y: gy, kind: 'slice' });
const frame = { name, x: fx, y: fy, w: W, h: y };
const missing = shots.filter((s) => !existsSync(s.file));
const count = (k) => shots.filter((s) => s.from === k).length;
console.log(`${ch}: ${shots.length} pictures, ${y} px tall (${(y / S).toFixed(1)} screens) – luma ${count('luma')} · render ${count('render')} · existing ${count('existing')}${missing.length ? ` · MISSING ${missing.map((s) => s.id).join(', ')}` : ''}`);
if (dry) { for (const s of shots.filter((s) => s.from !== "render")) console.log(`  ${s.id} ${s.from} ${s.file}`); process.exit(0); }

async function send(spec) {
	const r = await fetch(`http://127.0.0.1:${port}/cmd`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ cmd: 'draft', args: { spec } }) }).catch(() => null);
	if (!r) throw new Error(`no relay on ${port} – node tools/figma-bridge/relay.mjs`);
	const j = await r.json();
	if (!j.ok) throw new Error(j.error);
	return j.result;
}
await send({ page, frame, guides, rail, shots: [] });
for (const [i, s] of shots.entries()) {
	if (!existsSync(s.file)) continue;
	const { file, from, ...shot } = s;
	await send({ page, frame, shots: [{ ...shot, image: bytesOf(file) }] });
	process.stdout.write(`\r${i + 1}/${shots.length} ${s.id}            `);
}
console.log(`\n→ Figma "${page}" / "${name}" at x ${fx}, y ${fy}`);
