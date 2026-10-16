import type { Attachment } from 'svelte/attachments';
import { HIGHLIGHTERS, type Highlighter } from '@scutoons/content';
import { hintGradient, hintStreak } from './hint-streak.ts';

type Shape = [string, number, number, number, number, string, string | null];
type Alien = typeof import('./alien-shapes.ts');
/** the shapes of the owner's attention.svg: loaded the first time a chapter uses them (a chunk of its own) */
let alien: Alien | null = null;
let loading: Promise<void> | null = null;
const loadAlien = () => (loading ??= import('./alien-shapes.ts').then((m) => void (alien = m)));
const SVG = 'http://www.w3.org/2000/svg';

/**
 * The "Show hints" highlighters over a tap / hover / sound area (owner 2026-10-08) – the chapter's choice
 * (`Chapter.highlight`, chosen in the creator → `data-highlight` on the chapter view; --sc-hint-style on :root overrides
 * it for trying things out in the inspector):
 * - `fireflies` (default): a warm core with a blurred glow (after nerdmanship's "Miyazaki tribute" pen), drifting along
 *   soft curves over the area and a little past it, flickering
 * - `alienflies`: the same flight with a random brush stroke of the owner's attention.svg each (random size and turn)
 * - `specks`: the same with that SVG's small specks
 * - `trails`: a streak of light running along one of its thin arcs, fading, then another one somewhere else
 * The bigger the area, the more (one per --sc-firefly-area rem², at most --sc-firefly-max; trails --sc-trail-density
 * times as many, at most --sc-trail-max – owner: "increase the density for the trails, they look nice"). They
 * only move while the area is on screen and the hints are on; sideways never out of their slice (the page never gets
 * wider). Look: hint-areas.css (`.sc-firefly`, --sc-firefly-core / --sc-firefly-glow, --sc-hint-blend).
 */
export function fireflies(): Attachment<HTMLElement> {
	return (area) => {
		if (typeof IntersectionObserver === 'undefined' || typeof area.animate !== 'function') return;
		const flies: Element[] = [];
		const running = new Map<Element, Animation>();
		const timers = new Set<ReturnType<typeof setTimeout>>();
		/** trails: the slots whose streak runs or waits for its next one */
		const trailing = new Set<number>();
		let visible = false;
		let stopped = false;
		let kind: Highlighter = 'fireflies';

		const hintsOn = () => document.documentElement.classList.contains('sc-show-hints');
		const live = () => !stopped && visible && hintsOn();
		const valid = (v: string | undefined | null): v is Highlighter => !!v && (HIGHLIGHTERS as readonly string[]).includes(v);
		function kindOf(): Highlighter {
			const override = getComputedStyle(document.documentElement).getPropertyValue('--sc-hint-style').trim();
			if (valid(override)) return override;
			const chosen = area.closest<HTMLElement>('[data-highlight]')?.dataset.highlight;
			return valid(chosen) ? chosen : 'fireflies';
		}

		/**
		 * a point over the area, up to 10 % past its edges (in % of the area) – sideways never out of its slice, its own
		 * extent (whichever way it is turned) included
		 */
		function spot(fly?: Element) {
			const r = area.getBoundingClientRect();
			const slice = area.closest('[data-slice-index]')?.getBoundingClientRect();
			const w = Math.max(1, r.width);
			const box = fly?.getBoundingClientRect();
			const own = box ? Math.max(12, Math.hypot(box.width, box.height)) : 12;
			const lo = slice ? Math.max(-10, ((slice.left - r.left + own / 2) / w) * 100) : 0;
			const hi = slice ? Math.min(110, ((slice.right - r.left - own) / w) * 100) : 100;
			return [lo + Math.random() * Math.max(0, hi - lo), -10 + Math.random() * 120];
		}
		const place = (fly: Element, [x, y]: number[]) => ((fly as HTMLElement).style.left = `${x}%`, ((fly as HTMLElement).style.top = `${y}%`));

		/**
		 * how many: one per --sc-firefly-area of the area (rem², default 100), at least one, at most --sc-firefly-max
		 * (default 12); trails: that times --sc-trail-density (default 2), at most --sc-trail-max (default 24) – read live,
		 * so changing them in the inspector re-counts at once
		 */
		function count() {
			const css = getComputedStyle(document.documentElement);
			const num = (name: string, fallback: number) => {
				const v = parseFloat(css.getPropertyValue(name));
				return Number.isFinite(v) && v > 0 ? v : fallback;
			};
			const rem = parseFloat(css.fontSize) || 16;
			const r = area.getBoundingClientRect();
			const base = (r.width * r.height) / (rem * rem) / num('--sc-firefly-area', 100);
			if (kind === 'trails') return Math.min(Math.round(num('--sc-trail-max', 24)), Math.max(1, Math.round(base * num('--sc-trail-density', 2))));
			return Math.min(Math.round(num('--sc-firefly-max', 12)), Math.max(1, Math.round(base)));
		}

		/** an SVG of one shape with its own radial gradient (core → glow, hint-streak.ts) – filled, or (an arc) stroked */
		function shapeSvg([d, x, y, w, h, transform, rule]: Shape, rem: number, stroked: boolean): SVGSVGElement {
			const svg = document.createElementNS(SVG, 'svg');
			// an arc may be flat (a straight line): give the box some height so it has a size at all
			const long = Math.max(w, h);
			const [bw, bh] = [Math.max(w, long * 0.02), Math.max(h, long * 0.02)];
			svg.setAttribute('viewBox', `${x - (bw - w) / 2} ${y - (bh - h) / 2} ${bw} ${bh}`);
			svg.style.width = `${(rem * bw) / Math.max(bw, bh)}rem`;
			svg.style.height = `${(rem * bh) / Math.max(bw, bh)}rem`;
			svg.style.rotate = `${Math.round(Math.random() * 360)}deg`;
			const paint = hintGradient(svg, transform);
			const path = document.createElementNS(SVG, 'path');
			path.setAttribute('d', d);
			if (stroked) {
				// a thin arc: the same line width at any size (its 0.5 unit stroke scaled down would vanish)
				for (const [k, v] of [['fill', 'none'], ['stroke', paint], ['stroke-width', '2'], ['stroke-linecap', 'round'], ['vector-effect', 'non-scaling-stroke'], ['pathLength', '1']]) path.setAttribute(k, v);
			} else {
				path.setAttribute('fill', paint);
				if (rule) path.setAttribute('fill-rule', rule);
			}
			svg.append(path);
			return svg;
		}

		const pick = (list: Shape[]) => list[Math.floor(Math.random() * list.length)];
		const sqrtSize = (long: number, k: number, lo: number, hi: number) => Math.min(hi, Math.max(lo, k * Math.sqrt(long))) * (0.8 + Math.random() * 0.4);

		function make(): Element {
			let fly: Element;
			if (kind === 'alienflies' && alien) {
				const s = pick(alien.ALIEN_FLIES as Shape[]);
				fly = shapeSvg(s, sqrtSize(Math.max(s[3], s[4]), 0.55, 0.8, 2.5), false);
			} else if (kind === 'specks' && alien) {
				const s = pick(alien.ALIEN_SPECKS as Shape[]);
				// owner 2026-10-08: "3 to 4 times bigger each" – about 1–3rem
				fly = shapeSvg(s, sqrtSize(Math.max(s[3], s[4]), 1.2, 1.05, 3.15), false);
			} else fly = document.createElement('span');
			fly.classList.add('sc-firefly');
			if (kind !== 'fireflies') fly.classList.add('sc-alien');
			fly.setAttribute('aria-hidden', 'true');
			area.appendChild(fly);
			place(fly, spot(fly));
			// the flicker: slow breathing with an odd brighter moment
			fly.animate(
				[
					{ opacity: 0.35, transform: 'scale(0.85)' },
					{ opacity: 1, transform: 'scale(1.1)', offset: 0.45 },
					{ opacity: 0.7, transform: 'scale(0.95)', offset: 0.6 },
					{ opacity: 1, transform: 'scale(1.15)', offset: 0.7 },
					{ opacity: 0.35, transform: 'scale(0.85)' }
				],
				{ duration: 1800 + Math.random() * 2200, iterations: Infinity, delay: -Math.random() * 3000 }
			);
			flies.push(fly);
			return fly;
		}

		/** to a new spot along a curve (two bends on the way), then on again */
		function wander(fly: Element) {
			if (!live()) return void running.delete(fly);
			const el = fly as HTMLElement;
			const from = [parseFloat(el.style.left), parseFloat(el.style.top)];
			const to = spot(fly);
			const bend = () => [(Math.random() - 0.5) * 30, (Math.random() - 0.5) * 30];
			const [b1, b2] = [bend(), bend()];
			const at = (t: number, b: number[]) => [from[0] + (to[0] - from[0]) * t + b[0], from[1] + (to[1] - from[1]) * t + b[1]];
			const anim = el.animate(
				[from, at(0.33, b1), at(0.66, b2), to].map(([x, y]) => ({ left: `${x}%`, top: `${y}%` })),
				{ duration: 4000 + Math.random() * 3000, easing: 'ease-in-out', fill: 'forwards' }
			);
			running.set(fly, anim);
			anim.onfinish = () => {
				place(fly, to);
				anim.cancel();
				wander(fly);
			};
		}

		/** trails: an arc somewhere over the area, a streak of light runs along it and fades – then the next one */
		function trail(slot: number) {
			if (!live() || !alien || kind !== 'trails' || slot >= count()) {
				trailing.delete(slot);
				flies[slot]?.remove();
				return;
			}
			trailing.add(slot);
			const s = pick(alien.ALIEN_TRAILS as Shape[]);
			const svg = shapeSvg(s, Math.min(8, Math.max(3, Math.max(s[3], s[4]) * 0.04)) * (0.8 + Math.random() * 0.4), true);
			svg.classList.add('sc-firefly', 'sc-alien', 'sc-trail');
			svg.setAttribute('aria-hidden', 'true');
			flies[slot]?.remove();
			flies[slot] = svg;
			area.appendChild(svg);
			place(svg, spot(svg));
			const path = svg.querySelector('path')!;
			const length = 0.25 + Math.random() * 0.2;
			const anim = hintStreak(path, { length, from: 1 + length, to: -length, ms: 1600 + Math.random() * 1400, easing: 'ease-in-out', fade: [0.2, 0.75] });
			running.set(svg, anim);
			anim.onfinish = () => {
				running.delete(svg);
				svg.remove();
				const t = setTimeout(() => (timers.delete(t), trail(slot)), 200 + Math.random() * 1300);
				timers.add(t);
			};
		}

		function clear() {
			for (const anim of running.values()) anim.cancel();
			running.clear();
			for (const t of timers) clearTimeout(t);
			timers.clear();
			for (const fly of flies.splice(0)) fly?.remove();
			trailing.clear();
		}

		function wake() {
			if (!live()) return;
			const next = kindOf();
			if (next !== 'fireflies' && !alien) return void loadAlien().then(wake);
			if (next !== kind) (clear(), (kind = next));
			const want = count();
			if (kind === 'trails') {
				// every slot without a streak (new, or paused off screen) starts one; slots beyond the count end themselves
				for (let slot = 0; slot < want; slot++) {
					if (trailing.has(slot)) continue;
					trailing.add(slot);
					const t = setTimeout(() => (timers.delete(t), trail(slot)), Math.random() * 1200);
					timers.add(t);
				}
				return;
			}
			while (flies.length < want) make();
			while (flies.length > want) {
				const fly = flies.pop()!;
				running.get(fly)?.cancel();
				running.delete(fly);
				fly.remove();
			}
			for (const fly of flies) if (!running.has(fly)) wander(fly);
		}

		const io = new IntersectionObserver(([entry]) => {
			visible = entry.isIntersecting;
			if (visible) wake();
		});
		io.observe(area);
		// "Show hints" switched on later (account page, another tab): they start then; the variables changed on :root in
		// the inspector: re-read
		const mo = new MutationObserver(wake);
		mo.observe(document.documentElement, { attributes: true, attributeFilter: ['class', 'style'] });

		return () => {
			stopped = true;
			io.disconnect();
			mo.disconnect();
			clear();
		};
	};
}
