// A survey of a Figma page for the vector library (shared/tokens/vectors, owner 2026-10-17: "identify all of those …
// ungroup them to have a greater variety"): reads the page over the SCUTOONS Figma bridge, picks the candidates – the
// named things, every group and the parts inside it ("ungrouped": a part big enough on its own), loose vectors – drops
// copies (same name, size and child count), exports a thumbnail of each and writes survey.json. Nothing in the Figma
// file changes. A candidate whose node is already in the library (vectors.json, incl. `figma.copies`) carries `library:
// <slug>`. Output: media/scutoons/vectors-survey/<page>/ (the local dev media, not in git).
//   node scripts/vectors-survey.mjs [--page Vector] [--min 80] [--no-thumbs] [--from-tree]
import fs from 'node:fs';
import zlib from 'node:zlib';
import path from 'node:path';
import { bridge as call } from '../tools/figma-bridge/cli.mjs';

async function bridge(cmd, a) {
	const r = await call(cmd, a);
	if (!r.ok) throw new Error(r.error ?? `${cmd} failed`);
	return r.result;
}

const args = process.argv.slice(2);
const opt = (name, fallback) => (args.includes(name) ? args[args.indexOf(name) + 1] : fallback);
const pageName = opt('--page', 'Vector');
/** the smallest a candidate may be on its longer side (Figma px) */
const MIN = Number(opt('--min', 80));
/** a part of a group counts on its own from this share of the group's area */
const PART = 0.02;
/** a frame this big or this full is a sheet to look through, not a shape */
const SHEET = { side: 3000, children: 60 };
const THUMB = 160;
/** a single unnamed vector counts from this size on its longer side */
const VECTOR_MIN = Number(opt('--vector-min', 250));

const out = path.resolve(import.meta.dirname, '..', 'media/scutoons/vectors-survey', pageName.replace(/[^\w-]+/g, '-'));
fs.mkdirSync(path.join(out, 'thumbs'), { recursive: true });
const treeFile = path.join(out, 'tree.json');
const tree = args.includes('--from-tree') ? JSON.parse(fs.readFileSync(treeFile, 'utf8')) : await bridge('read', { page: pageName, depth: 12 });
fs.writeFileSync(treeFile, JSON.stringify(tree));

const GENERIC = /^(Vector|Group|Frame|Union|Subtract|Intersect|Exclude|Rectangle|Ellipse|Line|Polygon|Star)( \d+)?$/;
const SHAPES = new Set(['GROUP', 'FRAME', 'INSTANCE', 'COMPONENT', 'BOOLEAN_OPERATION', 'VECTOR']);
const big = (n) => Math.max(n.w, n.h) >= MIN;
const sheet = (n) => (n.type === 'FRAME' || n.type === 'PAGE') && (n.w > SHEET.side || n.h > SHEET.side || (n.childCount ?? 0) > SHEET.children || n.type === 'PAGE');

const found = [];
function walk(node, trail, parent) {
	const named = !GENERIC.test(node.name);
	const shape = SHAPES.has(node.type) && !node.hidden && !node.image;
	let take = false;
	if (shape && big(node) && !sheet(node)) {
		if (named) take = true;
		else if (!parent || sheet(parent)) take = true; // loose on a sheet
		else if (node.type !== 'VECTOR' || (node.w * node.h) / (parent.w * parent.h || 1) >= PART) take = (node.w * node.h) / (parent.w * parent.h || 1) >= PART;
	}
	// a single vector on its own: big enough and not just a line (the lines only work in their composition)
	if (take && node.type === 'VECTOR' && !named) take = Math.max(node.w, node.h) >= VECTOR_MIN && Math.min(node.w, node.h) >= 0.08 * Math.max(node.w, node.h);
	if (take) found.push({ id: node.id, name: node.name, type: node.type, w: node.w, h: node.h, children: node.childCount ?? 0, named, trail: trail.join(' / ') });
	for (const c of node.children ?? []) walk(c, node.type === 'PAGE' ? trail : [...trail, `${node.name} ${node.id}`], node);
}
walk(tree, [], null);

// copies: the same name, size and child count – keep the first, count the rest
const seen = new Map();
const candidates = [];
for (const c of found) {
	const key = `${c.name}|${c.w}|${c.h}|${c.children}`;
	if (seen.has(key)) seen.get(key).copies.push(c.id);
	else {
		const entry = { ...c, copies: [] };
		seen.set(key, entry);
		candidates.push(entry);
	}
}
console.log(`${found.length} found, ${candidates.length} after copies (${candidates.filter((c) => c.named).length} named)`);

// already in the library (by Figma node): marked, so the contact sheet can show only the new ones
const library = new Map();
for (const v of JSON.parse(fs.readFileSync(path.resolve(import.meta.dirname, '../shared/tokens/vectors/vectors.json'), 'utf8')).vectors)
	for (const node of [v.figma?.node, ...(v.figma?.copies ?? [])]) if (node) library.set(node, v.slug);
for (const c of candidates) {
	const known = [c.id, ...c.copies].find((id) => library.has(id));
	if (known) c.library = library.get(known);
}
console.log(`${candidates.filter((c) => c.library).length} already in the library`);

if (!args.includes('--no-thumbs')) {
	let i = 0;
	for (const c of candidates) {
		const file = path.join(out, 'thumbs', `${c.id.replace(/[^\w]+/g, '_')}.png`);
		c.thumb = `thumbs/${path.basename(file)}`;
		if (fs.existsSync(file)) continue;
		const scale = Math.min(1, Math.max(0.01, THUMB / Math.max(c.w, c.h)));
		try {
			const r = await bridge('export', { nodeId: c.id, format: 'png', scale });
			fs.writeFileSync(file, Buffer.from(r.base64, 'base64'));
		} catch (err) {
			c.error = String(err.message ?? err);
		}
		if (++i % 25 === 0) console.log(`${i} thumbs`);
	}
}
/** a thumbnail with nothing to see: every pixel transparent (Figma: "no visible layers" – paths without fill or stroke) */
function invisible(file) {
	const png = fs.readFileSync(file);
	const width = png.readUInt32BE(16);
	const channels = { 6: 4, 4: 2, 2: 3, 0: 1 }[png[25]] ?? 4;
	const parts = [];
	for (let at = 8; at < png.length; ) {
		const len = png.readUInt32BE(at);
		if (png.toString('latin1', at + 4, at + 8) === 'IDAT') parts.push(png.subarray(at + 8, at + 8 + len));
		at += len + 12;
	}
	const raw = zlib.inflateSync(Buffer.concat(parts));
	const row = width * channels + 1;
	// a transparent image filters to zeros under every PNG filter: everything but each row's filter byte is 0
	for (let i = 0; i < raw.length; i++) if (i % row !== 0 && raw[i] !== 0) return false;
	return true;
}

const hidden = [];
for (const c of [...candidates]) {
	if (!c.thumb || c.error || !fs.existsSync(path.join(out, c.thumb)) || !invisible(path.join(out, c.thumb))) continue;
	hidden.push({ id: c.id, name: c.name, w: c.w, h: c.h });
	candidates.splice(candidates.indexOf(c), 1);
}
if (hidden.length) console.log(`${hidden.length} without visible layers left out`);
fs.writeFileSync(path.join(out, 'survey.json'), JSON.stringify({ page: pageName, min: MIN, candidates, invisible: hidden }, null, '\t'));
console.log(`→ ${path.relative(process.cwd(), out)}`);
