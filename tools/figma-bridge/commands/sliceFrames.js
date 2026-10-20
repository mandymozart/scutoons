// sliceFrames – a bridge command module (#93; moved out of plugin/code.js 2026-10-20). Used by the storyboard scripts.
// production slice frames from a draft – the owner's way ("ich ziehe immer die ebenen auf einen slice die auf der
// selben hoehe im slice active werden … dannach loesche ich die textebenen"): per slice a frame (name, w × h,
// clipping) at { px, py } on the page, holding copies of every node of the draft's layers (minus `skip`) that reaches
// into the slice, placed as in the draft, unlocked (owner: locked layers must be unlocked before copying); hidden
// nodes stay hidden. { frame, skip: ['text', 'guides'], slices: [{ name, x, y, w, h, px, py }] } – frames of the same
// name are replaced.
// Plugin-sandbox syntax: no `?.`, no object spread.
const f = await figma.getNodeByIdAsync(args.frame);
if (!f || !('children' in f)) throw new Error(`no frame ${args.frame}`);
const page = f.parent;
const nodes = [];
for (const layer of f.children) {
	if ((args.skip ?? []).includes(layer.name)) continue;
	const kids = 'children' in layer && layer.type !== 'INSTANCE' ? layer.children : [layer];
	for (const n of kids) nodes.push({ n, x: (layer === n ? 0 : layer.x) + n.x, y: (layer === n ? 0 : layer.y) + n.y, w: n.width, h: n.height });
}
const out = [];
for (const s of args.slices ?? []) {
	for (const old of page.children.filter((c) => c.name === s.name)) old.remove();
	const fr = figma.createFrame();
	fr.name = s.name;
	fr.resize(s.w, s.h);
	fr.clipsContent = true;
	fr.fills = [{ type: 'SOLID', color: { r: 1, g: 1, b: 1 } }];
	page.appendChild(fr);
	fr.x = s.px; fr.y = s.py;
	let count = 0;
	for (const it of nodes) {
		if (it.x >= s.x + s.w || it.x + it.w <= s.x || it.y >= s.y + s.h || it.y + it.h <= s.y) continue;
		const c = it.n.clone();
		fr.appendChild(c);
		c.x = it.x - s.x; c.y = it.y - s.y;
		if ('locked' in c) c.locked = false;
		count++;
	}
	out.push({ name: s.name, id: fr.id, layers: count });
}
return out;
