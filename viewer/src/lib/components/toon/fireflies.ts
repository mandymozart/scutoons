import type { Attachment } from 'svelte/attachments';
import { HIGHLIGHTERS, type Highlighter } from '@scutoons/content';

type Shapes = (typeof import('./alienshine-shapes.ts'))['ALIENSHINE_SHAPES'];
/** the alienshine shapes: loaded the first time a chapter uses them (a chunk of its own) */
let shapes: Shapes | null = null;
let loading: Promise<void> | null = null;
const loadShapes = () => (loading ??= import('./alienshine-shapes.ts').then((m) => void (shapes = m.ALIENSHINE_SHAPES)));
let gradients = 0;
const SVG = 'http://www.w3.org/2000/svg';

/**
 * Fireflies over a tap / hover / sound area with "Show hints" on (owner 2026-10-08, after nerdmanship's "Miyazaki
 * tribute" pen): a warm core with a blurred glow, drifting along soft curves over the area – a little past its edges –
 * flickering now and then. The bigger the area, the more (one per --sc-firefly-area rem², at most --sc-firefly-max).
 * The highlighter is the chapter's (`Chapter.highlight`, chosen in the creator – `data-highlight` on the chapter view):
 * `fireflies` (round glows) or `alienshine` (a random brush stroke / speck of the owner's attention.svg each, random size
 * and turn); --sc-hint-style on :root overrides it for trying things out in the inspector. They only move while the area is on
 * screen and the hints are on; look in hint-areas.css (`.sc-firefly`, --sc-firefly-core / --sc-firefly-glow).
 */
export function fireflies(): Attachment<HTMLElement> {
	return (area) => {
		if (typeof IntersectionObserver === 'undefined' || typeof area.animate !== 'function') return;
		const flies: HTMLElement[] = [];
		const running = new Map<HTMLElement, Animation>();
		let visible = false;
		let stopped = false;

		const hintsOn = () => document.documentElement.classList.contains('sc-show-hints');
		const valid = (v: string | undefined | null): v is Highlighter => !!v && (HIGHLIGHTERS as readonly string[]).includes(v);
		function kindOf(): Highlighter {
			const override = getComputedStyle(document.documentElement).getPropertyValue('--sc-hint-style').trim();
			if (valid(override)) return override;
			const chosen = area.closest<HTMLElement>('[data-highlight]')?.dataset.highlight;
			return valid(chosen) ? chosen : 'fireflies';
		}
		let kind: Highlighter = 'fireflies';
		// a point over the area, up to 10 % past its edges (in % of the area) – sideways never out of its slice, so the
		// page never gets wider
		function spot() {
			const r = area.getBoundingClientRect();
			const slice = area.closest('[data-slice-index]')?.getBoundingClientRect();
			const w = Math.max(1, r.width);
			const lo = slice ? Math.max(-10, ((slice.left - r.left) / w) * 100) : 0;
			const hi = slice ? Math.min(110, ((slice.right - r.left - 12) / w) * 100) : 100;
			return [lo + Math.random() * Math.max(0, hi - lo), -10 + Math.random() * 120];
		}

		/**
		 * how many: one per --sc-firefly-area of the area (rem², default 100), at least one, at most --sc-firefly-max
		 * (default 12) – read live, so changing them in the inspector re-counts at once
		 */
		function count() {
			const css = getComputedStyle(document.documentElement);
			const rem = parseFloat(css.fontSize) || 16;
			const per = Math.max(1, parseFloat(css.getPropertyValue('--sc-firefly-area')) || 100);
			const max = Math.max(1, Math.round(parseFloat(css.getPropertyValue('--sc-firefly-max')) || 12));
			const r = area.getBoundingClientRect();
			return Math.min(max, Math.max(1, Math.round((r.width * r.height) / (rem * rem) / per)));
		}

		function make() {
			const fly = kind === 'alienshine' && shapes ? alien(shapes) : document.createElement('span');
			fly.classList.add('sc-firefly');
			if (kind === 'alienshine') fly.classList.add('sc-alienshine');
			fly.setAttribute('aria-hidden', 'true');
			const [x, y] = spot();
			fly.style.left = `${x}%`;
			fly.style.top = `${y}%`;
			area.appendChild(fly);
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

		/** one random shape of attention.svg with its own gradient (core → glow), a random size and turn */
		function alien(list: Shapes): HTMLElement {
			const [d, x, y, w, h, transform, rule] = list[Math.floor(Math.random() * list.length)];
			const id = `sc-alienshine-${++gradients}`;
			const svg = document.createElementNS(SVG, 'svg');
			svg.setAttribute('viewBox', `${x} ${y} ${w} ${h}`);
			// rem: a speck about 0.5, a long stroke up to 2.5 (the square root keeps the specks visible), ±20 % at random
			const size = Math.min(2.5, Math.max(0.5, 0.55 * Math.sqrt(Math.max(w, h)))) * (0.8 + Math.random() * 0.4);
			svg.style.width = `${(size * w) / Math.max(w, h)}rem`;
			svg.style.height = `${(size * h) / Math.max(w, h)}rem`;
			svg.style.rotate = `${Math.round(Math.random() * 360)}deg`;
			const gradient = document.createElementNS(SVG, 'radialGradient');
			gradient.setAttribute('id', id);
			gradient.setAttribute('cx', '0');
			gradient.setAttribute('cy', '0');
			gradient.setAttribute('r', '1');
			gradient.setAttribute('gradientUnits', 'userSpaceOnUse');
			gradient.setAttribute('gradientTransform', transform);
			for (const [offset, color] of [['0', 'var(--sc-firefly-core)'], ['1', 'var(--sc-firefly-glow)']]) {
				const stop = document.createElementNS(SVG, 'stop');
				stop.setAttribute('offset', offset);
				stop.style.stopColor = color;
				gradient.appendChild(stop);
			}
			const path = document.createElementNS(SVG, 'path');
			path.setAttribute('d', d);
			path.setAttribute('fill', `url(#${id})`);
			if (rule) path.setAttribute('fill-rule', rule);
			svg.append(gradient, path);
			return svg as unknown as HTMLElement;
		}

		/** to a new spot along a curve (two bends on the way), then on again */
		function wander(fly: HTMLElement) {
			if (stopped || !visible || !hintsOn()) return void running.delete(fly);
			const from = [parseFloat(fly.style.left), parseFloat(fly.style.top)];
			const to = spot();
			const bend = () => [(Math.random() - 0.5) * 30, (Math.random() - 0.5) * 30];
			const [b1, b2] = [bend(), bend()];
			const at = (t: number, b: number[]) => [from[0] + (to[0] - from[0]) * t + b[0], from[1] + (to[1] - from[1]) * t + b[1]];
			const points = [from, at(0.33, b1), at(0.66, b2), to];
			const anim = fly.animate(
				points.map(([x, y]) => ({ left: `${x}%`, top: `${y}%` })),
				{ duration: 4000 + Math.random() * 3000, easing: 'ease-in-out', fill: 'forwards' }
			);
			running.set(fly, anim);
			anim.onfinish = () => {
				fly.style.left = `${to[0]}%`;
				fly.style.top = `${to[1]}%`;
				anim.cancel();
				wander(fly);
			};
		}

		function clear() {
			for (const anim of running.values()) anim.cancel();
			running.clear();
			for (const fly of flies.splice(0)) fly.remove();
		}

		function wake() {
			if (!visible || !hintsOn()) return;
			const next = kindOf();
			if (next === 'alienshine' && !shapes) return void loadShapes().then(wake);
			if (next !== kind) (clear(), (kind = next));
			const want = count();
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
		// "Show hints" switched on later (account page, another tab): they start then; the count variables changed on
		// :root in the inspector: re-counted
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
