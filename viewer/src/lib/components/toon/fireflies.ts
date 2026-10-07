import type { Attachment } from 'svelte/attachments';

/**
 * Fireflies over a tap / hover / sound area with "Show hints" on (owner 2026-10-08, after nerdmanship's "Miyazaki
 * tribute" pen): a warm core with a blurred glow, drifting along soft curves over the area – a little past its edges –
 * flickering now and then. One per small area, two or three on bigger ones. They only move while the area is on
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

		function count() {
			const r = area.getBoundingClientRect();
			const share = (r.width * r.height) / Math.max(1, window.innerWidth * window.innerHeight);
			return share > 0.25 ? 3 : share > 0.08 ? 2 : 1;
		}

		function make() {
			const fly = document.createElement('span');
			fly.className = 'sc-firefly';
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

		function wake() {
			if (!visible || !hintsOn()) return;
			if (!flies.length) for (let i = 0; i < count(); i++) make();
			for (const fly of flies) if (!running.has(fly)) wander(fly);
		}

		const io = new IntersectionObserver(([entry]) => {
			visible = entry.isIntersecting;
			if (visible) wake();
		});
		io.observe(area);
		// "Show hints" switched on later (account page, another tab): they start then
		const mo = new MutationObserver(wake);
		mo.observe(document.documentElement, { attributes: true, attributeFilter: ['class'] });

		return () => {
			stopped = true;
			io.disconnect();
			mo.disconnect();
			for (const anim of running.values()) anim.cancel();
			for (const fly of flies) fly.remove();
		};
	};
}
