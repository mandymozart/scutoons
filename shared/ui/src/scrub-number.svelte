<script lang="ts">
	import Icon from './icon.svelte';
	import FieldReset from './field-reset.svelte';
	import { uiLabels } from './ui-labels.ts';
	import { clamp, evaluate, roundToStep, stepFor } from './scrub.ts';
	import { ScrubGesture } from './scrub-gesture.svelte.ts';
	import { roundTo, type Units } from './units.ts';
	import { unitPrefs } from './unit-prefs.svelte.ts';

	/**
	 * A compact number field (agents/research/EDITOR-INPUTS.md). The whole field is the gesture (owner 2026-10-18,
	 * scrub-gesture.svelte.ts): a tap goes into the value to type; press and slide – or a short hold, then slide – scrubs,
	 * with a mouse or a finger. Shift ×10, Alt ×0.1. While typing the field is a plain input. Typed maths: `12+3`, `*2`,
	 * `+10`, `-5` … (scrub.ts). ↑ ↓ step (Shift / Alt), PageUp / PageDown ×10, Home / End = min / max, Enter commits, Esc
	 * reverts. Empty = unset (`undefined`; the default shows as placeholder) unless `required`; while a value is set, the
	 * reset button (FieldReset) at the right end sets it back to the default. `mixed`: several values – "Mixed".
	 * `units` (owner 2026-10-18: "%, px, pt, rem with a dropdown on the unit"): the unit is a dropdown in the field; the
	 * value, its default, typing, steps and scrubbing are in the chosen unit, the stored value (`value`, `onchange`,
	 * `min`, `max`) stays in its own – the choice is remembered per `units.kind` in this browser. Without `units` a plain
	 * `unit` label. One control height (DESIGN.md): the input is the framed box; the icon / tag, the unit and the reset sit
	 * over its padding.
	 */
	let {
		value,
		onchange,
		label,
		icon,
		tag,
		unit,
		units,
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
		/** the new value (stored unit); `undefined` = unset (emptied or reset) */
		onchange: (value: number | undefined) => void;
		/** the accessible name and the tooltip */
		label: string;
		/** a Feather icon (or icons-extra.ts) at the start */
		icon?: string;
		/** a short text at the start (X, Y, W, H) when no icon fits */
		tag?: string;
		/** shown after the number (%, °, em) – without `units` */
		unit?: string;
		/** the units to choose from (units.ts) – replaces `unit` and `step` */
		units?: Units;
		step?: number;
		/** the stored range */
		min?: number;
		max?: number;
		/** the default, shown when unset (stored unit; a number is shown in the chosen unit and scrubbed from) */
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

	// ── the unit: stored ⟷ shown ──
	const chosen = $derived(units ? (units.options.find((o) => o.id === unitPrefs.get(units.kind, units.fallback)) ?? units.options[0]) : null);
	const toShown = (v: number) => (chosen ? roundTo(chosen.toShown(v), chosen.decimals) : v);
	const shownStep = $derived(chosen ? chosen.step : step);
	const unitLabel = $derived(chosen ? chosen.label : unit);
	const numericPlaceholder = $derived(placeholder !== '' && Number.isFinite(Number(placeholder)));
	const shownPlaceholder = $derived(numericPlaceholder ? String(toShown(Number(placeholder))) : placeholder);
	const shownValue = $derived(typeof value === 'number' ? toShown(value) : undefined);
	const shown = $derived(mixed || shownValue === undefined ? '' : String(shownValue));
	const base = $derived(shownValue ?? (numericPlaceholder ? toShown(Number(placeholder)) : 0));
	const shownRange = $derived<[number | undefined, number | undefined]>([min === undefined ? undefined : toShown(min), max === undefined ? undefined : toShown(max)]);
	const resettable = $derived(!required && !disabled && !mixed && typeof value === 'number');
	/** the unit area's width: the longest label + (a dropdown's) chevron */
	const unitWidth = $derived(units ? `calc(${Math.max(...units.options.map((o) => o.label.length))}ch + 0.85em)` : unit ? `calc(${unit.length}ch + var(--space-05))` : null);

	/** a shown value out: converted to the stored unit, in the stored range */
	function emit(next: number) {
		const v = chosen ? clamp(roundTo(chosen.toStored(next), 4), min, max) : clamp(roundToStep(next, step / 10), min, max);
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
		const v = evaluate(text, shownValue);
		if (v !== null) emit(chosen ? v : roundToStep(v, step / 100));
	}

	function onkeydown(e: KeyboardEvent) {
		const keys: Record<string, number> = { ArrowUp: 1, ArrowDown: -1, PageUp: 10, PageDown: -10 };
		if (e.key in keys) {
			e.preventDefault();
			commit();
			emit(base + keys[e.key] * stepFor(shownStep, e));
		} else if (e.key === 'Home' && min !== undefined) {
			e.preventDefault();
			onchange(min);
		} else if (e.key === 'End' && max !== undefined) {
			e.preventDefault();
			onchange(max);
		} else if (e.key === 'Enter') {
			commit();
			input?.select();
		} else if (e.key === 'Escape' && draft !== null) {
			e.stopPropagation();
			draft = null;
			input?.select();
		}
	}

	const gesture = new ScrubGesture({
		disabled: () => disabled,
		base: () => base,
		step: () => shownStep,
		range: () => shownRange,
		emit,
		tap: () => {
			input?.focus();
			input?.select();
		}
	});
</script>

<svelte:window onkeydown={gesture.key} />

<span
	class="sc-scrub"
	class:has-unit={!!unitLabel}
	class:has-units={!!units}
	class:has-reset={resettable}
	class:is-scrubbing={gesture.scrubbing}
	class:is-armed={gesture.armed}
	class:is-disabled={disabled}
	title={label}
	style:--scrub-unit={unitWidth}
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
		aria-valuenow={shownValue}
		aria-valuemin={shownRange[0]}
		aria-valuemax={shownRange[1]}
		aria-valuetext={mixed ? mixedText : shownValue !== undefined ? `${shownValue}${unitLabel ?? ''}` : shownPlaceholder ? `${shownPlaceholder}${unitLabel ?? ''}` : undefined}
		{disabled}
		value={draft ?? shown}
		placeholder={mixed ? mixedText : shownPlaceholder}
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
	{#if units && chosen}
		<select class="unit unit-select" aria-label="{label} · {t('unit')}" title={t('unit')} {disabled} onchange={(e) => unitPrefs.set(units.kind, e.currentTarget.value)} data-testid={testid ? `${testid}-unit` : undefined}>
			{#each units.options as o (o.id)}<option value={o.id} selected={o.id === chosen.id}>{o.label}</option>{/each}
		</select>
	{:else if unit}
		<span class="unit" aria-hidden="true">{unit}</span>
	{/if}
	{#if !editing && !disabled}
		<!-- the gesture surface over the field (pointer only – the input is the keyboard's control and carries the name) -->
		<span class="surface" aria-hidden="true" onpointerdown={gesture.down} onpointermove={gesture.move} onpointerup={gesture.up} onpointercancel={gesture.cancel}></span>
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
	/* the unit as a dropdown: its label + a small chevron, muted until hovered / focused; above the gesture surface */
	.unit-select {
		z-index: 2;
		box-sizing: border-box;
		height: 100%;
		margin: 0;
		padding: 0 0.7em 0 0;
		appearance: none;
		border: 0;
		background: transparent
			url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%236b6b6b' stroke-width='3' stroke-linecap='square'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E")
			no-repeat right 0.1em center / 0.55em;
		color: var(--text-muted);
		font: inherit;
		cursor: pointer;
		pointer-events: auto;
	}
	.unit-select:is(:hover, :focus-visible) {
		color: var(--ink);
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
	.has-units .surface {
		right: var(--scrub-unit);
	}
	.has-units.has-reset .surface {
		right: calc(var(--scrub-unit) + var(--field-reset-width));
	}
	@media (prefers-reduced-motion: reduce) {
		input {
			transition: none;
		}
	}
</style>
