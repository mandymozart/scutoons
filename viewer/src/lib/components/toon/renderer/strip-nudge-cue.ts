import type { Attachment } from 'svelte/attachments';
import { bounceCue } from '@scutoons/reader';
import { hintGradient, hintStreak } from '../hint-streak.ts';
import { rootRem } from './strip-dom.ts';
import { NUDGE_BOUNCE_MS, NUDGE_STILL_MS, NUDGE_SWEEP_MS, NUDGE_VISIBLE, nudgeController, nudgeFrame, shownPart, type Nudge } from './strip-nudge.ts';

const SVG = 'http://www.w3.org/2000/svg';
/** the line's height in its own units (= 3rem, hint-areas.css): a gentle wave at the frame's middle */
const HIGH = 10;
/** the line right to left across `w` units (pathLength 1) */
const wave = (w: number) => `M ${w} 5 C ${0.8 * w} 3.6, ${0.64 * w} 6.4, ${0.5 * w} 5 S ${0.18 * w} 3.6, 0 5`;
/** the streak's length along the line */
const STREAK = 0.32;

/** the band's side, read when the nudge needs it */
export interface NudgeBand {
	/** the frames in DOM order (window > track > frames) */
	frames(): HTMLElement[];
	/** the band can move at all (not whole and still) */
	movable(): boolean;
	/** a drag, glide or ease moves the band */
	busy(): boolean;
}

/**
 * The hint line over `frame`'s part on screen, `[lo, hi]` px of it (hint-areas.css `.sc-hint-sweep`): the trails' look –
 * one stroke in their core → glow gradient and their streak (hint-streak.ts), their blend and glow – the streak running
 * once right to left across the whole frame, the way the picture comes in when it is dragged on. A uniform scale (the
 * viewBox has the part's aspect): the stroke and its round caps stay 0.36rem at any slice width. Reduced motion: the
 * whole line fades in and out where it stands (no travel; like the tap glow, a fade still shows "there is something").
 */
function sweepLine(frame: HTMLElement, [lo, hi]: [number, number], still: boolean): { line: SVGSVGElement; anims: Animation[] } {
	const rem = rootRem(frame.ownerDocument);
	const line = document.createElementNS(SVG, 'svg');
	line.classList.add('sc-hint-sweep');
	line.style.left = `${lo / rem}rem`;
	line.style.width = `${(hi - lo) / rem}rem`;
	const w = (HIGH * (hi - lo)) / (3 * rem);
	line.setAttribute('viewBox', `0 0 ${w} ${HIGH}`);
	line.setAttribute('aria-hidden', 'true');
	const path = document.createElementNS(SVG, 'path');
	path.setAttribute('d', wave(w));
	path.setAttribute('pathLength', '1');
	line.append(path);
	// the core in the middle, the glow towards the ends and the edges (the trails' gradient laid along the line)
	path.setAttribute('stroke', hintGradient(line, `translate(${w / 2} 5) scale(${w / 2} 0.6)`));
	frame.appendChild(line);
	const anims = still
		? [line.animate([{ opacity: 0 }, { opacity: 1, offset: 0.3 }, { opacity: 1, offset: 0.7 }, { opacity: 0 }], { duration: NUDGE_STILL_MS, fill: 'forwards' })]
		: [hintStreak(path, { length: STREAK, from: STREAK, to: -1, ms: NUDGE_SWEEP_MS, easing: 'cubic-bezier(.45,.05,.55,.95)', fade: [0.12, 0.85] })];
	return { line, anims };
}

/** sweep, then (not with reduced motion) the screen's bounce as the streak reaches the left; returns the cancel */
function playNudge(el: HTMLElement, band: NudgeBand, end: () => void): () => void {
	const frames = band.frames();
	const view = frames[0]?.parentElement?.parentElement;
	const win = view?.getBoundingClientRect();
	const boxes = win ? frames.map((f) => { const r = f.getBoundingClientRect(); return { l: r.left - win.left, w: r.width }; }) : [];
	const index = win ? nudgeFrame(boxes, win.width) : -1;
	if (!win || index < 0) return (end(), () => {});
	const still = matchMedia('(prefers-reduced-motion: reduce)').matches;
	const { line, anims } = sweepLine(frames[index], shownPart(boxes[index], win.width), still);
	const screen = el.closest<HTMLElement>('.chapter-view') ?? el.ownerDocument.querySelector('main') ?? el.ownerDocument.body;
	let unbounce: (() => void) | null = null;
	const timers = [
		// zoomed for the band's width: full-bleed, wider than the reading column on a desktop
		...(still ? [] : [setTimeout(() => (unbounce = bounceCue(screen, NUDGE_BOUNCE_MS, el.offsetWidth)), NUDGE_SWEEP_MS * 0.78)]),
		setTimeout(() => (line.remove(), end()), still ? NUDGE_STILL_MS : NUDGE_SWEEP_MS * 0.78 + NUDGE_BOUNCE_MS)
	];
	return () => {
		for (const t of timers) clearTimeout(t);
		for (const anim of anims) anim.cancel();
		unbounce?.();
		line.remove();
	};
}

/**
 * The endless band's drag hint (strip-nudge.ts says when): `const nudge = bandNudge(band)`, `{@attach nudge.attach}` on
 * the band; the band calls `nudge.touch()` when the reader moves it and `nudge.changed()` when it may have become movable
 * (a measure). Watches the band's share on screen, "Show hints" (`html.sc-show-hints`, +layout.svelte) and the story's
 * blocking (`html[data-vn-blocking]`); a pointer or finger going down on the band pokes it, and while a finger stays on
 * it (a vertical swipe scrolls the page – the browser takes the gesture, no pointer events follow, touch events do) the
 * band counts as busy.
 */
export function bandNudge(band: NudgeBand): { attach: Attachment<HTMLElement>; touch(): void; changed(): void } {
	let nudge: Nudge | null = null;
	const attach: Attachment<HTMLElement> = (el) => {
		if (typeof IntersectionObserver === 'undefined' || typeof el.animate !== 'function') return;
		const root = el.ownerDocument.documentElement;
		let fingers = 0;
		const own = nudgeController({
			hints: () => root.classList.contains('sc-show-hints'),
			blocked: () => root.dataset.vnBlocking !== undefined,
			movable: band.movable,
			busy: () => fingers > 0 || band.busy(),
			play: (end) => playNudge(el, band, end),
			setTimer: (fn, ms) => setTimeout(fn, ms),
			clearTimer: (handle) => clearTimeout(handle as ReturnType<typeof setTimeout>)
		});
		nudge = own;
		// the latest entry: several can queue up for one callback
		const seen = new IntersectionObserver(
			(entries) => {
				const entry = entries[entries.length - 1];
				own.view(entry.isIntersecting && entry.intersectionRatio >= NUDGE_VISIBLE - 0.01);
			},
			{ threshold: [0, NUDGE_VISIBLE - 0.01, 1] }
		);
		seen.observe(el);
		const watch = new MutationObserver(() => own.changed());
		watch.observe(root, { attributes: true, attributeFilter: ['class', 'data-vn-blocking'] });
		const poke = () => own.poke();
		const finger = (event: TouchEvent) => {
			fingers = event.touches.length;
			if (event.type === 'touchstart') own.poke();
		};
		const touches = ['touchstart', 'touchend', 'touchcancel'] as const;
		el.addEventListener('pointerdown', poke, true);
		for (const type of touches) el.addEventListener(type, finger, { capture: true, passive: true });
		return () => {
			seen.disconnect();
			watch.disconnect();
			el.removeEventListener('pointerdown', poke, true);
			for (const type of touches) el.removeEventListener(type, finger, { capture: true });
			own.destroy();
			if (nudge === own) nudge = null;
		};
	};
	return { attach, touch: () => nudge?.touch(), changed: () => nudge?.changed() };
}
