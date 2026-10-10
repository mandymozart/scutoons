// Figma draft → production draft (owner 2026-10-10: "aus den drafts dann production ready slices kopiert, die texte aus
// dem entwurf wieder entfernt, mit den 390x625 … in 1x 2x 3x in die richtigen aufloesungen exportiert … text box
// vorschlaege in den sprachen die verfuegbar sind … ein transfer json, das dann via mcp als draft auf die production
// seite geladen wird"; source: the storyboard draft frame, its `layout` layer on top).
//
//   node scripts/storyboard/figma-production.mjs storyboards/catpope/s1c3 --page "S1C3 - BARGAINING" --frame "s1c3 – storyboard draft 2026-10-09" [--only 1-5]
//
// Per scene group (compile.mjs's chapter draft – the same layout figma-chapter.mjs drew):
//   vertical    cut at shot boundaries into slices of 625 (the rest: a shorter slice; a rest < 160 joins the one above)
//   panorama    the strip as 390 × 844 slides
// each exported from the frame with `text` and `guides` hidden, at 1x / 2x / 3x → WebP
// `<toon>/s<n>/c<n>/s<n>c<n>-slice-NN.webp` (+ `.2x.webp`, `.3x.webp` = imageVariants low / medium / high).
// The texts come from the frame's `text` layer (as the owner edited them: wording + place), become the slice's dialog
// (rect in % of the slice, the preset's style from the chapter draft), German as written plus the other languages of
// the toon from production/translations.json when there (suggestions, marked in `meta`).
// Out: storyboards/_out/<ch>/production/ – chapter.json (the language-neutral chapter: the scene groups replaced by the
// sliced ones and nothing else – no test groups, no thumbnail slice), uploads.json [{ file, path }], slices/*.webp, preview.png. Nothing is uploaded here.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { ROOT, S, W, readEntity } from './lib.mjs';

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const page = take('--page'), frameName = take('--frame');
const only = take('--only', null)?.split('-').map(Number);
// --figma-slices <x>: first build the slices as frames in Figma (the owner's way – a frame per slice holding the
// layers active there, no text), at page x <x>, each at its draft height; then export FROM those frames (re-runs
// replace them; the owner can fix a slice frame in Figma and run with --from-figma-slices to export again)
const sliceX = take('--figma-slices', null);
const fromFrames = args.includes('--from-figma-slices') && args.splice(args.indexOf('--from-figma-slices'), 1);
const force = args.includes('--force') && args.splice(args.indexOf('--force'), 1);
const src = args[0];
if (!src || !page || !frameName) { console.error('usage: figma-production.mjs <storyboard folder> --page "<Figma page>" --frame "<draft frame>" [--only a-b]'); process.exit(2); }
const ch = basename(resolve(src));
const [, sn, cn] = /^s(\d+)c(\d+)$/.exec(ch) ?? [];
const OUT = resolve(ROOT, 'storyboards/_out', ch);
const PROD = resolve(OUT, 'production');
mkdirSync(resolve(PROD, 'slices'), { recursive: true });
mkdirSync(resolve(PROD, 'png'), { recursive: true });
const port = Number(process.env.SCUTOONS_FIGMA_PORT ?? 3056);
const SLICE = 625, MIN_REST = 160;

async function bridge(c, a) {
	const r = await fetch(`http://127.0.0.1:${port}/cmd`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ cmd: c, args: a }) }).catch(() => null);
	if (!r) throw new Error(`no relay on ${port} – node tools/figma-bridge/relay.mjs`);
	const j = await r.json();
	if (!j.ok) throw new Error(j.error);
	return j.result;
}

const draft = JSON.parse(readFileSync(resolve(OUT, 'chapter.draft.json'), 'utf8'));
const toon = draft.groups.flatMap((g) => g.slices)[0]?.image ? 'catpope' : 'catpope';
let languages = ['de'];
try { languages = readEntity(`toon:${toon}`)?.languages ?? languages; } catch {}
const tr = existsSync(resolve(PROD, 'translations.json')) ? JSON.parse(readFileSync(resolve(PROD, 'translations.json'), 'utf8')) : {};

// the frame and its texts
const tree = await bridge('read', { page, depth: 1 });
const frame = tree.children.find((n) => n.name === frameName);
if (!frame) throw new Error(`no frame "${frameName}" on "${page}"`);
const layers = await bridge('read', { nodeId: frame.id, depth: 2 });
const texts = (layers.children.find((l) => l.name === 'text')?.children ?? []).filter((t) => t.type === 'TEXT' && (t.text ?? '').trim());

// the chapter draft's layout (as figma-chapter.mjs drew it) and its dialog styles by shot
let y = 0;
const groups = draft.groups.map((g) => {
	const pano = g.renderer?.type === 'horizontal';
	const shots = pano ? [{ id: basename(g.slices[0].image).replace(/-\d+\.png$/, ''), y, h: S, dialog: g.slices.flatMap((s) => s.dialog ?? []) }]
		: g.slices.map((s) => { const o = { id: basename(s.image).replace(/\.\w+$/, ''), y, h: s.height, dialog: s.dialog ?? [] }; y += s.height; return o; });
	if (pano) y += S;
	return { g, pano, shots, own: /^scene-/.test(g.label ?? '') };
});

// a Figma text → a dialog of the slice that holds its top
const byShot = {};
for (const t of texts) { const id = t.name.split(' · ')[0]; (byShot[id] ??= []).push(t); }
function dialogFor(t, slice) {
	const id = t.name.split(' · ')[0];
	const k = byShot[id].indexOf(t);
	const shot = groups.flatMap((g) => g.shots).find((s) => s.id === id);
	const base = shot?.dialog?.[k] ?? {};
	const de = t.text.trim();
	const text = { de };
	for (const l of languages) if (l !== 'de' && tr[de]?.[l]) text[l] = tr[de][l];
	const { rect, text: _t, ...style } = base;
	return { ...style, id: base.id ?? `${ch}-${id}-${k + 1}`, rect: { x: +((t.x - slice.x) / slice.w * 100).toFixed(1), y: +((t.y - slice.y) / slice.h * 100).toFixed(1), width: +(t.w / slice.w * 100).toFixed(1) }, text };
}

// the slices
const slices = [];
for (const G of groups) {
	if (!G.own) continue;
	if (G.pano) {
		const s = G.shots[0], n = G.g.slices.length;
		for (let i = 0; i < n; i++) slices.push({ group: G, x: i * W, y: s.y, w: W, h: S, pano: true });
		continue;
	}
	const top = G.shots[0].y, bottom = G.shots.at(-1).y + G.shots.at(-1).h;
	const cuts = [];
	for (let at = top; at < bottom; at += SLICE) cuts.push([at, Math.min(at + SLICE, bottom)]);
	if (cuts.length > 1 && cuts.at(-1)[1] - cuts.at(-1)[0] < MIN_REST) { const last = cuts.pop(); cuts.at(-1)[1] = last[1]; }
	for (const [a, b] of cuts) slices.push({ group: G, x: 0, y: a, w: W, h: b - a });
}
slices.forEach((s, i) => { s.n = i + 1; s.name = `s${sn}c${cn}-slice-${String(i + 1).padStart(2, '0')}`; });
for (const t of texts) {
	const s = slices.find((s) => t.y >= s.y && t.y < s.y + s.h && t.x + t.w / 2 >= s.x && t.x + t.w / 2 < s.x + s.w + (s.pano ? 0 : 1e6) && (!s.pano || t.x >= s.x - 1));
	if (s) (s.dialog ??= []).push(dialogFor(t, s));
}
console.log(`${ch}: ${slices.length} slices (${slices.filter((s) => s.pano).length} panorama slides), ${texts.length} texts, languages ${languages.join(' + ')}${Object.keys(tr).length ? '' : ' (no translations.json yet – German only)'}`);

// export: 1x / 2x / 3x PNG from Figma → WebP
const todo = slices.filter((s) => !only || (s.n >= only[0] && s.n <= (only[1] ?? only[0])));
const frameIds = {};
if (sliceX != null) {
	for (let i = 0; i < todo.length; i += 10) {
		const part = todo.slice(i, i + 10);
		const made = await bridge('sliceFrames', { frame: frame.id, skip: ['text', 'guides'], slices: part.map((s) => ({ name: s.name, x: s.x, y: s.y, w: s.w, h: s.h, px: Number(sliceX) + s.x, py: frame.y + s.y })) });
		for (const m of made) frameIds[m.name] = m.id;
		process.stdout.write(`slice frames ${Math.min(i + 10, todo.length)}/${todo.length}   `);
	}
	console.log();
}
if (fromFrames && sliceX == null) {
	const pg = await bridge('read', { page, depth: 1 });
	for (const n of pg.children) if (/-slice-\d+$/.test(n.name)) frameIds[n.name] = n.id;
}
for (const s of todo) {
	if (!force && sliceX == null && !fromFrames && existsSync(resolve(PROD, 'slices', `${s.name}.3x.webp`))) continue;
	const res = frameIds[s.name]
		? await bridge('exportScales', { node: frameIds[s.name], scales: [1, 2, 3] })
		: await bridge('region', { frame: frame.id, x: s.x, y: s.y, w: s.w, h: s.h, hide: ['text', 'guides'], unclip: s.pano, scales: [1, 2, 3] });
	for (const r of res) writeFileSync(resolve(PROD, 'png', `${s.name}@${r.scale}x.png`), Buffer.from(r.base64, 'base64'));
	execFileSync('python', ['-I', '-c', `import sys
from PIL import Image
d, n = sys.argv[1], sys.argv[2]
for s, suf in ((1, ''), (2, '.2x'), (3, '.3x')):
    Image.open(f'{d}/png/{n}@{s}x.png').convert('RGB').save(f'{d}/slices/{n}{suf}.webp', 'WEBP', quality=90, method=6)`, PROD, s.name]);
	process.stdout.write(`\r${s.n}/${slices.length} ${s.name}   `);
}
console.log();

// the chapter: ONLY the scene groups as sliced – the stored test groups (mama-forest, mama-biker, weed) and the
// thumbnail placeholder slice go (owner 2026-10-10: "die mama scenen und weed koennten wir loeschen … das war alles nur
// test", "chapter 3 4 5 sollten eigentlich ausser dem thumbnail als ersten slice nichts haben … das der nicht bleibt")
const dir = `s${sn}/c${cn}`;
const chapter = { ...draft, groups: [] };
for (const G of groups) {
	if (!G.own) continue;
	const { slices: _s, ...g } = G.g;
	chapter.groups.push({ ...g, slices: slices.filter((s) => s.group === G).map((s) => ({
		image: `${dir}/${s.name}.webp`, width: s.w, height: s.h,
		imageVariants: { low: `${dir}/${s.name}.webp`, medium: `${dir}/${s.name}.2x.webp`, high: `${dir}/${s.name}.3x.webp` },
		...(s.dialog?.length ? { dialog: s.dialog } : {})
	})) });
}
chapter.meta = { ...(draft.meta ?? {}), production: { from: `Figma "${page}" / "${frameName}"`, date: new Date().toLocaleDateString('sv'), suggested: languages.filter((l) => l !== 'de') } };
writeFileSync(resolve(PROD, 'chapter.json'), JSON.stringify(chapter, null, '\t'));
const uploads = slices.flatMap((s) => ['', '.2x', '.3x'].map((suf) => ({ file: `slices/${s.name}${suf}.webp`, path: `${toon}/${dir}/${s.name}${suf}.webp` })));
writeFileSync(resolve(PROD, 'uploads.json'), JSON.stringify(uploads, null, '\t'));
const missingTr = [...new Set(texts.map((t) => t.text.trim()))].filter((de) => languages.some((l) => l !== 'de' && !tr[de]?.[l]));
writeFileSync(resolve(PROD, 'to-translate.json'), JSON.stringify(missingTr, null, '\t'));
console.log(`→ ${PROD}: chapter.json, uploads.json (${uploads.length} files), slices/ – ${missingTr.length} texts without a translation (to-translate.json)`);
