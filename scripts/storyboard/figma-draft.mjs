// The first throw as Figma draft frames (STORYBOARD.md §9 "The first throw lands in Figma"): per scene a frame
// "s1c3 / scene-25 – draft", 390 wide, on a page of its own, holding
//   storyboard  (locked) the schematic shots as SVG at their y – hide it to judge the layout alone
//   guides      (locked, hidden by default) a line every screen (844) and every slice (625)
//   text        the dialog as real TEXT layers (Track A's convention: text becomes live text boxes later)
//   layout      empty, on top – the owner's work
// and the rhythm rail as text beside the frame (x > 390, never exported).
//
// Writes Plugin-API scripts (≤ 45,000 characters each, the use_figma limit is 50,000) – one or more per scene – to run
// through Figma's MCP (`use_figma`) or the SCUTOONS Figma bridge. Nothing is sent from here. Idempotent: a re-run
// replaces the scene's shots by id and leaves `layout` alone.
//
//   node scripts/storyboard/figma-draft.mjs storyboards/catpope/s1c3 [--page "S1C3 · storyboard (test)"] [--out storyboards/_out]

import { mkdirSync, writeFileSync } from 'node:fs';
import { basename, resolve } from 'node:path';
import { ROOT, S, SLICE_H, W, textOf } from './lib.mjs';
import { shotSvg } from './schematic.mjs';
import { loadStoryboards, measureShot } from './storyboard.mjs';

const args = process.argv.slice(2);
const take = (flag, dflt) => {
	const i = args.indexOf(flag);
	return i >= 0 ? args.splice(i, 2)[1] : dflt;
};
const outRoot = resolve(ROOT, take('--out', 'storyboards/_out'));
const src = args[0];
if (!src) {
	console.error('usage: figma-draft.mjs <storyboard folder>');
	process.exit(2);
}
const name = basename(resolve(src));
const PAGE = take('--page', `${name.toUpperCase()} · storyboard (test)`);
const LIMIT = 45000;
const GAP = 160; // between scene frames

const list = loadStoryboards(src);
const out = resolve(outRoot, name, 'figma');
mkdirSync(out, { recursive: true });

// the shared prelude: page, scene frame and its four layers (created once, found again on re-runs)
const prelude = (scene, x, h) => `
const PAGE = ${JSON.stringify(PAGE)};
let page = figma.root.children.find((p) => p.name === PAGE);
if (!page) { page = figma.createPage(); page.name = PAGE; }
await figma.setCurrentPageAsync(page);
const FRAME = ${JSON.stringify(`${name} / scene-${scene} – draft`)};
let frame = page.children.find((n) => n.name === FRAME);
if (!frame) { frame = figma.createFrame(); frame.name = FRAME; frame.x = ${x}; frame.y = 0; frame.clipsContent = true; frame.fills = [{ type: 'SOLID', color: { r: 1, g: 1, b: 1 } }]; }
frame.resize(${W}, ${Math.max(1, Math.round(h))});
function layer(name, locked) {
  let f = frame.children.find((n) => n.name === name);
  if (!f) { f = figma.createFrame(); f.name = name; f.fills = []; f.clipsContent = false; frame.appendChild(f); }
  f.x = 0; f.y = 0; f.resize(${W}, ${Math.max(1, Math.round(h))}); f.locked = locked;
  return f;
}
const storyboard = layer('storyboard', true);
const guides = layer('guides', true);
const text = layer('text', false);
const layout = layer('layout', false);
// order bottom → top: storyboard, guides, text, layout
frame.appendChild(storyboard); frame.appendChild(guides); frame.appendChild(text); frame.appendChild(layout);
`;

const files = [];
let x = 0;
for (const { sb } of list) {
	const scene = sb.source?.scene;
	const shots = (sb.groups ?? []).flatMap((g) => g.shots ?? []);
	// stack: vertical / viewport at their height; a loop360 as one screen-high band (its panorama scaled to fit 390,
	// the full panorama placed beside the frame for reference)
	let y = 0;
	const placed = shots.map((shot) => {
		const { svg, w, h } = shotSvg(shot, sb, { noText: true });
		const pano = shot.format === 'loop360';
		const p = { shot, svg, w, h, y, pano, band: pano ? S : h };
		y += p.band;
		return p;
	});
	const H = y;
	// part 1: frame, guides, rail; then the shots in chunks
	const guideLines = [];
	for (let gy = S; gy < H; gy += S) guideLines.push({ y: gy, kind: 'screen' });
	for (let gy = SLICE_H; gy < H; gy += SLICE_H) guideLines.push({ y: gy, kind: 'slice' });
	const rail = placed.map((p) => {
		const m = measureShot(p.shot);
		return { y: p.y, label: `${p.shot.id}  ${m.seconds.toFixed(0)} s · ${m.words} w · ${p.shot.source?.want ?? ''}` };
	});
	const head = `${prelude(scene, x, H)}
// guides (recreated)
for (const n of [...guides.children]) n.remove();
for (const g of ${JSON.stringify(guideLines)}) {
  const r = figma.createRectangle(); r.name = g.kind + ' ' + g.y; r.x = 0; r.y = g.y; r.resize(${W}, 1);
  r.fills = [{ type: 'SOLID', color: g.kind === 'screen' ? { r: 0.9, g: 0.2, b: 0.2 } : { r: 0.2, g: 0.4, b: 0.9 }, opacity: 0.5 }];
  guides.appendChild(r);
}
guides.visible = false;
// the rhythm rail beside the frame (outside the 390, never exported)
await figma.loadFontAsync({ family: 'Inter', style: 'Regular' });
const RAIL = FRAME + ' · rail';
const oldRail = page.children.find((n) => n.name === RAIL); if (oldRail) oldRail.remove();
const railNodes = [];
for (const r of ${JSON.stringify(rail)}) {
  const t = figma.createText(); t.fontName = { family: 'Inter', style: 'Regular' }; t.fontSize = 11; t.characters = r.label;
  t.x = frame.x + ${W + 12}; t.y = r.y; page.appendChild(t); railNodes.push(t);
}
if (railNodes.length) { const g = figma.group(railNodes, page); g.name = RAIL; g.locked = true; }
`;
	const shotCode = (p) => {
		const dialogs = (p.shot.dialog ?? []).map((d) => ({
			id: d.id ?? `${p.shot.id}-t`, x: ((d.rect?.x ?? 5) / 100) * W, y: p.y + ((d.rect?.y ?? 5) / 100) * p.band,
			w: ((d.rect?.width ?? 60) / 100) * W, text: textOf(d.text), lettering: d.font === 'lettering', variant: d.variant ?? 'narration'
		}));
		return `
{ // ${p.shot.id}
  for (const n of [...storyboard.children, ...text.children]) if (n.name === ${JSON.stringify(p.shot.id)} || n.name.startsWith(${JSON.stringify(p.shot.id + ' ·')})) n.remove();
  const node = figma.createNodeFromSvg(${JSON.stringify(p.svg)});
  node.name = ${JSON.stringify(p.shot.id)};
  ${p.pano ? `node.rescale(${(W / p.w).toFixed(5)}); node.x = 0; node.y = ${p.y + Math.round((S - (p.h * W) / p.w) / 2)};` : `node.x = 0; node.y = ${p.y};`}
  storyboard.appendChild(node);
  for (const d of ${JSON.stringify(dialogs)}) {
    const t = figma.createText(); t.fontName = { family: 'Inter', style: d.lettering ? 'Bold' : 'Regular' };
    t.fontSize = d.lettering ? 28 : 13; t.characters = d.text; t.textAutoResize = 'HEIGHT'; t.resize(Math.max(40, d.w), t.height);
    t.x = d.x; t.y = d.y; t.name = ${JSON.stringify(p.shot.id)} + ' · ' + d.variant; text.appendChild(t);
  }
}`;
	};
	// chunk the shots
	const parts = [];
	let cur = head + "await figma.loadFontAsync({ family: 'Inter', style: 'Bold' });\n";
	for (const p of placed) {
		const code = shotCode(p);
		if (cur.length + code.length > LIMIT && cur !== head) {
			parts.push(cur);
			cur = prelude(scene, x, H) + "await figma.loadFontAsync({ family: 'Inter', style: 'Regular' });\nawait figma.loadFontAsync({ family: 'Inter', style: 'Bold' });\n";
		}
		cur += code;
	}
	parts.push(cur);
	parts.forEach((code, i) => {
		const body = `${code}\nreturn { frame: frame.id, page: page.id, shots: storyboard.children.length, texts: text.children.length };\n`;
		const file = resolve(out, `scene-${scene}.part${i + 1}.js`);
		writeFileSync(file, body);
		files.push({ file, chars: body.length, scene, shots: placed.length });
	});
	x += W + 220 + GAP;
}
for (const f of files) console.log(`${basename(f.file)}  ${f.chars} chars  (scene ${f.scene}, ${f.shots} shots)`);
const over = files.filter((f) => f.chars > 50000);
if (over.length) console.log(`\n${over.length} part(s) over 50,000 characters – a single shot's SVG is too large`);
console.log(`\npage: "${PAGE}" · run the parts in order through use_figma (or the bridge); nothing was sent.`);
