import { buzz, playFxSound, type FxSound } from './sounds.ts';
import { frameOf, toScreen, type Shape } from './shape.ts';

/**
 * The reader's feedback cues (agents/FX.md): motion + sound + haptic, fired together. Motion is transforms / opacity /
 * filters only and never changes the layout. Whatever leaves its slice (an item flying to the bag, sparkles, a
 * whole-screen flash) is drawn in ONE fixed layer the size of the viewport (`fxLayer`): it may cross slices but never
 * the viewport – no scrollbars, no jumping page. The layer lives on `document.body`, outside `.slice` (its
 * `container-type` would make a fixed child slice-relative).
 */

const LAYER_ID = 'sc-fx-layer';

export function fxLayer(): HTMLElement {
	let layer = document.getElementById(LAYER_ID);
	if (!layer) {
		layer = document.createElement('div');
		layer.id = LAYER_ID;
		layer.setAttribute('aria-hidden', 'true');
		Object.assign(layer.style, { position: 'fixed', inset: '0', overflow: 'hidden', pointerEvents: 'none', zIndex: '1000' });
		document.body.appendChild(layer);
	}
	return layer;
}

const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
const imgOf = (el: HTMLElement) => (el instanceof HTMLImageElement ? el : el.querySelector('img')) ?? el;

/** the viewport's content may move (a screen shake) without a horizontal scrollbar while it does */
function clipViewport(ms: number) {
	const root = document.documentElement;
	const before = root.style.overflowX;
	root.style.overflowX = 'clip';
	setTimeout(() => (root.style.overflowX = before), ms + 50);
}

export interface Lift {
	/** taken: the item flies into `target` (the inventory button; off screen or none: the bottom right) */
	fly(target: Element | null, sound?: FxSound | null): Promise<void>;
	/** not taken: shake it, it stays */
	refuse(): void;
	/** nothing happened (a second tap while the first still asks): back down, quietly */
	drop(): void;
}

/**
 * A tapped pickup: it jumps up a little at once (the grant still asks the server), then `fly` or `refuse`.
 * `rotation` = the placement's (deg) – the element's own transform is kept.
 */
export function liftCue(el: HTMLElement, rotation = 0): Lift {
	const lift = -0.6 * rem();
	const anim = el.animate(
		[
			{ transform: `rotate(${rotation}deg) translateY(0) scale(1)` },
			{ transform: `rotate(${rotation}deg) translateY(${lift}px) scale(1.22)` }
		],
		{ duration: 140, easing: 'cubic-bezier(.2,.8,.3,1.2)', fill: 'forwards' }
	);
	playFxSound('pop');
	buzz(12);

	return {
		fly(target, sound = 'pling') {
			return new Promise((resolve) => {
				const f = frameOf(el, null, rotation);
				const src = (imgOf(el) as HTMLImageElement).currentSrc || (imgOf(el) as HTMLImageElement).src;
				const fly = document.createElement('img');
				fly.src = src;
				fly.alt = '';
				Object.assign(fly.style, { position: 'absolute', left: '0', top: '0', width: `${f.w}px`, height: `${f.h}px`, objectFit: 'contain', willChange: 'transform' });
				fxLayer().appendChild(fly);
				anim.cancel();
				el.style.visibility = 'hidden';

				const vw = window.innerWidth;
				const vh = window.innerHeight;
				const r = target?.getBoundingClientRect();
				const seen = !!r && r.width > 0 && r.bottom > 0 && r.top < vh && r.right > 0 && r.left < vw;
				const tx = seen ? r!.left + r!.width / 2 : vw - 2 * rem();
				const ty = seen ? r!.top + r!.height / 2 : vh - 2 * rem();
				const x0 = f.cx - f.w / 2;
				const y0 = f.cy - f.h / 2;
				const dx = tx - f.cx;
				const dy = ty - f.cy;
				const cx = dx * 0.25;
				const cy = Math.min(lift, dy) - 6 * rem();
				const frames: Keyframe[] = [];
				for (let i = 0; i <= 10; i++) {
					const t = i / 10;
					const u = 1 - t;
					const px = 2 * u * t * cx + t * t * dx;
					const py = u * u * lift + 2 * u * t * cy + t * t * dy;
					frames.push({
						transform: `translate(${x0 + px}px,${y0 + py}px) rotate(${rotation + t * 300}deg) scale(${1.22 - t})`,
						filter: `drop-shadow(0 0 ${0.6 - t * 0.3}em rgba(0,209,255,${1 - t * 0.3}))`,
						opacity: seen ? 1 : 1 - t * t
					});
				}
				playFxSound(sound);
				buzz(20);
				setTimeout(() => playFxSound('whoosh'), 120);
				fly.animate(frames, { duration: 560, easing: 'cubic-bezier(.3,.1,.4,1)', fill: 'forwards' }).onfinish = () => {
					fly.remove();
					if (seen && target instanceof HTMLElement) {
						target.animate([{ transform: 'scale(1)' }, { transform: 'scale(1.22)' }, { transform: 'scale(0.94)' }, { transform: 'scale(1)' }], { duration: 360, easing: 'ease-out' });
						playFxSound('tick');
					}
					resolve();
				};
			});
		},
		refuse() {
			anim.cancel();
			refuseCue(el, rotation);
		},
		drop() {
			anim.cancel();
		}
	};
}

/** a pickup the reader can't take: it shakes ("no"), glows red, a dull thunk – and stays */
export function refuseCue(el: HTMLElement, rotation = 0) {
	el.animate(
		[0, -9, 9, -7, 7, -4, 0].map((x) => ({ transform: `translateX(${x}px) rotate(${rotation}deg)` })),
		{ duration: 380, easing: 'ease-out' }
	);
	imgOf(el).animate([{ filter: 'drop-shadow(0 0 0.5em rgba(255,33,10,1))' }, { filter: 'drop-shadow(0 0 0 rgba(255,33,10,0))' }], { duration: 700 });
	playFxSound('thunk');
	buzz([30, 60, 30]);
}

const STAR =
	"<svg viewBox='0 0 24 24' width='100%' height='100%'><path d='M12 0 L14.5 9.5 L24 12 L14.5 14.5 L12 24 L9.5 14.5 L0 12 L9.5 9.5 Z' fill='#00d1ff' stroke='#000' stroke-width='1.5'/></svg>";

/** a few sparkles on the item's outline (its shape; without one: the edge of its box) */
export function sparkles(el: HTMLElement, rotation: number, shape: Shape | null, count = 7) {
	const layer = fxLayer();
	const f = frameOf(el, shape, rotation);
	const size = 0.75 * rem();
	for (let i = 0; i < count; i++) {
		let x: number;
		let y: number;
		if (shape?.edge.length) {
			const [sx, sy] = shape.edge[Math.floor(Math.random() * shape.edge.length)];
			[x, y] = toScreen(f, shape, sx, sy);
		} else {
			const t = Math.random() * 4;
			const unit = { w: 1, h: 1 };
			const side = Math.floor(t);
			const p = t - side;
			const [ux, uy] = side === 0 ? [p, 0] : side === 1 ? [1, p] : side === 2 ? [1 - p, 1] : [0, 1 - p];
			[x, y] = toScreen(f, unit, ux, uy);
		}
		const s = document.createElement('div');
		s.innerHTML = STAR;
		Object.assign(s.style, { position: 'absolute', left: `${x - size / 2}px`, top: `${y - size / 2}px`, width: `${size}px`, height: `${size}px` });
		layer.appendChild(s);
		const angle = Math.random() * Math.PI * 2;
		const d = rem() * (0.8 + Math.random());
		s.animate(
			[
				{ transform: 'scale(0) rotate(0deg)', opacity: 1 },
				{ transform: `translate(${Math.cos(angle) * d * 0.5}px,${Math.sin(angle) * d * 0.5}px) scale(1.1) rotate(45deg)`, opacity: 1, offset: 0.4 },
				{ transform: `translate(${Math.cos(angle) * d}px,${Math.sin(angle) * d}px) scale(0) rotate(90deg)`, opacity: 0 }
			],
			{ duration: 700 + Math.random() * 300, delay: i * 60, easing: 'ease-out', fill: 'backwards' }
		).onfinish = () => s.remove();
	}
}

/** "you can tap me": one wiggle with sparkles on its outline and a soft tick (the app decides when – once per visit) */
export function nudgeCue(el: HTMLElement, rotation: number, shape: Shape | null) {
	el.animate(
		[0, -12, 10, -7, 4, 0].map((r, i) => ({ transform: `rotate(${rotation + r}deg) scale(${i === 1 || i === 2 ? 1.08 : 1})` })),
		{ duration: 700, easing: 'ease-in-out' }
	);
	sparkles(el, rotation, shape);
	playFxSound('tick');
	setTimeout(() => playFxSound('tick'), 140);
}

export const IMPACTS = ['shake', 'flash', 'punch', 'vignette'] as const;
export type Impact = (typeof IMPACTS)[number];

const IMPACT_SOUND: Record<Impact, FxSound> = { shake: 'boom', flash: 'zap', punch: 'thud', vignette: 'hum' };
export const IMPACT_DEFAULT_MS: Record<Impact, number> = { shake: 450, flash: 420, punch: 380, vignette: 1600 };

function shake(el: HTMLElement, px: number, ms: number) {
	el.animate(
		Array.from({ length: 10 }, (_, i) => {
			const k = px * (1 - i / 10);
			return { transform: `translate(${(Math.random() - 0.5) * 2 * k}px,${(Math.random() - 0.5) * 2 * k}px)` };
		}).concat([{ transform: 'none' }]),
		{ duration: ms, easing: 'linear' }
	);
}

function cover(host: HTMLElement, background: string, keyframes: Keyframe[], ms: number) {
	const d = document.createElement('div');
	Object.assign(d.style, { position: 'absolute', inset: '0', pointerEvents: 'none', zIndex: '4', background });
	host.appendChild(d);
	d.animate(keyframes, { duration: ms, easing: 'ease-out' }).onfinish = () => d.remove();
}

/**
 * An impact (the `fx` event): `slice` = the slice it plays on (its own box: a flash / vignette over it, the slice
 * shakes or punches); null = the whole screen (`screen` moves, the flash / vignette cover the viewport).
 * `sound`: none = the impact's own, null = silent.
 */
export function impactCue(kind: Impact, options: { slice?: HTMLElement | null; screen?: HTMLElement | null; sound?: FxSound | null; duration?: number } = {}) {
	const ms = options.duration ?? IMPACT_DEFAULT_MS[kind];
	const slice = options.slice ?? null;
	const screen = options.screen ?? document.querySelector<HTMLElement>('main') ?? document.body;
	playFxSound(options.sound === undefined ? IMPACT_SOUND[kind] : options.sound);
	if (kind === 'shake') {
		buzz(80);
		clipViewport(ms);
		shake(slice ?? screen, (slice ? 0.55 : 0.75) * rem(), ms);
	} else if (kind === 'punch') {
		buzz(35);
		clipViewport(ms);
		(slice ?? screen).animate([{ transform: 'scale(1)' }, { transform: `scale(${slice ? 1.08 : 1.04})`, offset: 0.25 }, { transform: 'scale(1)' }], {
			duration: ms,
			easing: 'cubic-bezier(.2,.8,.3,1)'
		});
	} else if (kind === 'flash') {
		buzz(25);
		cover(slice ?? fxLayer(), '#ffffff', [{ opacity: 0.95 }, { opacity: 0 }], ms);
	} else if (kind === 'vignette') {
		cover(slice ?? fxLayer(), 'radial-gradient(ellipse at center, rgba(0,0,0,0) 35%, rgba(0,0,0,0.85) 100%)', [{ opacity: 0 }, { opacity: 1, offset: 0.3 }, { opacity: 1, offset: 0.7 }, { opacity: 0 }], ms);
	}
}
