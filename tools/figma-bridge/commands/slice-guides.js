// slice-guides – a bridge command module (#107): redraws a storyboard draft frame's slice lines and its rail, nothing
// else. The frame is found by its EXACT name on the page; only the children of its `guides` layer and the rail (the
// group "<frame> · rail" on the page) are replaced. The frame and its layers are never resized, re-stacked, shown,
// hidden or locked, and `storyboard`, `text` and `layout` are never touched – unlike the `draft` command (applyDraft),
// so this is the way to fix the lines of a frame the owner works in.
// { page, frame: '<exact name>', guides: [{ y, kind: screen | slice | end | band, label? }], rail: [{ y, label, shot? }],
//   shots?: [{ id, y }], force? } → { frame, drawn, guides, labels, rail, railX, moved, missing, outside, hidden }
// - y: from the frame's top (lines and rail). The lines and rails come from lib.mjs sliceLines – the cut's own layout.
// - shots: where the layout (and so the cut) expects each shot. A shot of the `storyboard` layer elsewhere → nothing is
//   drawn (`moved`, unless force): the lines would miss the real cut. A shot not in the layer is only listed (`missing`).
// - rail[].shot: the old rail's label of that shot is kept (it says where its picture came from), at its new place.
// - the rail stays at its x when there is one (the owner may have moved the frame and left the rail), else beside the frame.
// Plugin-sandbox syntax: no `?.`, no object spread.
//   node scripts/storyboard/figma-chapter.mjs storyboards/catpope/s1c3 --page "S1C3 - BARGAINING" --name "s1c3 – storyboard draft 2026-10-09" --guides-only
const page = await lib.nodeOrPage({ page: args.page });
const named = page.children.filter((n) => n.type === 'FRAME' && n.name === args.frame);
if (named.length !== 1) throw new Error(named.length ? `${named.length} frames named "${args.frame}" on "${page.name}" – rename the others` : `no frame "${args.frame}" on "${page.name}"`);
const frame = named[0];
const layerOf = (name) => frame.children.find((n) => n.name === name && n.type === 'FRAME');
const guides = layerOf('guides'), storyboard = layerOf('storyboard');
if (!guides || !storyboard) throw new Error(`"${args.frame}" is no storyboard draft frame (no guides / storyboard layer)`);

const at = {};
for (const n of storyboard.children) at[n.name] = Math.round(n.y);
const moved = [], missing = [];
for (const s of args.shots || []) {
	if (!(s.id in at)) missing.push(s.id);
	else if (at[s.id] !== s.y) moved.push(`${s.id} at ${at[s.id]}, the cut expects ${s.y}`);
}
if (moved.length && !args.force) return { frame: frame.id, drawn: false, moved, missing };

await figma.loadFontAsync({ family: 'Inter', style: 'Regular' });
const COLOUR = { screen: { r: 0.9, g: 0.2, b: 0.2 }, slice: { r: 0.2, g: 0.4, b: 0.9 }, end: { r: 0.95, g: 0.45, b: 0 }, band: { r: 0.1, g: 0.6, b: 0.3 } };
for (const n of guides.children.slice()) n.remove();
let lines = 0, labels = 0, outside = 0;
for (const g of args.guides || []) {
	if (g.y < 0 || g.y >= frame.height) { outside++; continue; }
	const colour = COLOUR[g.kind] || COLOUR.slice;
	const r = figma.createRectangle();
	guides.appendChild(r);
	r.name = g.label ? `${g.kind} ${g.y} · ${g.label}` : `${g.kind} ${g.y}`;
	r.x = 0; r.y = g.y; r.resize(frame.width, 1);
	r.fills = [{ type: 'SOLID', color: colour, opacity: g.kind === 'screen' ? 0.5 : 0.9 }];
	lines++;
	if (!g.label) continue;
	// the label right-aligned just above its line (an end line: at the bottom of the group it ends)
	const t = figma.createText();
	guides.appendChild(t);
	t.fontName = { family: 'Inter', style: 'Regular' }; t.fontSize = 10; t.characters = g.label;
	t.fills = [{ type: 'SOLID', color: colour }];
	t.name = `${g.kind} ${g.y} label`;
	t.x = Math.max(0, frame.width - t.width - 4); t.y = Math.max(0, g.y - t.height - 1);
	labels++;
}

const railName = `${frame.name} · rail`;
const old = page.children.filter((n) => n.name === railName);
let railX = frame.x + frame.width + 12;
const kept = {};
if (old.length) {
	railX = Math.round(old[0].x);
	for (const o of old) for (const t of o.children || []) if (t.type === 'TEXT') kept[t.characters.split(/\s+/)[0]] = t.characters;
	for (const o of old) o.remove();
}
const nodes = [];
for (const r of args.rail || []) {
	const t = figma.createText();
	page.appendChild(t);
	t.fontName = { family: 'Inter', style: 'Regular' }; t.fontSize = 11;
	t.characters = (r.shot && kept[r.shot]) || r.label;
	t.x = railX; t.y = frame.y + r.y;
	nodes.push(t);
}
if (nodes.length) { const grp = figma.group(nodes, page); grp.name = railName; grp.locked = true; }
return { frame: frame.id, drawn: true, guides: lines, labels, rail: nodes.length, railX, moved, missing, outside, hidden: guides.visible === false };
