// The whole-field gesture of ScrubNumber (owner 2026-10-18): a tap types; press and slide – past a few px (more for a
// finger), or any slide after a 200 ms hold – scrubs; a mouse scrub takes the pointer lock; Esc during it puts the
// value from before back. Works in the field's shown unit: `base` / `step` / `emit` are the shown value's.
import { clamp, dragSteps, roundToStep, stepFor } from './scrub.ts';

export interface ScrubTarget {
	disabled: () => boolean;
	/** the shown value the scrub starts from */
	base: () => number;
	/** one step in the shown unit */
	step: () => number;
	/** the shown range */
	range: () => [number | undefined, number | undefined];
	emit: (shown: number) => void;
	/** a tap: go into the value */
	tap: () => void;
}

const HOLD_MS = 200;

export class ScrubGesture {
	scrubbing = $state(false);
	armed = $state(false);
	#t: ScrubTarget;
	#start = 0;
	#raw = 0;
	#moved = 0;
	#lastX = 0;
	#pressed = false;
	#hold = 0;

	constructor(target: ScrubTarget) {
		this.#t = target;
	}

	down = (e: PointerEvent) => {
		if (this.#t.disabled() || e.button !== 0) return;
		e.preventDefault();
		try {
			(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		} catch {
			/* a synthetic event – the moves still arrive on the surface */
		}
		this.#pressed = true;
		this.#start = this.#raw = this.#t.base();
		this.#moved = 0;
		this.#lastX = e.clientX;
		clearTimeout(this.#hold);
		this.#hold = window.setTimeout(() => (this.armed = this.#pressed), HOLD_MS);
	};

	move = (e: PointerEvent) => {
		if (!this.#pressed) return;
		const surface = e.currentTarget as HTMLElement;
		const dx = document.pointerLockElement === surface ? e.movementX : e.clientX - this.#lastX;
		this.#lastX = e.clientX;
		this.#moved += Math.abs(dx);
		if (!this.scrubbing) {
			// a slide past a few px (more for a finger), or any slide after the hold
			if (!(this.armed ? this.#moved >= 1 : this.#moved >= (e.pointerType === 'mouse' ? 3 : 6))) return;
			this.scrubbing = true;
			clearTimeout(this.#hold);
			// no screen edge while scrubbing with a mouse (Theatre.js, Blender's wrap); touch and pens keep the finger
			if (e.pointerType === 'mouse') Promise.resolve(surface.requestPointerLock?.()).catch(() => {});
		}
		const s = stepFor(this.#t.step(), e);
		const [min, max] = this.#t.range();
		this.#raw = clamp(this.#raw + dragSteps(dx) * s, min, max);
		this.#t.emit(roundToStep(Math.round(this.#raw / s) * s, s));
	};

	up = (e: PointerEvent) => this.#end(e, false);
	cancel = (e: PointerEvent) => this.#end(e, true);

	/** on the window: Esc during a scrub puts the value back */
	key = (e: KeyboardEvent) => {
		if (e.key !== 'Escape' || !this.scrubbing) return;
		e.preventDefault();
		this.#t.emit(this.#start);
		this.scrubbing = this.armed = this.#pressed = false;
		if (document.pointerLockElement) document.exitPointerLock();
	};

	#end(e: PointerEvent, cancel: boolean) {
		if (!this.#pressed) return;
		this.#pressed = false;
		this.armed = false;
		clearTimeout(this.#hold);
		const surface = e.currentTarget as HTMLElement;
		if (surface.hasPointerCapture?.(e.pointerId)) surface.releasePointerCapture(e.pointerId);
		if (document.pointerLockElement === surface) document.exitPointerLock();
		if (cancel && this.scrubbing) this.#t.emit(this.#start);
		if (!this.scrubbing && !cancel) this.#t.tap();
		this.scrubbing = false;
	}
}
