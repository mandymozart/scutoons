// One shot → a schematic SVG at canvas size (390 wide, the shot's height): the layout only – air hatched, figures as
// silhouettes at their place and size, text boxes with their text, the projection as a guide, the scroll move as an
// arrow, a label. Used by render.mjs (the storyboard page) and figma-draft.mjs (the locked `storyboard` group).

import { S, W, esc, parseAspect, shotHeight, textOf } from './lib.mjs';

const TONE = { day: '#dfe6ea', night: '#2c3340', dusk: '#e8c9b0', dawn: '#e9d9c4', interior: '#d9d3c7', snow: '#eef2f5' };

function wrap(text, maxChars) {
	const out = [];
	let line = '';
	for (const w of String(text).split(/\s+/)) {
		if ((line + ' ' + w).trim().length > maxChars) { if (line) out.push(line); line = w; } else line = (line + ' ' + w).trim();
	}
	if (line) out.push(line);
	return out;
}

function projectionGuide(p, w, h) {
	const c = 'stroke="#0003" fill="none" stroke-width="1"';
	if (p === 'polar') {
		const cx = w / 2, cy = h * 0.62;
		return [0.15, 0.3, 0.45, 0.62, 0.8].map((r) => `<circle cx="${cx}" cy="${cy}" r="${(r * Math.max(w, h * 0.6)).toFixed(0)}" ${c}/>`).join('');
	}
	if (p === 'top-down') {
		const lines = [];
		for (let x = 0; x <= w; x += 65) lines.push(`<line x1="${x}" y1="0" x2="${x}" y2="${h}" ${c}/>`);
		for (let y = 0; y <= h; y += 65) lines.push(`<line x1="0" y1="${y}" x2="${w}" y2="${y}" ${c}/>`);
		return lines.join('');
	}
	if (p === 'isometric') {
		const lines = [];
		for (let k = -h; k < w + h; k += 60) {
			lines.push(`<line x1="${k}" y1="0" x2="${k + h * 1.73}" y2="${h}" ${c}/>`, `<line x1="${k}" y1="0" x2="${k - h * 1.73}" y2="${h}" ${c}/>`);
		}
		return lines.join('');
	}
	// perspective: a horizon at a third
	return `<line x1="0" y1="${(h * 0.38).toFixed(0)}" x2="${w}" y2="${(h * 0.38).toFixed(0)}" ${c} stroke-dasharray="6 6"/>`;
}

function figure(f, w, h, label) {
	const fh = (f.h ?? 0.3) * S;
	const fw = fh * (f.framing === 'cu' || f.framing === 'ecu' ? 0.9 : 0.36);
	const x = (f.x ?? 0.5) * w - fw / 2;
	const y = (f.y ?? 0.5) * h - fh / 2;
	const head = Math.min(fw * 0.55, fh * 0.22);
	return `<g opacity="0.85"><rect x="${x.toFixed(0)}" y="${(y + head * 0.9).toFixed(0)}" width="${fw.toFixed(0)}" height="${(fh - head * 0.9).toFixed(0)}" rx="${(fw * 0.3).toFixed(0)}" fill="#1d1d1b"/>`
		+ `<circle cx="${(x + fw / 2).toFixed(0)}" cy="${(y + head / 2).toFixed(0)}" r="${(head / 2).toFixed(0)}" fill="#1d1d1b"/>`
		+ `<text x="${(x + fw / 2).toFixed(0)}" y="${(y + fh + 14).toFixed(0)}" text-anchor="middle" font-size="11" font-family="monospace" fill="#1d1d1b">${esc(label)}${f.mood ? ` · ${esc(f.mood)}` : ''}</text></g>`;
}

function dialogBox(d, w, h) {
	const r = d.rect ?? { x: 5, y: 5, width: 60 };
	const bw = ((r.width ?? 60) / 100) * w;
	const lettering = d.font === 'lettering';
	const fs = lettering ? 26 : 12;
	const lines = wrap(textOf(d.text), Math.max(8, Math.floor(bw / (fs * 0.62))));
	const bh = lines.length * fs * 1.25 + 12;
	const x = (r.x / 100) * w, y = (r.y / 100) * h;
	const box = lettering ? '' : `<rect x="${x.toFixed(0)}" y="${y.toFixed(0)}" width="${bw.toFixed(0)}" height="${bh.toFixed(0)}" fill="#fff" stroke="#1d1d1b" stroke-width="1"${d.variant === 'bubble' ? ' rx="14"' : ''}/>`;
	const text = lines.map((l, i) => `<tspan x="${(x + 6).toFixed(0)}" dy="${i ? (fs * 1.25).toFixed(1) : fs}">${esc(l)}</tspan>`).join('');
	return `<g>${box}<text y="${(y + 4).toFixed(0)}" font-size="${fs}" font-family="${lettering ? 'Impact, sans-serif' : 'monospace'}" font-weight="${lettering ? 700 : 400}" fill="#1d1d1b">${text}</text></g>`;
}

const ARROW = { 'tilt-down': '↓', fall: '↓', descend: '↓', reveal: '↓', travel: '↓', 'tilt-up': '↑', climb: '↑', hold: '■', 'look-around': '↔', pass: '→' };

/** → { svg, w, h } – the shot at canvas size (a loop360 at its full panorama width) */
export function shotSvg(shot, sb, opts = {}) {
	const a = parseAspect(shot.aspect) ?? { w: 9, h: 16 };
	const pano = shot.format === 'loop360';
	const h = pano ? S : shotHeight(shot.aspect, shot.format);
	const w = pano ? Math.round((S * a.w) / a.h) : W;
	const loc = (sb.locations ?? []).find((l) => l.ref === shot.location);
	const tone = TONE[loc?.time] ?? TONE.day;
	const cast = Object.fromEntries((sb.cast ?? []).map((c) => [c.ref, c.ref.toUpperCase()]));
	const air = (shot.air ?? []).map(([f, t]) => `<rect x="0" y="${((f / 100) * h).toFixed(0)}" width="${w}" height="${(((t - f) / 100) * h).toFixed(0)}" fill="url(#air)"/>`).join('');
	const seams = pano ? Array.from({ length: Math.floor(w / W) }, (_, i) => `<line x1="${(i + 1) * W}" y1="0" x2="${(i + 1) * W}" y2="${h}" stroke="#0002" stroke-dasharray="3 5"/>`).join('') : '';
	const label = `${shot.id} · ${shot.format} ${shot.aspect} · ${shot.projection ?? 'perspective'} · ${shot.move ?? ''}${shot.shot ? ` · ${shot.shot.size ?? ''} ${shot.shot.angle ?? ''}` : ''}`;
	const action = wrap(textOf(shot.action), 52).slice(0, 4);
	const want = shot.source?.want ? ` [${shot.source.want}]` : '';
	const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="${w}" height="${h}" viewBox="0 0 ${w} ${h}">
<defs><pattern id="air" width="10" height="10" patternUnits="userSpaceOnUse" patternTransform="rotate(45)"><rect width="10" height="10" fill="#fff8"/><line x1="0" y1="0" x2="0" y2="10" stroke="#0001" stroke-width="4"/></pattern></defs>
<rect width="${w}" height="${h}" fill="${tone}"/>${projectionGuide(shot.projection, w, h)}${air}${seams}
${(shot.figures ?? []).map((f) => figure(f, w, h, cast[f.cast] ?? f.cast)).join('')}
${opts.noText ? '' : (shot.dialog ?? []).map((d) => dialogBox(d, w, h)).join('')}
<g font-family="monospace" fill="#1d1d1b"><rect x="0" y="0" width="${w}" height="${18 + action.length * 13}" fill="#fffc"/>
<text x="6" y="13" font-size="11" font-weight="700">${esc(label)}${esc(want)}</text>
${action.map((l, i) => `<text x="6" y="${27 + i * 13}" font-size="10.5">${esc(l)}</text>`).join('')}</g>
<text x="${w - 8}" y="${(h / 2).toFixed(0)}" font-size="34" text-anchor="end" fill="#1d1d1b55">${ARROW[shot.move] ?? ''}</text>
</svg>`;
	return { svg, w, h };
}
