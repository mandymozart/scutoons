<script lang="ts">
	import Icon from './icon.svelte';
	import { clamp, dragSteps, evaluate, roundToStep, stepFor } from './scrub.ts';

	/**
	 * A compact number field (agents/research/EDITOR-INPUTS.md): the handle on the left (an icon or a short tag like "X")
	 * scrubs – drag left / right, Shift ×10, Alt ×0.1, a mouse drag takes the pointer lock, Esc during the drag puts the
	 * old value back; a click on the handle (or on the number) types. Typed maths: `12+3`, `*2`, `+10`, `-5` … (scrub.ts).
	 * ↑ ↓ step (Shift / Alt), PageUp / PageDown ×10, Home / End = min / max, Enter commits, Esc reverts.
	 * Empty = unset (`undefined`; the default shows as placeholder) unless `required`. `mixed`: several values – "Mixed".
	 * One control height (DESIGN.md): the input is the framed box, the handle and the unit sit over its padding.
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
		mixedLabel = 'Mixed',
		disabled = false,
		id,
		testid
	}: {
		value: number | null | undefined;
		/** the new value; `undefined` = unset (the field was emptied) */
		onchange: (value: number | undefined) => void;
		/** the accessible name and the tooltip */
		label: string;
		/** a Feather icon (or icons-extra.ts) as the handle */
		icon?: string;
		/** a short text as the handle (X, Y, W, H) when no icon fits */
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
		mixedLabel?: string;
		disabled?: boolean;
		id?: string;
		testid?: string;
	} = $props();

	const uid = $props.id();
	const inputId = $derived(id ?? `scrub-${uid}`);
	let input: HTMLInputElement | undefined = $state();
	/** what is typed while the field has focus (null = show the value) */
	let draft: string | null = $state(null);
	let scrubbing = $state(false);

	const shown = $derived(mixed || value === undefined || value === null ? '' : String(value));
	const fallback = $derived(placeholder !== '' && Number.isFinite(Number(placeholder)) ? Number(placeholder) : 0);
	const base = $derived(typeof value === 'number' ? value : fallback);

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

	// ── scrubbing on the handle ──────────────────────────────────────────────────────────────────────────────────
	let start = 0;
	let raw = 0;
	let moved = 0;
	let lastX = 0;
	let pressed = false;

	function onpointerdown(e: PointerEvent) {
		if (disabled || e.button !== 0) return;
		e.preventDefault();
		commit();
		const handle = e.currentTarget as HTMLElement;
		handle.setPointerCapture(e.pointerId);
		pressed = true;
		start = raw = base;
		moved = 0;
		lastX = e.clientX;
	}

	function onpointermove(e: PointerEvent) {
		const handle = e.currentTarget as HTMLElement;
		if (!pressed) return;
		const dx = document.pointerLockElement === handle ? e.movementX : e.clientX - lastX;
		lastX = e.clientX;
		moved += Math.abs(dx);
		if (!scrubbing) {
			if (moved < 3) return;
			scrubbing = true;
			// no screen edge while scrubbing with a mouse (Theatre.js, Blender's wrap); touch and pens keep the finger
			if (e.pointerType === 'mouse') Promise.resolve(handle.requestPointerLock?.()).catch(() => {});
		}
		raw = clamp(raw + dragSteps(dx) * stepFor(step, e), min, max);
		const snapped = roundToStep(Math.round(raw / stepFor(step, e)) * stepFor(step, e), stepFor(step, e));
		emit(snapped);
	}

	function end(e: PointerEvent, cancel = false) {
		if (!pressed) return;
		pressed = false;
		const handle = e.currentTarget as HTMLElement;
		if (handle.hasPointerCapture(e.pointerId)) handle.releasePointerCapture(e.pointerId);
		if (document.pointerLockElement === handle) document.exitPointerLock();
		if (cancel && scrubbing) emit(start);
		// a click without a drag types (Figma: the label focuses the field)
		if (!scrubbing && !cancel) {
			input?.focus();
			input?.select();
		}
		scrubbing = false;
	}

	function onhandlekey(e: KeyboardEvent) {
		if (e.key === 'Escape' && scrubbing) {
			e.preventDefault();
			emit(start);
			scrubbing = pressed = false;
			if (document.pointerLockElement) document.exitPointerLock();
		}
	}
</script>

<svelte:window onkeydown={onhandlekey} />

<span class="sc-scrub" class:has-unit={!!unit} class:is-scrubbing={scrubbing} class:is-disabled={disabled} title={label} data-testid={testid}>
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
		aria-valuetext={mixed ? mixedLabel : typeof value === 'number' ? `${value}${unit ?? ''}` : placeholder ? `${placeholder}${unit ?? ''}` : undefined}
		{disabled}
		value={draft ?? shown}
		placeholder={mixed ? mixedLabel : placeholder}
		oninput={(e) => (draft = e.currentTarget.value)}
		onfocus={(e) => e.currentTarget.select()}
		onblur={commit}
		{onkeydown}
	/>
	<!-- the handle: pointer only (the input is the keyboard's control and carries the name) -->
	<span
		class="handle"
		aria-hidden="true"
		{onpointerdown}
		{onpointermove}
		onpointerup={(e) => end(e)}
		onpointercancel={(e) => end(e, true)}
		ondblclick={() => input?.select()}
	>
		{#if icon}<Icon name={icon} size="1.1em" />{:else}{tag}{/if}
	</span>
	{#if unit}<span class="unit" aria-hidden="true">{unit}</span>{/if}
</span>

<style>
	.sc-scrub {
		--scrub-handle: 2rem;
		--scrub-unit: 1.75rem;
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
		padding-right: var(--scrub-unit);
	}
	.sc-scrub:hover input:not(:disabled),
	input:focus,
	.is-scrubbing input {
		border-color: var(--ink);
	}
	.handle {
		position: absolute;
		inset: 0 auto 0 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--scrub-handle);
		color: var(--text-muted);
		cursor: ew-resize;
		touch-action: none;
		user-select: none;
	}
	.sc-scrub:hover .handle,
	.is-scrubbing .handle {
		color: var(--ink);
	}
	.is-disabled .handle {
		cursor: not-allowed;
		color: var(--border-muted);
	}
	.unit {
		position: absolute;
		inset: 0 0 0 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		width: var(--scrub-unit);
		color: var(--text-muted);
		pointer-events: none;
	}
	@media (prefers-reduced-motion: reduce) {
		input {
			transition: none;
		}
	}
</style>
