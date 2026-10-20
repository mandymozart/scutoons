// region – a bridge command module (#93; moved out of plugin/code.js 2026-10-20). Used by figma-production.mjs (the production slices).
// a region of a frame as PNGs at several scales, some of its layers hidden – the production slices (figma-production.mjs):
// { frame, x, y, w, h, hide: ['text', 'guides'], hideIds: ['2231:5'], scales: [1, 2, 3], partial?, layers? } →
// [{ scale, base64 }] (hide = the frame's direct children by name, hideIds = layers at any depth by id). Works on a
// clone inside a clipping frame far off the canvas of the CURRENT page (always loaded – the draft's page may not be),
// removed afterwards – the draft itself is never touched.
// partial: clone only what reaches into the region (always for long drafts – a whole clone per slice was failure D);
// the box takes the frame's own fills (over white). layers: with partial, the frame's direct children are layer
// frames (a storyboard draft: storyboard / guides / text / layout) – per layer a shell with the layer's look (fills,
// strokes, effects, opacity, blend mode, clipping) at the layer's place, holding clones of only ITS children that
// reach into the region, each at its own place in the layer – not the whole layer. A layer holding a mask is cloned
// whole (a mask alone would cover the rest).
// Plugin-sandbox syntax: no `?.`, no object spread.
const f = await figma.getNodeByIdAsync(args.frame);
if (!f || !('clone' in f)) throw new Error(`no frame ${args.frame}`);
const page = figma.currentPage;
const box = figma.createFrame();
box.name = lib.TEMP;
const boxId = box.id;
lib.setTemps(lib.temps().concat(boxId));
box.resize(args.w, args.h);
box.clipsContent = true;
// white under everything (a transparent PNG turns black in the WebP); partial: the frame's own fills on top – the
// whole clone below brings them itself
box.fills = [{ type: 'SOLID', color: { r: 1, g: 1, b: 1 } }].concat(args.partial && Array.isArray(f.fills) ? f.fills : []);
page.appendChild(box);
box.x = -200000; box.y = -200000;
// hideIds: layers hidden at any depth (lettering nested in groups) – the clone keeps the original's child order,
// so original and clone are walked side by side
const ids = new Set(args.hideIds ?? []);
const hideIn = (o, c) => {
	if (!ids.size || !('children' in o) || !('children' in c)) return;
	o.children.forEach((k, i) => { const ck = c.children[i]; if (!ck) return; if (ids.has(k.id)) ck.visible = false; else hideIn(k, ck); });
};
try {
	if (args.partial && 'children' in f) {
		// a huge hand-made draft: clone only the frame's direct children that reach into the region (their
		// own positions stay – a child's x / y are relative to the frame), not the whole frame each time
		const fb = f.absoluteBoundingBox;
		// by the drawn box on the page: a rotated layer reaches far from its x / y (blood vessels turned 146°)
		const reaches = (n, dx, dy) => {
			const b = n.absoluteBoundingBox;
			const nx = b && fb ? b.x - fb.x : dx + n.x, ny = b && fb ? b.y - fb.y : dy + n.y, nw = b ? b.width : n.width, nh = b ? b.height : n.height;
			return !(nx >= args.x + args.w || nx + nw <= args.x || ny >= args.y + args.h || ny + nh <= args.y);
		};
		for (const n of f.children) {
			if ((args.hide ?? []).includes(n.name) || n.visible === false || ids.has(n.id) || !reaches(n, 0, 0)) continue;
			// a layer frame (not a group, not one holding a mask): a shell in its place with its look, its children
			// that reach into the region inside, each with its own transform in the layer (rotation included)
			if (args.layers && n.type === 'FRAME' && !n.children.some((k) => k.isMask)) {
				const shell = figma.createFrame();
				box.appendChild(shell);
				shell.name = n.name;
				shell.fills = []; shell.clipsContent = false; // a new frame is white and clips – the layer's own look below
				lib.copyLook(n, shell);
				shell.resize(Math.max(0.01, n.width), Math.max(0.01, n.height));
				const t = n.relativeTransform;
				shell.relativeTransform = [[t[0][0], t[0][1], t[0][2] - args.x], [t[1][0], t[1][1], t[1][2] - args.y]];
				for (const k of n.children) {
					if (k.visible === false || ids.has(k.id) || !reaches(k, n.x, n.y)) continue;
					const c = k.clone();
					shell.appendChild(c);
					c.relativeTransform = k.relativeTransform;
					hideIn(k, c);
				}
				continue;
			}
			const c = n.clone();
			box.appendChild(c);
			c.x = n.x - args.x; c.y = n.y - args.y;
			hideIn(n, c);
		}
		const out = [];
		for (const s of args.scales ?? [1]) out.push({ scale: s, base64: figma.base64Encode(await box.exportAsync({ format: 'PNG', constraint: { type: 'SCALE', value: s } })) });
		return out;
	}
	const c = f.clone();
	box.appendChild(c);
	c.x = -args.x; c.y = -args.y;
	if (args.unclip && 'clipsContent' in c) c.clipsContent = false; // a panorama strip reaches past the frame's width
	if ('children' in c) for (const l of c.children) if ((args.hide ?? []).includes(l.name)) l.visible = false;
	hideIn(f, c);
	const out = [];
	for (const s of args.scales ?? [1]) out.push({ scale: s, base64: figma.base64Encode(await box.exportAsync({ format: 'PNG', constraint: { type: 'SCALE', value: s } })) });
	return out;
} finally {
	// never throws: an exception here would replace the export's result
	try { box.remove(); lib.setTemps(lib.temps().filter((id) => id !== boxId)); } catch (e) {} // still listed: the next sweep takes it
	lib.sweepPage(page);
}
