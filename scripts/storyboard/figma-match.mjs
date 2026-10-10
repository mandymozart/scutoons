// The Figma pictures to the shots (owner 2026-10-10: "die dortigen bilder in den 3 kapiteln zu sortieren so dass diese
// neben dem frame auf hoehe der shot liegen denen sie am besten zugeordnet passen. am kapitel 4 und 5 probieren"):
// the chapter page holds the owner's pre-selection (photos by shooting, Unreal stills, first-draft placements); each
// picture gets the shot it fits best, and a COPY of it goes beside the storyboard draft frame, at that shot's height.
// The originals stay where they are.
//
//   node scripts/storyboard/figma-match.mjs collect storyboards/catpope/s1c4 --page "S1C4 - DEPRESSION"
//        → _out/<ch>/figma-match/pictures.json (one per distinct image), thumbs/<n>.jpg, sheets/sheet-<k>.jpg (48 a
//          sheet, numbered), shots.jpg (the shots' renders with their ids) – for the matching (a person or an agent
//          writes assign.json: { "<n>": "sb-36-4", … }, unassigned = left out)
//   node scripts/storyboard/figma-match.mjs place storyboards/catpope/s1c4 --page "S1C4 - DEPRESSION" --frame "s1c4 – storyboard draft 2026-10-10"
//        → copies beside the frame, a row per shot (group "<frame> · pictures", replaced on a re-run)

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { ROOT, S, W } from './lib.mjs';

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const [cmd, src] = args;
const page = take('--page');
const frameName = take('--frame');
if (!['collect', 'place'].includes(cmd) || !src || !page) { console.error('usage: figma-match.mjs collect|place <storyboard folder> --page "<Figma page>" [--frame "<draft frame>"]'); process.exit(2); }
const ch = basename(resolve(src));
const OUT = resolve(ROOT, 'storyboards/_out', ch);
const DIR = resolve(OUT, 'figma-match');
const port = Number(process.env.SCUTOONS_FIGMA_PORT ?? 3056);

async function bridge(c, a) {
	const r = await fetch(`http://127.0.0.1:${port}/cmd`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ cmd: c, args: a }) }).catch(() => null);
	if (!r) throw new Error(`no relay on ${port} – node tools/figma-bridge/relay.mjs`);
	const j = await r.json();
	if (!j.ok) throw new Error(j.error);
	return j.result;
}

if (cmd === 'collect') {
	mkdirSync(resolve(DIR, 'thumbs'), { recursive: true });
	mkdirSync(resolve(DIR, 'sheets'), { recursive: true });
	const tree = await bridge('read', { page, depth: 3 });
	const seen = new Map();
	const walk = (n, top, ox, oy) => {
		const x = (n.x ?? 0) + ox, y = (n.y ?? 0) + oy;
		if (n.image && !seen.has(n.image) && !/storyboard draft|thumbnail|keyvisual/.test(top ?? '') && n.w >= 60 && n.h >= 60) seen.set(n.image, { id: n.id, name: n.name, x, y, w: n.w, h: n.h, where: top ?? 'page' });
		for (const c of n.children ?? []) walk(c, top ?? n.name, n.type === 'PAGE' ? 0 : x, n.type === 'PAGE' ? 0 : y);
	};
	for (const c of tree.children ?? []) walk(c, null, 0, 0);
	const pictures = [...seen.values()].map((p, i) => ({ n: i + 1, ...p }));
	writeFileSync(resolve(DIR, 'pictures.json'), JSON.stringify(pictures, null, 1));
	console.log(`${pictures.length} distinct pictures on "${page}"`);
	for (let i = 0; i < pictures.length; i += 40) {
		const batch = pictures.slice(i, i + 40);
		const res = await bridge('thumbs', { ids: batch.map((p) => p.id), width: 200 });
		res.forEach((r, k) => { if (r.base64) writeFileSync(resolve(DIR, 'thumbs', `${batch[k].n}.jpg`), Buffer.from(r.base64, 'base64')); });
		process.stdout.write(`\rthumbnails ${Math.min(i + 40, pictures.length)}/${pictures.length}`);
	}
	console.log();
	// contact sheets (48 numbered thumbnails) and the shots' renders
	const draft = JSON.parse(readFileSync(resolve(OUT, 'chapter.draft.json'), 'utf8'));
	const shots = draft.groups.flatMap((g) => (g.renderer?.type === 'horizontal' ? [basename(g.slices[0].image).replace(/-\d+\.png$/, '')] : g.slices.filter((s) => /\/storyboard\//.test(s.image)).map((s) => basename(s.image).replace(/\.\w+$/, ''))));
	const py = `
import sys, os, json
from PIL import Image, ImageDraw
d, out, n = sys.argv[1], sys.argv[2], int(sys.argv[3])
cell, cols, rows = 200, 8, 6
for k in range(0, n, cols * rows):
    s = Image.new('RGB', (cols * cell, rows * (cell + 18)), (255, 255, 255)); g = ImageDraw.Draw(s)
    for j in range(cols * rows):
        i = k + j + 1
        if i > n: break
        f = os.path.join(d, 'thumbs', f'{i}.jpg')
        if not os.path.exists(f): continue
        im = Image.open(f).convert('RGB'); im.thumbnail((cell - 6, cell - 6))
        x, y = (j % cols) * cell, (j // cols) * (cell + 18)
        s.paste(im, (x + 3, y + 18)); g.rectangle([x, y, x + 34, y + 16], fill=(0, 0, 0)); g.text((x + 3, y + 3), str(i), fill=(255, 255, 255))
    s.save(os.path.join(out, f'sheet-{k // (cols * rows) + 1}.jpg'), quality=82)
`;
	execFileSync('python', ['-I', '-c', py, DIR, resolve(DIR, 'sheets'), String(pictures.length)]);
	const pyShots = `
import sys, os
from PIL import Image, ImageDraw
clean, out, ids = sys.argv[1], sys.argv[2], sys.argv[3].split(',')
cell, cols = 150, 10
rows = (len(ids) + cols - 1) // cols
s = Image.new('RGB', (cols * cell, rows * (cell * 2 + 18)), (255, 255, 255)); g = ImageDraw.Draw(s)
for j, i in enumerate(ids):
    f = os.path.join(clean, i + '.png')
    x, y = (j % cols) * cell, (j // cols) * (cell * 2 + 18)
    g.text((x + 3, y + 3), i, fill=(0, 0, 0))
    if os.path.exists(f):
        im = Image.open(f).convert('RGB'); im.thumbnail((cell - 6, cell * 2 - 6)); s.paste(im, (x + 3, y + 18))
s.save(out, quality=85)
`;
	execFileSync('python', ['-I', '-c', pyShots, resolve(OUT, 'clean'), resolve(DIR, 'shots.jpg'), shots.join(',')]);
	console.log(`→ ${DIR} (sheets/, shots.jpg) – write assign.json { "<n>": "<shot id>" }`);
}

if (cmd === 'place') {
	if (!frameName) { console.error('--frame "<draft frame name>"'); process.exit(2); }
	const pictures = JSON.parse(readFileSync(resolve(DIR, 'pictures.json'), 'utf8'));
	const assign = JSON.parse(readFileSync(resolve(DIR, 'assign.json'), 'utf8'));
	// the shots' heights in the frame – the same layout as figma-chapter.mjs
	const draft = JSON.parse(readFileSync(resolve(OUT, 'chapter.draft.json'), 'utf8'));
	const at = {};
	let y = 0;
	for (const g of draft.groups) {
		if (g.renderer?.type === 'horizontal') { at[basename(g.slices[0].image).replace(/-\d+\.png$/, '')] = { y, h: S }; y += S; continue; }
		for (const s of g.slices) { at[basename(s.image).replace(/\.\w+$/, '')] = { y, h: s.height }; y += s.height; }
	}
	const tree = await bridge('read', { page, depth: 1 });
	const frame = tree.children.find((n) => n.name === frameName);
	if (!frame) throw new Error(`no frame "${frameName}" on "${page}"`);
	const X0 = frame.x + W + 260, CW = 180, GAP = 10, PER_ROW = 6;
	const byShot = {};
	for (const [n, shot] of Object.entries(assign)) if (at[shot]) (byShot[shot] ??= []).push(pictures.find((p) => p.n === Number(n)));
	const items = [];
	for (const [shot, list] of Object.entries(byShot)) {
		let rowY = frame.y + at[shot].y, rowH = 0;
		list.filter(Boolean).forEach((p, i) => {
			if (i && i % PER_ROW === 0) { rowY += rowH + GAP; rowH = 0; }
			const h = (p.h * CW) / p.w;
			items.push({ id: p.id, shot, x: X0 + (i % PER_ROW) * (CW + GAP), y: Math.round(rowY), w: CW });
			rowH = Math.max(rowH, h);
		});
	}
	const r = await bridge('place', { page, group: `${frameName} · pictures`, items });
	console.log(`${r.placed} copies beside "${frameName}" – ${Object.keys(byShot).length} shots with pictures`);
}
