// Re-cut a chapter's continuous slices from the owner's hand-made Figma draft and swap ONLY the images in the production
// draft – the text boxes there stay (owner 2026-10-10, chapter 2: "vom meinem haendischen draft nur die slices aus figma
// neuziehen. ab scene 17 auch erst. davor benutzt ich horizontale layout als kuenstlerisches mittel", "der entwurf in 2
// ist aktueller als die slice bilder in production").
//
//   node scripts/storyboard/figma-reslice.mjs --key chapter:catpope:2 --group scene-17-24 --frame 2369:3211 --top 13867 \
//        --folder s1/c2/v2 [--export-only | --upload]
//
// --top: the draft's y of the group's first slice (found by matching the images of the old slice frames against the
// draft). Each slice of the group: the draft region top + i · 625, exported at 1x / 2x / 3x with the draft's reading
// texts hidden (narration, speech, notes, the title – they are text boxes or left out on production) and its lettering
// kept (onomatopoeia like ROARRR is part of the art there) → WebP <toon>/<folder>/<name>[.2x|.3x].webp (new paths: the
// published chapter keeps its images until the owner publishes). --upload then uploads them and saves a draft that
// only sets each slice's image + imageVariants on the version read.
// Lettering kept was the 2026-10-10 rule; since 2026-10-18 the pictures carry no text at all – figma-clean-slices.mjs.

import { execFileSync } from 'node:child_process';
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { basename, resolve } from 'node:path';
import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
import { ROOT } from './lib.mjs';

const args = process.argv.slice(2);
const take = (f, d) => { const i = args.indexOf(f); return i >= 0 ? args.splice(i, 2)[1] : d; };
const key = take('--key'), groupLabel = take('--group'), frameId = take('--frame'), top = Number(take('--top')), folder = take('--folder');
const upload = args.includes('--upload');
if (!key || !groupLabel || !frameId || !Number.isFinite(top) || !folder) { console.error('usage: figma-reslice.mjs --key chapter:<toon>:<n> --group <label> --frame <figma node id> --top <y> --folder <s1/c2/v2> [--upload]'); process.exit(2); }
const [, toon, n] = key.split(':');
const OUT = resolve(ROOT, 'storyboards/_out', `s1c${n}`, 'reslice');
mkdirSync(resolve(OUT, 'png'), { recursive: true });
mkdirSync(resolve(OUT, 'slices'), { recursive: true });

// the SCUTOONS MCP (as production-upload.mjs)
const cfg = JSON.parse(readFileSync(resolve(homedir(), '.claude.json'), 'utf8')).mcpServers?.scutoons;
const client = new Client({ name: 'scutoons-reslice', version: '1.0.0' });
await client.connect(new StdioClientTransport({ command: cfg.command, args: cfg.args, env: { ...process.env, ...cfg.env }, stderr: 'ignore' }));
const call = async (name, a) => {
	const r = await client.callTool({ name, arguments: a });
	const text = (r.content ?? []).map((c) => c.text ?? '').join('\n');
	let data; try { data = JSON.parse(text); } catch { data = text; }
	return { error: !!r.isError, data };
};
const bridge = async (c, a) => {
	const r = await fetch(`http://127.0.0.1:${process.env.SCUTOONS_FIGMA_PORT ?? 3056}/cmd`, { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify({ cmd: c, args: a, timeout: 600_000 }) }).catch(() => null); // a huge draft: wait up to 10 min
	if (!r) throw new Error('no relay – node tools/figma-bridge/relay.mjs');
	const j = await r.json();
	if (!j.ok) throw new Error(j.error);
	return j.result;
};

try {
	const cur = await call('read_entity', { key });
	const base = cur.data?.version?.id;
	const gi = cur.data.content.groups.findIndex((g) => g.label === groupLabel);
	if (gi < 0) throw new Error(`no group "${groupLabel}" in ${key}`);
	const group = cur.data.content.groups[gi];
	console.log(`${key} version ${base}: group ${gi} "${groupLabel}", ${group.slices.length} slices`);

	// the draft's reading texts (hidden) vs lettering (kept): a reading text has a sentence (two words with a lower-case
	// letter, or more than 20 characters) – or is the chapter title / a note
	// the draft's tree, read once and kept (a huge hand-made draft takes long to read – re-runs resume from the file)
	const treeFile = resolve(OUT, 'draft-tree.json');
	const draft = existsSync(treeFile) ? JSON.parse(readFileSync(treeFile, 'utf8')) : await bridge('read', { nodeId: frameId, depth: 3 });
	if (!existsSync(treeFile)) writeFileSync(treeFile, JSON.stringify(draft));
	const reading = (t) => /^(kapitel|“.*”|need to)/i.test(t.trim()) || t.trim().length > 20 || (t.trim().split(/\s+/).length >= 2 && /[a-zäöüß]/.test(t));
	// the owner's text boxes are instances of the component "Dialog Textbox" – hidden in the image, they become dialog
	const BOX = 'Dialog Textbox';
	const hide = [...new Set([BOX, ...draft.children.filter((c) => c.type === 'TEXT' && reading(c.text ?? '')).map((c) => c.name)])];
	console.log(`hidden texts: ${hide.length} (kept as lettering: ${draft.children.filter((c) => c.type === 'TEXT' && !reading(c.text ?? '')).map((c) => JSON.stringify(c.text)).join(' ')})`);

	const items = group.slices.map((s, i) => ({ i, name: basename(s.image).replace(/\.\w+$/, ''), y: top + i * 625, h: s.height ?? 625 }));

	// text boxes for the slices that have none on production yet (owner: "wo noch keine text boxen sind diese ergänzen"):
	// every Dialog Textbox instance and every free reading text of the draft, in the slice that holds its top
	const textOf = (n) => { const out = []; const w = (x) => { if (x.type === 'TEXT') out.push(x.text ?? ''); for (const c of x.children ?? []) w(c); }; w(n); return out.join('\n'); };
	const clean = (t) => t.replace(/\r\n?/g, '\n').split('\n').map((l) => l.trim()).join('\n').replace(/\n{3,}/g, '\n\n').trim();
	// only what the reader can see: inside the draft frame (the frame clips – a loose copy beside it is a note)
	const sources = draft.children.filter((c) => !c.hidden && c.x < 390 && c.x + c.w > 0 && (c.name === BOX || (c.type === 'TEXT' && reading(c.text ?? '') && !/^(kapitel|“.*”|need to)/i.test((c.text ?? '').trim()))));
	const trFile = resolve(OUT, 'translations.json');
	const tr = existsSync(trFile) ? JSON.parse(readFileSync(trFile, 'utf8')) : {};
	const added = {};
	for (const c of sources) {
		const de = clean(c.type === 'TEXT' ? c.text ?? '' : textOf(c));
		if (!de) continue;
		const it = items.find((x) => c.y >= x.y && c.y < x.y + x.h);
		if (!it || (group.slices[it.i].dialog ?? []).length) continue;
		if ((added[it.i] ?? []).some((d) => d.text.de === de)) continue; // the same text twice in a slice (a box and a loose text)
		const speech = !/^\d+\s+(INT|EXT)/.test(de) && (de.length < 70 || /[?!]$/.test(de)) && !/^PLF/.test(de);
		(added[it.i] ??= []).push({ id: `${toon}-c${n}-${it.name}-${(added[it.i]?.length ?? 0) + 1}`, rect: { x: +((Math.max(0, c.x) / 390) * 100).toFixed(1), y: +(((c.y - it.y) / it.h) * 100).toFixed(1), width: +((Math.min(c.w, 390 - Math.max(0, c.x)) / 390) * 100).toFixed(1), height: +((c.h / it.h) * 100).toFixed(1) }, variant: speech ? 'caption' : 'narration', size: 4, text: { de, en: tr[de]?.en ?? '' } });
	}
	const allDe = Object.values(added).flat().map((d) => d.text.de);
	writeFileSync(resolve(OUT, 'to-translate.json'), JSON.stringify(allDe.filter((d) => !tr[d]), null, '	'));
	console.log(`text boxes to add: ${allDe.length} in ${Object.keys(added).length} slices (${allDe.filter((d) => !tr[d]).length} without translation – to-translate.json)`);
	for (const it of items) {
		if (existsSync(resolve(OUT, 'slices', `${it.name}.3x.webp`))) continue;
		const res = await bridge('region', { frame: frameId, x: 0, y: it.y, w: 390, h: it.h, hide, partial: true, scales: [1, 2, 3] });
		for (const r of res) writeFileSync(resolve(OUT, 'png', `${it.name}@${r.scale}x.png`), Buffer.from(r.base64, 'base64'));
		execFileSync('python', ['-I', '-c', `import sys
from PIL import Image
d, n = sys.argv[1], sys.argv[2]
for s, suf in ((1, ''), (2, '.2x'), (3, '.3x')):
    Image.open(f'{d}/png/{n}@{s}x.png').convert('RGB').save(f'{d}/slices/{n}{suf}.webp', 'WEBP', quality=90, method=6)`, OUT, it.name]);
		process.stdout.write(`\rexport ${it.i + 1}/${items.length} ${it.name}   `);
	}
	console.log();
	writeFileSync(resolve(OUT, 'plan.json'), JSON.stringify({ key, base, group: groupLabel, index: gi, folder, items }, null, '\t'));
	if (!upload) { console.log(`exported → ${OUT}/slices – run again with --upload`); process.exit(0); }

	let fresh = 0, there = 0;
	for (const it of items) for (const suf of ['', '.2x', '.3x']) {
		const r = await call('upload_asset', { file: resolve(OUT, 'slices', `${it.name}${suf}.webp`), path: `${toon}/${folder}/${it.name}${suf}.webp` });
		if (!r.error) fresh++; else if (/409|exists/i.test(String(r.data))) there++; else throw new Error(`${it.name}${suf}: ${String(r.data).slice(0, 200)}`);
		process.stdout.write(`\rupload ${fresh + there}/${items.length * 3} (new ${fresh}, already there ${there})   `);
	}
	console.log();
	const ops = items.flatMap((it) => [
		{ op: 'set', path: ['groups', gi, 'slices', it.i, 'image'], value: `${folder}/${it.name}.webp` },
		{ op: 'set', path: ['groups', gi, 'slices', it.i, 'imageVariants'], value: { low: `${folder}/${it.name}.webp`, medium: `${folder}/${it.name}.2x.webp`, high: `${folder}/${it.name}.3x.webp` } }
	]);
	for (const [i, list] of Object.entries(added)) ops.push({ op: 'set', path: ['groups', gi, 'slices', Number(i), 'dialog'], value: list });
	const r = await call('save_draft', { key, base, ops, message: `${groupLabel}: slices re-cut from the Figma draft (1x/2x/3x, ${folder}); text boxes added where none were` });
	console.log(`save_draft: ${typeof r.data === 'string' ? r.data.slice(0, 400) : JSON.stringify(r.data).slice(0, 400)}`);
} finally {
	await client.close();
}
