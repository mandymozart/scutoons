// text-paints – the fill and the outline of text layers as SCUTOONS paints (issue #94: Dialog.color / stroke = a CSS
// colour or a gradient): { ids: ['3352:5654', …] } → [{ id, name, text, fill, stroke, strokeWeight }], a paint being
// '#rrggbb', { type: 'linear', angle, stops } or { type: 'radial', center, radius, stops } (stops { at 0–100, color,
// opacity? }); null when the layer has none. The topmost visible paint counts (Figma paints bottom → top).
// Figma's gradientTransform maps the layer's 0–1 box into gradient space (the line from (0, 0.5) to (1, 0.5), a radial's
// circle around (0.5, 0.5) with radius 0.5): its inverse brings those points back into the layer; a linear gradient
// becomes a CSS angle with its stops re-placed on the CSS gradient line, a radial one a centre in % and a radius in %
// of the farthest corner (CSS `circle farthest-corner`). Read-only. Plugin-sandbox syntax: no `?.`, no object spread.
//   node tools/figma-bridge/cli.mjs run text-paints --args '{"ids":["3352:5654"]}'

const hex2 = (v) => Math.round(Math.max(0, Math.min(1, v)) * 255).toString(16).padStart(2, '0');
const hex = (c) => '#' + hex2(c.r) + hex2(c.g) + hex2(c.b);
const round = (n) => Math.round(n * 100) / 100;

function invert(t) {
	const a = t[0][0], b = t[0][1], c = t[0][2], d = t[1][0], e = t[1][1], f = t[1][2];
	const det = a * e - b * d;
	return [[e / det, -b / det, (b * f - c * e) / det], [-d / det, a / det, (c * d - a * f) / det]];
}
const apply = (m, x, y) => ({ x: m[0][0] * x + m[0][1] * y + m[0][2], y: m[1][0] * x + m[1][1] * y + m[1][2] });

function stopOf(s, at, paintOpacity) {
	const o = (s.color.a === undefined ? 1 : s.color.a) * paintOpacity;
	const stop = { at: round(Math.max(0, Math.min(100, at))), color: hex(s.color) };
	if (o < 0.999) stop.opacity = round(o);
	return stop;
}

function paintOf(paints, w, h) {
	if (!Array.isArray(paints)) return null;
	const p = paints.slice().reverse().find((x) => x.visible !== false);
	if (!p) return null;
	const opacity = p.opacity === undefined ? 1 : p.opacity;
	if (p.type === 'SOLID') return hex(p.color);
	if (p.type !== 'GRADIENT_LINEAR' && p.type !== 'GRADIENT_RADIAL' && p.type !== 'GRADIENT_DIAMOND') return null;
	const m = invert(p.gradientTransform);
	const px = (q) => ({ x: q.x * w, y: q.y * h });
	if (p.type === 'GRADIENT_LINEAR') {
		const s = px(apply(m, 0, 0.5)), e = px(apply(m, 1, 0.5));
		const dx = e.x - s.x, dy = e.y - s.y;
		const rad = Math.atan2(dx, -dy); // CSS: 0 = up, clockwise
		const ux = Math.sin(rad), uy = -Math.cos(rad);
		const len = Math.abs(w * ux) + Math.abs(h * uy); // the CSS gradient line through the box centre
		const stops = p.gradientStops.map((g) => {
			const q = { x: s.x + dx * g.position, y: s.y + dy * g.position };
			return stopOf(g, (((q.x - w / 2) * ux + (q.y - h / 2) * uy) / len + 0.5) * 100, opacity);
		});
		stops.sort((a, b) => a.at - b.at);
		return { type: 'linear', angle: round(((rad * 180) / Math.PI + 360) % 360), stops };
	}
	// radial (a diamond drawn as a radial one): the circle's centre and its radius along the gradient's x axis
	const c = px(apply(m, 0.5, 0.5)), r = px(apply(m, 1, 0.5));
	const radius = Math.hypot(r.x - c.x, r.y - c.y);
	const far = Math.max(Math.hypot(c.x, c.y), Math.hypot(w - c.x, c.y), Math.hypot(c.x, h - c.y), Math.hypot(w - c.x, h - c.y)) || 1;
	const stops = p.gradientStops.map((g) => stopOf(g, g.position * 100, opacity));
	stops.sort((a, b) => a.at - b.at);
	return { type: 'radial', center: { x: round((c.x / w) * 100), y: round((c.y / h) * 100) }, radius: round((radius / far) * 100), stops };
}

const out = [];
for (const id of args.ids || []) {
	const n = await figma.getNodeByIdAsync(id);
	if (!n) {
		out.push({ id, missing: true });
		continue;
	}
	out.push({
		id,
		name: n.name,
		text: n.type === 'TEXT' ? n.characters : undefined,
		fill: 'fills' in n && n.fills !== figma.mixed ? paintOf(n.fills, n.width, n.height) : null,
		stroke: 'strokes' in n ? paintOf(n.strokes, n.width, n.height) : null,
		strokeWeight: typeof n.strokeWeight === 'number' ? n.strokeWeight : null
	});
}
return out;
