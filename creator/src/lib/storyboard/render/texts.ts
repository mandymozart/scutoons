// The frame's 2D layer (#124, moved from tools/storyboard-render/core.js): where each figure landed (figureBoxes), the
// storyboard marks, the texts in the SCUTOONS text presets (drawDialog) and the reader's fonts.
import * as THREE from 'three';
import type { ShotSpec } from '@scutoons/content/storyboard';
import type { Figurine } from './figures.ts';
import { presetOf, presetStyle } from './text-presets.ts';

/** a figure's projected box (px) */
export interface FigureBox {
	index: number;
	box: { x0: number; y0: number; x1: number; y1: number };
}

/** the noise seed of a shot (its id's char codes, mod 97) */
export const seedOf = (id: string) => [...String(id)].reduce((a, c) => a + c.charCodeAt(0), 0) % 97;

/** where each figure landed on the image: its projected box in output px */
export function figureBoxes(figs: Figurine[], camera: THREE.Camera, W: number, H: number, K = 1): FigureBox[] {
	return figs.map((m, index) => {
		const b = new THREE.Box3().setFromObject(m.root, true);
		let x0 = Infinity,
			y0 = Infinity,
			x1 = -Infinity,
			y1 = -Infinity;
		for (const cx of [b.min.x, b.max.x])
			for (const cy of [b.min.y, b.max.y])
				for (const cz of [b.min.z, b.max.z]) {
					const p = new THREE.Vector3(cx, cy, cz).project(camera);
					const px = ((p.x + 1) / 2) * W,
						py = ((1 - p.y) / 2) * H;
					x0 = Math.min(x0, px);
					y0 = Math.min(y0, py);
					x1 = Math.max(x1, px);
					y1 = Math.max(y1, py);
				}
		return { index, box: { x0: x0 / K, y0: y0 / K, x1: x1 / K, y1: y1 / K } };
	});
}

/** the storyboard marks: id + format top left, air zones as a bar on the left edge */
export function drawMarks(g: CanvasRenderingContext2D, shot: ShotSpec, W: number, H: number, K = 1): void {
	g.font = `600 ${11 * K}px ui-monospace, Consolas, monospace`;
	const label = `${shot.id} · ${shot.format} ${shot.aspect}${shot.move ? ` · ${shot.move}` : ''}${shot.want ? ` [${shot.want}]` : ''}`;
	const tw = g.measureText(label).width;
	g.fillStyle = '#000';
	g.fillRect(0, 0, tw + 12 * K, 18 * K);
	g.fillStyle = '#fff';
	g.fillText(label, 6 * K, 13 * K);
	for (const [a, b] of shot.air ?? []) {
		const y0 = (a / 100) * H,
			y1 = (b / 100) * H;
		g.fillStyle = '#888';
		g.fillRect(K, y0 + 2 * K, 3 * K, Math.max(2 * K, y1 - y0 - 4 * K));
	}
}

/**
 * The texts in the SCUTOONS text presets (text-presets.ts, agents/DESIGN.md Part 2 – owner 2026-10-09: the first throw
 * follows the design dogma): Narration (paper box, 2px ink rule, italic IBM Plex Mono), Scene (the same in capitals),
 * Speech bubble (a paper box, 2px ink outline, a tail – towards the nearest figure's head unless the text sets
 * `tail`), Onomatopoeia (Bangers, white with an ink outline, −7°), Story (bare mono). Font size = % of the shot's width
 * like the reader's cqw; the box at the storyboard's rect; the real words. figures: their boxes in canvas px.
 */
export function drawDialog(g: CanvasRenderingContext2D, shot: Pick<ShotSpec, 'dialog'>, figures: FigureBox[], W: number, H: number, K = 1): void {
	const wrap = (text: string, maxW: number) => {
		const out: string[] = [];
		for (const para of String(text).split('\n')) {
			let line = '';
			for (const w of para.split(/\s+/)) {
				const t = line ? `${line} ${w}` : w;
				if (g.measureText(t).width > maxW && line) {
					out.push(line);
					line = w;
				} else line = t;
			}
			out.push(line);
		}
		return out;
	};
	const INK = '#000',
		PAPER = '#fff';
	for (const d of shot.dialog ?? []) {
		if (!d.text) continue;
		const preset = presetOf(d);
		const st = presetStyle(preset, d);
		const r = d.rect ?? { x: 5, y: 5, width: 60 };
		const x = (r.x / 100) * W,
			y = (r.y / 100) * H,
			w = ((r.width ?? 60) / 100) * W;
		const size = ((d.size ?? st.size) / 100) * W;
		const text = st.uppercase ? String(d.text).toUpperCase() : String(d.text);
		g.save();
		if (preset === 'onomatopoeia' || preset === 'title') {
			g.font = `${size}px Bangers, Impact, sans-serif`;
			const lines = wrap(text, w);
			const lh = size * 1.05;
			g.translate(x + w / 2, y + (lines.length * lh) / 2);
			g.rotate(((st.rotation ?? 0) * Math.PI) / 180);
			g.textAlign = 'center';
			g.textBaseline = 'middle';
			g.lineJoin = 'round';
			lines.forEach((l, i) => {
				const ly = (i - (lines.length - 1) / 2) * lh;
				g.lineWidth = size * (st.strokeWidth ?? 0) * 2;
				g.strokeStyle = INK;
				g.strokeText(l, 0, ly);
				g.fillStyle = st.color ?? PAPER;
				g.fillText(l, 0, ly);
			});
			g.restore();
			continue;
		}
		const italic = preset === 'narration' || preset === 'scene';
		g.font = `${italic ? 'italic ' : ''}400 ${size}px 'IBM Plex Mono', ui-monospace, monospace`;
		const pad = size * 0.5;
		const lines = wrap(text, w - pad * 2);
		const lh = size * 1.15;
		const th = Math.max(((r.height ?? 0) / 100) * H, lines.length * lh + pad * 2);
		const rule = 2 * K;
		g.lineWidth = rule;
		g.strokeStyle = INK;
		g.fillStyle = PAPER;
		if (preset === 'speech-bubble') {
			const cx = x + w / 2,
				cy = y + th / 2,
				rx = w / 2 + pad * 0.6,
				ry = th / 2 + pad * 0.8;
			// the tail: the text's `tail`, else towards the nearest figure's head
			let tx: number, ty: number;
			const heads = figures.map((f) => ({ x: (f.box.x0 + f.box.x1) / 2, y: f.box.y0 + (f.box.y1 - f.box.y0) * 0.12 }));
			if (d.tail && d.tail !== 'none') {
				ty = d.tail.startsWith('top') ? y - size * 1.8 : y + th + size * 1.8;
				tx = d.tail.endsWith('left') ? x + w * 0.22 : d.tail.endsWith('right') ? x + w * 0.78 : cx;
			} else if (heads.length) {
				const h = heads.sort((a, b) => Math.hypot(a.x - cx, a.y - cy) - Math.hypot(b.x - cx, b.y - cy))[0];
				const dx = h.x - cx,
					dy = h.y - cy,
					l = Math.hypot(dx, dy) || 1;
				const reach = Math.min(l * 0.7, Math.max(rx, ry) + size * 2.2);
				tx = cx + (dx / l) * reach;
				ty = cy + (dy / l) * reach;
			} else {
				tx = x + w * 0.22;
				ty = y + th + size * 1.8;
			}
			// a box with a tail, like the reader's bubble (owner 2026-10-10: "alle sprechblasen in rechteckige mit einem
			// pfeil", "oval is redundant") – the tail leaves the top or the bottom edge towards its target
			const bx0 = x - pad * 0.4,
				by0 = y - pad * 0.4,
				bw = w + pad * 0.8,
				bh = th + pad * 0.8;
			const down = ty >= cy;
			const edge = down ? by0 + bh : by0;
			const base = Math.min(Math.max(tx, bx0 + bw * 0.15 + size * 0.6), bx0 + bw * 0.85 - size * 0.6);
			const tipY = down ? Math.max(ty, edge + size * 1.2) : Math.min(ty, edge - size * 1.2);
			const tail = () => {
				g.beginPath();
				g.moveTo(base - size * 0.6, edge);
				g.lineTo(tx, tipY);
				g.lineTo(base + size * 0.6, edge);
			};
			g.beginPath();
			g.rect(bx0, by0, bw, bh);
			g.fill();
			g.stroke();
			tail();
			g.closePath();
			g.fill();
			tail();
			g.stroke();
			g.fillStyle = INK;
			g.textAlign = 'center';
			g.textBaseline = 'middle';
			lines.forEach((l, i) => g.fillText(l, cx, cy + (i - (lines.length - 1) / 2) * lh));
		} else {
			if (preset !== 'story') {
				g.fillRect(x, y, w, th);
				g.strokeRect(x + rule / 2, y + rule / 2, w - rule, th - rule);
			}
			g.fillStyle = INK;
			g.textAlign = 'left';
			g.textBaseline = 'middle';
			const top = preset === 'story' ? y + pad + lh / 2 : y + th / 2 - ((lines.length - 1) * lh) / 2;
			lines.forEach((l, i) => g.fillText(l, x + pad, top + i * lh));
		}
		g.restore();
	}
}

/** the reader's fonts (self-hosted in @scutoons/tokens) – a canvas draws text only with fonts already loaded */
export async function loadFonts(): Promise<void> {
	if (typeof document === 'undefined' || !document.fonts) return;
	// never wait forever (a page can stall on a font while a model loads): at most 3 s, then draw with what is there
	const all = Promise.all(["400 16px 'IBM Plex Mono'", "italic 400 16px 'IBM Plex Mono'", '16px Bangers'].map((f) => document.fonts.load(f).catch(() => null)));
	await Promise.race([all, new Promise((ok) => setTimeout(ok, 3000))]);
}
