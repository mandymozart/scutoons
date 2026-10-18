<script lang="ts">
	import ColourPopover from './colour-popover.svelte';
	import FieldReset from './field-reset.svelte';
	import { uiLabels } from './ui-labels.ts';
	import { HEX, longHex, rememberColour } from './colour.ts';

	/**
	 * An inline colour field for property rows (agents/research/EDITOR-INPUTS.md): a swatch button and the hex text in one
	 * framed box. The swatch opens our own colour popover (owner 2026-10-18: "the native picker isn't app UI style" –
	 * colour-popover.svelte: palette, recent colours, saturation / brightness + hue, hex) in the top layer, so no panel clips
	 * it; Esc or a click outside closes it, the focus goes back to the swatch. Empty = unset (`undefined`): the swatch
	 * shows the placeholder colour, or – when the placeholder isn't a colour ("none") – a diagonal line; while a colour is
	 * set, the reset button (FieldReset) at the right end unsets it. Typed text is passed on only once it is a whole hex
	 * colour (`#fff`, `#00d1ff`) or empty – never the half-typed `#`, `#1a` … on the way (owner 2026-10-18: a half-typed
	 * collage outline colour made chapter 2 no longer fit its type); anything else is put back on blur. The popover
	 * writes `#rrggbb`.
	 */
	let {
		value,
		onchange,
		label,
		placeholder = '',
		pickerLabel,
		palette = [],
		disabled = false,
		id,
		testid
	}: {
		value: string | null | undefined;
		onchange: (value: string | undefined) => void;
		/** the accessible name and the tooltip */
		label: string;
		/** the default: a colour (#000000) or a word (none) */
		placeholder?: string;
		/** the swatch button's accessible name (default: the app's "pick a colour") */
		pickerLabel?: string;
		/** extra swatches shown first in the popover (e.g. a toon's palette) */
		palette?: { name: string; hex: string }[];
		disabled?: boolean;
		id?: string;
		testid?: string;
	} = $props();

	const t = uiLabels();
	const uid = $props.id();
	const popId = `colour-${uid}`;
	const current = $derived(typeof value === 'string' && value ? value : '');
	/** the colour the swatch shows (null = none) */
	const swatch = $derived(HEX.test(current) ? longHex(current) : !current && HEX.test(placeholder) ? longHex(placeholder) : null);

	function type(raw: string) {
		const v = raw.trim();
		if (v === '') onchange(undefined);
		else if (HEX.test(v)) onchange(v);
	}
	/** typing done (blur, Enter): the text shows the value again – a host may keep its value for an emptied field */
	function settle(input: HTMLInputElement) {
		if (input.value.trim() !== current) input.value = current;
	}

	// ── the popover ──
	let field: HTMLSpanElement | undefined = $state();
	let button: HTMLButtonElement | undefined = $state();
	let pop: HTMLDivElement | undefined = $state();
	let open = $state(false);
	let openedWith = '';

	/** under the field (above it when there is no room), inside the window; in rem of the root (DESIGN.md) */
	function place() {
		if (!field || !pop) return;
		const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
		const r = field.getBoundingClientRect();
		const gap = 0.25 * rem;
		let top = r.bottom + gap;
		if (top + pop.offsetHeight > innerHeight && r.top - pop.offsetHeight - gap > 0) top = r.top - pop.offsetHeight - gap;
		const left = Math.max(gap, Math.min(r.left, innerWidth - pop.offsetWidth - gap));
		pop.style.top = `${top / rem}rem`;
		pop.style.left = `${left / rem}rem`;
	}

	function ontoggle(e: Event) {
		if ((e as ToggleEvent).newState === 'open') {
			open = true;
			openedWith = current;
			place();
			// once the content is drawn: placed by its real height, the focus on the first swatch
			setTimeout(() => {
				place();
				pop?.querySelector<HTMLElement>('button, [tabindex]')?.focus();
			});
			return;
		}
		open = false;
		if (current && current !== openedWith) rememberColour(current);
		// back to the swatch (unless the click outside went to another control)
		if (!document.activeElement || document.activeElement === document.body || pop?.contains(document.activeElement)) button?.focus();
	}
</script>

<span bind:this={field} class="sc-color-field" class:is-unset={!current} class:has-reset={!!current && !disabled} title={label} data-testid={testid}>
	<input {id} type="text" spellcheck="false" autocomplete="off" aria-label={label} {disabled} value={current} {placeholder} oninput={(e) => type(e.currentTarget.value)} onchange={(e) => settle(e.currentTarget)} />
	<button
		bind:this={button}
		type="button"
		class="swatch-button"
		popovertarget={popId}
		aria-haspopup="dialog"
		aria-expanded={open}
		aria-label={pickerLabel ?? t('pickColour')}
		title={pickerLabel ?? t('pickColour')}
		{disabled}
		data-testid={testid ? `${testid}-swatch` : undefined}
	>
		<span class="swatch" class:is-none={!swatch} style:--swatch={swatch ?? 'transparent'}></span>
	</button>
	{#if current && !disabled}<FieldReset onreset={() => onchange(undefined)} testid={testid ? `${testid}-reset` : undefined} />{/if}
	<div bind:this={pop} id={popId} class="popover" popover="auto" role="dialog" tabindex="-1" aria-label={label} {ontoggle} onkeydown={(e) => e.key === 'Escape' && e.stopPropagation()} data-testid={testid ? `${testid}-popover` : undefined}>
		{#if open}<ColourPopover {value} fallback={HEX.test(placeholder) ? placeholder : null} {palette} onpick={(hex) => onchange(hex)} />{/if}
	</div>
</span>

<style>
	.sc-color-field {
		--field-reset-width: 1.5rem;
		position: relative;
		display: block;
		min-width: 0;
	}
	input[type='text'] {
		box-sizing: border-box;
		width: 100%;
		height: var(--control-height);
		margin: 0;
		padding: 0 var(--space-05) 0 2.25rem;
		border: var(--border-width-hairline) solid var(--border-muted);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		transition: border-color var(--duration-fast);
	}
	.has-reset input[type='text'] {
		padding-right: calc(var(--space-05) + var(--field-reset-width));
	}
	.sc-color-field:hover input[type='text']:not(:disabled),
	input[type='text']:focus {
		border-color: var(--ink);
	}
	/* the swatch button: the field's left part, the full control height; the swatch a square in it (2px ink frame, paper
	   colours stay visible) */
	.swatch-button {
		position: absolute;
		inset: 0 auto 0 0;
		display: flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: 2.25rem;
		margin: 0;
		padding: 0;
		border: var(--border-width-hairline) solid transparent;
		background: transparent;
		cursor: pointer;
	}
	.swatch-button:hover:not(:disabled) {
		border-color: var(--ink);
	}
	.swatch {
		box-sizing: border-box;
		width: 1.25rem;
		height: 1.25rem;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--swatch);
	}
	.is-unset .swatch {
		border-color: var(--border-muted);
	}
	.swatch.is-none {
		background: linear-gradient(to top right, transparent calc(50% - 0.0625rem), var(--text-muted) calc(50% - 0.0625rem), var(--text-muted) calc(50% + 0.0625rem), transparent calc(50% + 0.0625rem));
	}
	/* the top layer: placed by place(); the content draws its own frame */
	.popover {
		position: fixed;
		inset: auto;
		margin: 0;
		padding: 0;
		border: 0;
		background: transparent;
		overflow: visible;
	}
	@media (prefers-reduced-motion: reduce) {
		input[type='text'] {
			transition: none;
		}
	}
</style>
