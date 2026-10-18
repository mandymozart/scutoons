<script lang="ts">
	import Icon from './icon.svelte';
	import FieldReset from './field-reset.svelte';
	import { uiLabels } from './ui-labels.ts';
	import { clamp, dragSteps, evaluate, roundToStep, stepFor } from './scrub.ts';

	/**
	 * A compact number field (agents/research/EDITOR-INPUTS.md). The whole field is the gesture (owner 2026-10-18): a tap
	 * (click) goes into the value to type; press and slide – past a few px, or after a short hold (200 ms) – scrubs left /
	 * right, with a mouse or a finger (a finger moving up / down still scrolls the panel). Shift ×10, Alt ×0.1; a mouse
	 * scrub takes the pointer lock; Esc during it puts the old value back. While typing the field is a plain input (text
	 * selection as usual). Typed maths: `12+3`, `*2`, `+10`, `-5` … (scrub.ts). ↑ ↓ step (Shift / Alt), PageUp / PageDown ×10,
	 * Home / End = min / max, Enter commits, Esc reverts. Empty = unset (`undefined`; the default shows as placeholder)
	 * unless `required`; while a value is set, the reset button (FieldReset) at the right end sets it back to the default.
	 * `mixed`: several values – "Mixed". One control height (DESIGN.md): the input is the framed box; the icon / tag, the
	 * unit and the reset sit over its padding.
	 */
	let {
		value,
		onchange,
		label,
		icon,
		tag,
		unit,
		step = 1,
		min,
		max,
		placeholder = '',
		required = false,
		mixed = false,
		mixedLabel,
		disabled = false,
		id,
		testid
	}: {
		value: number | null | undefined;
		/** the new value; `undefined` = unset (emptied or reset) */
		onchange: (value: number | undefined) => void;
		/** the accessible name and the tooltip */
		label: string;
		/** a Feather icon (or icons-extra.ts) at the start */
		icon?: string;
		/** a short text at the start (X, Y, W, H) when no icon fits */
		tag?: string;
		/** shown after the number (%, °, em) */
		unit?: string;
		step?: number;
		min?: number;
		max?: number;
		/** the default, shown when unset (a number scrubs from it) */
		placeholder?: string;
		required?: boolean;
		mixed?: boolean;
		/** default: the app's word (ui-labels.ts) */
		mixedLabel?: string;
		disabled?: boolean;
		id?: string;
		testid?: string;
	} = $props();

	const t = uiLabels();
	const uid = $props.id();
	const inputId = $derived(id ?? `scrub-${uid}`);
	const mixedText = $derived(mixedLabel ?? t('mixed'));
	let input: HTMLInputElement | undefined = $state();
	/** what is typed while the field has focus (null = show the value) */
	let draft: string | null = $state(null);
	let editing = $state(false);
	let scrubbing = $state(false);
	let armed = $state(false);

	const shown = $derived(mixed || value === undefined || value === null ? '' : String(value));
	const fallback = $derived(placeholder !== '' && Number.isFinite(Number(placeholder)) ? Number(placeholder) : 0);
	const base = $derived(typeof value === 'number' ? value : fallback);
	const resettable = $derived(!required && !disabled && !mixed && typeof value === 'number');

	function emit(next: number) {
		// to a tenth of the step (Alt scrubs that fine)
		const v = clamp(roundToStep(next, step / 10), min, max);
		if (v !== value) onchange(v);
	}

	function commit() {
		if (draft === null) return;
		const text = draft;
		draft = null;
		if (text.trim() === '') {
			if (!required && value !== undefined) onchange(undefined);
			return;
		}
		const v = evaluate(text, typeof value === 'number' ? value : undefined);
		if (v !== null) emit(roundToStep(v, step / 100));
	}

	function onkeydown(e: KeyboardEvent) {
		const keys: Record<string, number> = { ArrowUp: 1, ArrowDown: -1, PageUp: 10, PageDown: -10 };
		if (e.key in keys) {
			e.preventDefault();
			commit();
			emit(base + keys[e.key] * stepFor(step, e));
		} else if (e.key === 'Home' && min !== undefined) {
			e.preventDefault();
			emit(min);
		} else if (e.key === 'End' && max !== undefined) {
			e.preventDefault();
			emit(max);
		} else if (e.key === 'Enter') {
			commit();
			input?.select();
		} else if (e.key === 'Escape' && draft !== null) {
			e.stopPropagation();
			draft = null;
			input?.select();
		}
	}

	// ── the gesture on the whole field (not while typing) ────────────────────────────────────────────────────────
	const HOLD_MS = 200;
	let start = 0;
	let raw = 0;
	let moved = 0;
	let lastX = 0;
	let pressed = false;
	let hold = 0;

	function onpointerdown(e: PointerEvent) {
		if (disabled || e.button !== 0) return;
		e.preventDefault();
		const surface = e.currentTarget as HTMLElement;
		try {
			surface.setPointerCapture(e.pointerId);
		} catch {
			/* a synthetic event – the moves still arrive on the surface */
		}
		pressed = true;
		start = raw = base;
		moved = 0;
		lastX = e.clientX;
		clearTimeout(hold);
		hold = window.setTimeout(() => (armed = pressed), HOLD_MS);
	}

	function onpointermove(e: PointerEvent) {
		if (!pressed) return;
		const surface = e.currentTarget as HTMLElement;
		const dx = document.pointerLockElement === surface ? e.movementX : e.clientX - lastX;
		lastX = e.clientX;
		moved += Math.abs(dx);
		if (!scrubbing) {
			// a slide past a few px (more for a finger), or any slide after the hold
			if (!(armed ? moved >= 1 : moved >= (e.pointerType === 'mouse' ? 3 : 6))) return;
			scrubbing = true;
			clearTimeout(hold);
			// no screen edge while scrubbing with a mouse (Theatre.js, Blender's wrap); touch and pens keep the finger
			if (e.pointerType === 'mouse') Promise.resolve(surface.requestPointerLock?.()).catch(() => {});
		}
		raw = clamp(raw + dragSteps(dx) * stepFor(step, e), min, max);
		const s = stepFor(step, e);
		emit(roundToStep(Math.round(raw / s) * s, s));
	}

	function end(e: PointerEvent, cancel = false) {
		if (!pressed) return;
		pressed = false;
		armed = false;
		clearTimeout(hold);
		const surface = e.currentTarget as HTMLElement;
		if (surface.hasPointerCapture?.(e.pointerId)) surface.releasePointerCapture(e.pointerId);
		if (document.pointerLockElement === surface) document.exitPointerLock();
		if (cancel && scrubbing) emit(start);
		// a tap types (the value selected, ready to overwrite)
		if (!scrubbing && !cancel) {
			input?.focus();
			input?.select();
		}
		scrubbing = false;
	}

	function onwindowkey(e: KeyboardEvent) {
		if (e.key === 'Escape' && scrubbing) {
			e.preventDefault();
			emit(start);
			scrubbing = pressed = armed = false;
			if (document.pointerLockElement) document.exitPointerLock();
		}
	}
</script>

<svelte:window onkeydown={onwindowkey} />

<span
	class="sc-scrub"
	class:has-unit={!!unit}
	class:has-reset={resettable}
	class:is-scrubbing={scrubbing}
	class:is-armed={armed}
	class:is-disabled={disabled}
	title={label}
	style:--scrub-unit={unit ? `calc(${unit.length}ch + var(--space-05))` : null}
	data-testid={testid}
>
	<input
		bind:this={input}
		id={inputId}
		type="text"
		inputmode="decimal"
		autocomplete="off"
		spellcheck="false"
		role="spinbutton"
		aria-label={label}
		aria-valuenow={typeof value === 'number' ? value : undefined}
		aria-valuemin={min}
		aria-valuemax={max}
		aria-valuetext={mixed ? mixedText : typeof value === 'number' ? `${value}${unit ?? ''}` : placeholder ? `${placeholder}${unit ?? ''}` : undefined}
		{disabled}
		value={draft ?? shown}
		placeholder={mixed ? mixedText : placeholder}
		oninput={(e) => (draft = e.currentTarget.value)}
		onfocus={(e) => {
			editing = true;
			e.currentTarget.select();
		}}
		onblur={() => {
			editing = false;
			commit();
		}}
		{onkeydown}
	/>
	<span class="lead" aria-hidden="true">{#if icon}<Icon name={icon} size="1.1em" />{:else}{tag}{/if}</span>
	{#if unit}<span class="unit" aria-hidden="true">{unit}</span>{/if}
	{#if !editing && !disabled}
		<!-- the gesture surface over the field (pointer only – the input is the keyboard's control and carries the name) -->
		<span
			class="surface"
			aria-hidden="true"
			{onpointerdown}
			{onpointermove}
			onpointerup={(e) => end(e)}
			onpointercancel={(e) => end(e, true)}
		></span>
	{/if}
	{#if resettable}<FieldReset onreset={() => onchange(undefined)} testid={testid ? `${testid}-reset` : undefined} />{/if}
</span>

<style>
	.sc-scrub {
		--scrub-handle: 1.5rem;
		--field-reset-width: 1.5rem;
		position: relative;
		display: block;
		min-width: 0;
	}
	input {
		box-sizing: border-box;
		width: 100%;
		height: var(--control-height);
		margin: 0;
		padding: 0 var(--space-05) 0 var(--scrub-handle);
		border: var(--border-width-hairline) solid var(--border-muted);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		font-variant-numeric: tabular-nums;
		transition: border-color var(--duration-fast);
	}
	.has-unit input {
		/* the unit's own width (ch of the mono font, set from its length) */
		padding-right: var(--scrub-unit);
	}
	.has-reset input {
		padding-right: calc(var(--scrub-unit, 0rem) + var(--field-reset-width));
	}
	.sc-scrub:hover input:not(:disabled),
	input:focus,
	.is-scrubbing input {
		border-color: var(--ink);
	}
	/* held long enough: the next slide scrubs (the frame and the icon turn ink) */
	.is-armed input {
		border-color: var(--ink);
	}
	.lead {
		position: absolute;
		inset: 0 auto 0 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--scrub-handle);
		color: var(--text-muted);
		pointer-events: none;
		user-select: none;
	}
	.sc-scrub:hover .lead,
	.is-armed .lead,
	.is-scrubbing .lead {
		color: var(--ink);
	}
	.is-disabled .lead {
		color: var(--border-muted);
	}
	.unit {
		position: absolute;
		inset: 0 0 0 auto;
		display: flex;
		align-items: center;
		justify-content: flex-start;
		width: var(--scrub-unit);
		color: var(--text-muted);
		pointer-events: none;
	}
	.has-reset .unit {
		right: var(--field-reset-width);
	}
	/* tap = type, press + slide = scrub; a finger moving up / down scrolls the panel */
	.surface {
		position: absolute;
		inset: 0;
		z-index: 1;
		cursor: ew-resize;
		touch-action: pan-y;
		user-select: none;
	}
	.has-reset .surface {
		right: var(--field-reset-width);
	}
	@media (prefers-reduced-motion: reduce) {
		input {
			transition: none;
		}
	}
</style>
