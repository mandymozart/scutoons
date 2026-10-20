// Clean slices from Figma: text boxes AND lettering out of the pictures, the lettering back as text boxes (owner
// 2026-10-18: "slice chapters 1 and 2 again from Figma … WITHOUT text boxes and onomatopoeia – those are legacy", "all
// 3 resolutions"; the sound words "add as lettering boxes"; ads "need to also be translated", the fidget cube "just
// remove the original text"; the elevator numbers are "picture"). Replaces figma-reslice.mjs's rule (lettering kept).
//
//   node scripts/storyboard/figma-clean-slices.mjs --key chapter:catpope:1 --group scenes-1-to-10 --folder s1/c1/v2 \
//        --page 1526:10190 --prefix "S1C1 - Slice " [--upload]
//   node scripts/storyboard/figma-clean-slices.mjs --key chapter:catpope:2 --group scene-17-24 --folder s1/c2/v3 \
//        --frame 2369:3211 --top 13867 --changed-only [--upload]
//
// Source: the owner's per-slice frames (`--page` + `--prefix`: slice `…-slice-N` ↔ frame "<prefix>N"; slices without
// a frame – the key visual – stay) or one long draft frame (`--frame` + `--top`: slice i = the region top + i · 625).
// Hidden at any depth (bridge `region` hideIds): the instances Dialog Textbox / Speech Bubble / Action Button, every
// TEXT layer except KEEP (elevator floor numbers, the book title), the vector letter groups "ROAARR". A hidden lettering
// text (short, not a sentence) becomes a text box in its slice – font lettering, Figma's size / angle / colours, the
// word in every language – unless the slice already has a box there or with that word. --changed-only (a draft cut
// before, figma-reslice.mjs): only slices where something hides that the old rule kept (Dialog Textbox + top-level
// reading texts). Out: storyboards/_out/s1c<n>/clean/ (slices/*.webp, plan.json); --upload: new CDN paths (never
// overwrites) + ONE draft that sets image + imageVariants and appends the lettering boxes – nothing else changes.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { basename, resolve } from 'node:path';
import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
import { ROOT, SLICE_H } from './lib.mjs';

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const key = take('--key'), groupLabel = take('--group'), folder = take('--folder');
const page = take('--page'), prefix = take('--prefix'), frameId = take('--frame'), top = Number(take('--top'));
// --color word=#hex: a lettering whose fill the bridge can't give (a gradient) – e.g. sampled from the export
const colors = Object.fromEntries((take('--color') ?? '').split(',').filter(Boolean).map((x) => x.split('=')));
// --hide id,id: more layers to hide (a panel that only held a text – owner's pick)
const extra = new Set((take('--hide') ?? '').split(',').filter(Boolean));
const upload = args.includes('--upload'), changedOnly = args.includes('--changed-only');
if (!key || !groupLabel || !folder || !((page && prefix) || (frameId && Number.isFinite(top)))) {
	console.error('usage: figma-clean-slices.mjs --key chapter:<toon>:<n> --group <label> --folder <s1/c1/v2> (--page <id> --prefix "<name> " | --frame <id> --top <y> [--changed-only]) [--upload]');
	process.exit(2);
}
const [, toon, n] = key.split(':');
const OUT = resolve(ROOT, 'storyboards/_out', `s1c${n}`, 'clean');
for (const d of ['png', 'slices', 'trees']) mkdirSync(resolve(OUT, d), { recursive: true });
const W = 390, H = SLICE_H;

const cfg = JSON.parse(readFileSync(resolve(homedir(), '.claude.json'), 'utf8')).mcpServers?.scutoons;
const client = new Client({ name: 'scutoons-clean-slices', version: '1.0.0' });
await client.connect(new StdioClientTransport({ command: cfg.command, args: cfg.args, env: { ...process.env, ...cfg.env }, stderr: 'ignore' }));
const call = async (name, a) => {
	const r = await client.callTool({ name, arguments: a });
	const text = (r.content ?? []).map((c) => c.text ?? '').join('\n');
	let data; try { data = JSON.parse(text); } catch { data = text; }
	return { error: !!r.isError, data };
};
const bridge = async (c, a, tries = 5) => {
	const r = await fetch(`http://127.0.0.1:${process.env.SCUTOONS_FIGMA_PORT ?? 3056}/cmd`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ cmd: c, args: a, timeout: 600_000 }) }).catch(() => null);
	const j = r && await r.json().catch(() => null);
	if (!j?.ok && tries > 1 && (!j || /no plugin/.test(j.error))) { // a relay / plugin restart: wait and try again
		await new Promise((ok) => setTimeout(ok, 10_000));
		return bridge(c, a, tries - 1);
	}
	if (!j) throw new Error('no relay – node tools/figma-bridge/relay.mjs');
	if (!j.ok) throw new Error(j.error);
	return j.result;
};
const tree = async (id) => { // cached: re-runs resume without reading Figma again
	const f = resolve(OUT, 'trees', `${id.replace(':', '-')}.json`);
	if (existsSync(f)) return JSON.parse(readFileSync(f, 'utf8'));
	const t = await bridge('read', { nodeId: id, depth: 6 });
	writeFileSync(f, JSON.stringify(t));
	return t;
};

const KEEP = (t) => /^[0-9E]$/.test(t.trim()) || /Boccaccio|Decameron/.test(t);
const BOX = /^(Dialog Textbox|Speech Bubble|Action Button)/;
const reading = (t) => t.trim().length > 20 || (t.trim().split(/\s+/).length >= 2 && /[a-zäöüß]/.test(t));
const norm = (s) => String(s ?? '').toUpperCase().replace(/[^A-ZÄÖÜ0-9?!]/g, '');

// what to hide in a tree; positions relative to the root frame (TEXT by its rotated page box, the rest by x / y)
function classify(root, decor) {
	const hide = [], letters = [];
	const walk = (node, depth) => {
		const parent = node.id;
		for (const c of node.children ?? []) {
			if (c.hidden) continue;
			const box = c.abs ? { x: c.abs.x - root.abs.x, y: c.abs.y - root.abs.y, w: c.abs.w, h: c.abs.h } : { x: c.x, y: c.y, w: c.w, h: c.h };
			if (extra.has(c.id)) { hide.push({ id: c.id, box, old: false }); continue; }
			const old = depth === 0 && (c.name === 'Dialog Textbox' || (c.type === 'TEXT' && reading(c.text ?? ''))); // figma-reslice.mjs hid these
			if (((c.type === 'INSTANCE' || c.type === 'FRAME') && BOX.test(c.name)) || (c.type === 'TEXT' && !KEEP(c.text ?? ''))) {
				hide.push({ id: c.id, box, old, name: c.type === 'INSTANCE' ? c.name : undefined, parent });
				if (c.type === 'TEXT' && !reading(c.text ?? '')) letters.push({ node: c, box });
				continue;
			}
			if (c.type === 'GROUP' && /^ROAARR/.test(c.name)) { hide.push({ id: c.id, box, old: false }); if (!/Stroke/.test(c.name)) letters.push({ node: { ...c, text: 'ROAARR', fontSize: c.h }, box }); continue; }
			if (c.type === 'VECTOR' && c.w <= 40 && c.h <= 40) small.push({ id: c.id, box, parent });
			if (c.type !== 'INSTANCE') walk(c, depth + 1);
		}
	};
	const small = [];
	walk(root, 0);
	// a box's decoration (the ♪ ♫ of a lyrics box, small vectors beside it in the same layer) goes with it – only in
	// per-slice frames (in the long draft small vectors in a box's area are art) – the reader draws its own
	const near = (a, b) => a.x < b.x + b.w + 10 && a.x + a.w > b.x - 10 && a.y < b.y + b.h + 10 && a.y + a.h > b.y - 10;
	const boxes = hide.filter((h) => BOX.test(h.name ?? ''));
	if (decor) for (const v of small) if (boxes.some((b) => b.parent === v.parent && near(v.box, b.box))) hide.push({ ...v, old: false });
	return { hide, letters };
}

// a lettering text box from a hidden Figma text, in a slice whose top is `y0` in the root's coordinates
const letterBox = (l, y0, id) => {
	const t = l.node, size = (t.fontSize ?? 24) / W * 100, cx = l.box.x + l.box.w / 2, cy = l.box.y + l.box.h / 2 - y0;
	const text = (t.text ?? '').replace(/\r\n?/g, '\n').split('\n').map((s) => s.trim()).filter(Boolean).join('\n');
	// auto width (a word never wraps) anchored where Figma's frame holds the text: its left / right edge (rotated with it)
	// or its centre – the word sits where it was drawn, give or take the rotation of a shorter box
	const align = ({ LEFT: 'left', RIGHT: 'right' })[t.align] ?? 'center', rot = -(t.rotation ?? 0) * Math.PI / 180;
	const side = align === 'left' ? -1 : align === 'right' ? 1 : 0, half = (t.w ?? l.box.w) / 2 * side;
	const px = cx + half * Math.cos(rot), py = cy + half * Math.sin(rot);
	return {
		id, rect: { x: +(px / W * 100).toFixed(2), y: +(py / H * 100).toFixed(2), width: +((t.w ?? l.box.w) * 1.5 / W * 100).toFixed(2) }, fit: 'width', // max width 1.5 × Figma's: never reached
		anchor: side < 0 ? 'l' : side > 0 ? 'r' : 'c', text: { de: text, en: text }, variant: 'bubble', font: 'lettering', align, size: +size.toFixed(2),
		...(colors[text] ? { color: colors[text] } : t.fill && t.fill !== '#000000' ? { color: t.fill } : {}),
		...(t.stroke ? { stroke: t.stroke, strokeWidth: +Math.max(0.05, (t.strokeWeight ?? 1) / (t.fontSize ?? 24)).toFixed(2) } : {}),
		...(t.rotation ? { rotation: -t.rotation } : {})
	};
};
// an existing box already says it: the same word (in this slice or a neighbour), or a lettering box over the spot here
// (HÄÄÄH? = Hääääh?, Kapitel ⊂ 2. Kapitel)
const same = (a, b) => { const x = norm(a).replace(/(.)\1+/g, '$1'), y = norm(b).replace(/(.)\1+/g, '$1'); return x === y || (y.length >= 4 && x.length <= y.length + 3 && x.includes(y)); };
const said = (e, d) => Object.values(typeof e.text === 'object' && !Array.isArray(e.text) ? e.text : { x: e.text }).some((v) => same(v, d.text.de));
const over = (e, d) => (e.font === 'lettering' || (e.variant ?? 'bubble') === 'bubble') && d.rect.x >= e.rect.x && d.rect.x <= e.rect.x + (e.rect.width ?? 10) && d.rect.y >= e.rect.y && d.rect.y <= e.rect.y + (e.rect.height ?? 10);

try {
	const cur = await call('read_entity', { key });
	const base = cur.data?.version?.id;
	const gi = cur.data.content.groups.findIndex((g) => g.label === groupLabel);
	if (gi < 0) throw new Error(`no group "${groupLabel}" in ${key}`);
	const group = cur.data.content.groups[gi];
	console.log(`${key} version ${base}: group ${gi} "${groupLabel}", ${group.slices.length} slices`);

	const items = [], kept = [];
	if (page) {
		const p = await bridge('read', { nodeId: page, depth: 1 });
		const frames = new Map(p.children.filter((c) => c.type === 'FRAME' && c.name.startsWith(prefix)).map((c) => [c.name.slice(prefix.length), c.id]));
		for (const [i, s] of group.slices.entries()) {
			const num = s.image.match(/slice-(\d+)\.\w+$/)?.[1], id = num && frames.get(num);
			if (!id) { kept.push(`${basename(s.image)} (no Figma frame)`); continue; }
			const t = await tree(id);
			const { hide, letters } = classify(t, true);
			items.push({ i, name: basename(s.image).replace(/\.\w+$/, ''), frame: id, region: { x: 0, y: 0 }, hideIds: hide.map((h) => h.id), boxes: letters.map((l, k) => letterBox(l, 0, `${toon}-c${n}-${num}-l${k + 1}`)) });
			process.stdout.write(`\rread ${i + 1}/${group.slices.length}   `);
		}
		console.log();
	} else {
		const t = await tree(frameId);
		const { hide, letters } = classify(t);
		for (const [i, s] of group.slices.entries()) {
			const y0 = top + i * H, y1 = y0 + (s.height ?? H), hit = (b) => b.y < y1 && b.y + b.h > y0 && b.x < W && b.x + b.w > 0;
			const name = basename(s.image).replace(/\.\w+$/, ''), num = name.match(/(\d+)$/)?.[1];
			const mine = letters.filter((l) => { const cy = l.box.y + l.box.h / 2; return cy >= y0 && cy < y1 && l.box.x < W && l.box.x + l.box.w > 0; }); // a word running out of the frame stays (the art's way)
			if (changedOnly && !hide.some((h) => !h.old && hit(h.box))) { kept.push(`${name} (nothing new to hide)`); continue; }
			items.push({ i, name, frame: frameId, region: { x: 0, y: y0 }, partial: true, hideIds: hide.filter((h) => hit(h.box)).map((h) => h.id), boxes: mine.map((l, k) => letterBox(l, y0, `${toon}-c${n}-${num}-l${k + 1}`)) });
		}
	}
	// a word across a seam sits in both slice frames: the lower slice owns it (research/TEXT-ACROSS-SLICES.md §1)
	for (const it of items) {
		const next = items.find((x) => x.i === it.i + 1);
		if (next) it.boxes = it.boxes.filter((d) => !next.boxes.some((e) => norm(e.text.de) === norm(d.text.de) && Math.abs(e.rect.y + 100 - d.rect.y) < 2 && Math.abs(e.rect.x - d.rect.x) < 2));
	}
	// this script's own boxes (made once; a re-run leaves them)
	const own = (e) => new RegExp(`^${toon}-c${n}-\\d+-l\\d+$`).test(e.id ?? '');
	const dl = (i) => (group.slices[i]?.dialog ?? []).filter((e) => !own(e));
	for (const it of items) it.boxes = it.boxes.filter((d) => !dl(it.i).some((e) => said(e, d) || over(e, d)) && ![...dl(it.i - 1), ...dl(it.i + 1)].some((e) => said(e, d)));
	console.log(`replace ${items.length}, keep ${kept.length}; lettering boxes to add: ${items.reduce((a, it) => a + it.boxes.length, 0)}`);

	for (const it of items) {
		if (existsSync(resolve(OUT, 'slices', `${it.name}.3x.webp`))) continue;
		const res = await bridge('region', { frame: it.frame, x: it.region.x, y: it.region.y, w: W, h: group.slices[it.i].height ?? H, hideIds: it.hideIds, partial: !!it.partial, scales: [1, 2, 3] });
		for (const r of res) writeFileSync(resolve(OUT, 'png', `${it.name}@${r.scale}x.png`), Buffer.from(r.base64, 'base64'));
		execFileSync('python', ['-I', '-c', `import sys
from PIL import Image
d, n = sys.argv[1], sys.argv[2]
for s, suf in ((1, ''), (2, '.2x'), (3, '.3x')):
    Image.open(f'{d}/png/{n}@{s}x.png').convert('RGB').save(f'{d}/slices/{n}{suf}.webp', 'WEBP', quality=90, method=6)`, OUT, it.name]);
		process.stdout.write(`\rexport ${items.indexOf(it) + 1}/${items.length} ${it.name}   `);
	}
	console.log();
	if (changedOnly) { // what comes out pixel-identical to the picture there now stays
		const same = execFileSync('python', ['-I', '-c', `import sys, io, urllib.request
from PIL import Image, ImageChops, ImageStat
d, cdn = sys.argv[1], sys.argv[2]
for a in sys.argv[3:]:
    name, img = a.split('|')
    old = Image.open(io.BytesIO(urllib.request.urlopen(f'{cdn}/{img}').read())).convert('L')
    new = Image.open(f'{d}/slices/{name}.webp').convert('L')
    if old.size == new.size and ImageStat.Stat(ImageChops.difference(old, new)).mean[0] < 0.05: print(name)`, OUT, `https://cdn.scutoons.com/${toon}`, ...items.filter((it) => !group.slices[it.i].image.startsWith(`${folder}/`)).map((it) => `${it.name}|${group.slices[it.i].image}`)] /* a re-run: already ours */, { encoding: 'utf8' }).split(/\s+/).filter(Boolean);
		for (const name of same) { kept.push(`${name} (pixel-identical)`); items.splice(items.findIndex((it) => it.name === name), 1); }
		console.log(`pixel-identical, kept: ${same.join(' ') || 'none'} → replace ${items.length}`);
	}
	writeFileSync(resolve(OUT, 'plan.json'), JSON.stringify({ key, base, group: groupLabel, index: gi, folder, kept, items }, null, '\t'));
	if (!upload) { console.log(`exported → ${OUT}/slices (plan.json) – run again with --upload`); process.exit(0); }

	let fresh = 0, there = 0;
	for (const it of items) for (const suf of ['', '.2x', '.3x']) {
		const r = await call('upload_asset', { file: resolve(OUT, 'slices', `${it.name}${suf}.webp`), path: `${toon}/${folder}/${it.name}${suf}.webp` });
		if (!r.error) fresh++; else if (/409|exists/i.test(String(r.data))) there++; else throw new Error(`${it.name}${suf}: ${String(r.data).slice(0, 200)}`);
		process.stdout.write(`\rupload ${fresh + there}/${items.length * 3} (new ${fresh}, already there ${there})   `);
	}
	console.log();
	const ops = items.flatMap((it) => [
		{ op: 'set', path: ['groups', gi, 'slices', it.i, 'image'], value: `${folder}/${it.name}.webp` },
		{ op: 'set', path: ['groups', gi, 'slices', it.i, 'imageVariants'], value: { low: `${folder}/${it.name}.webp`, medium: `${folder}/${it.name}.2x.webp`, high: `${folder}/${it.name}.3x.webp` } },
		// boxes once: a slice that has this script's boxes keeps them as they are now (the owner may have edited them)
		...(it.boxes.length && !(group.slices[it.i].dialog ?? []).some(own) ? [{ op: 'set', path: ['groups', gi, 'slices', it.i, 'dialog'], value: [...it.boxes, ...dl(it.i)] }] : [])
	]);
	const r = await call('save_draft', { key, base, ops, message: `${groupLabel}: ${items.length} slices re-cut clean from Figma (no text boxes / lettering, 1x/2x/3x, ${folder}); lettering as text boxes` });
	console.log(`save_draft: ${typeof r.data === 'string' ? r.data.slice(0, 400) : JSON.stringify(r.data).slice(0, 400)}`);
} finally {
	await client.close();
}
