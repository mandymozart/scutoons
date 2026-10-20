<script lang="ts">
	import PaintPopover from './paint-popover.svelte';
	import FieldReset from './field-reset.svelte';
	import { colourSampler, uiLabels } from './ui-labels.ts';
	import { HEX, longHex, rememberColour } from './colour.ts';
	import { isGradient, paintCss, type Paint } from './paint.ts';

	/**
	 * A paint field (issue #94): ColorField for a paint – a solid colour (typed as hex, as before) or a
	 * linear / radial gradient (paint.ts). The swatch shows the paint; with a gradient the text reads "Linear · 180°" /
	 * "Radial · 3 stops" and opens the popover. The popover (paint-popover.svelte) has Figma's switch Solid · Linear ·
	 * Radial, the stop bar and the gradient's controls; a solid paint is the colour popover as in ColorField.
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
		value: Paint | null | undefined;
		onchange: (value: Paint | undefined) => void;
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
	const sampler = colourSampler();
	const uid = $props.id();
	const popId = `colour-${uid}`;
	const gradient = $derived(isGradient(value) ? value : null);
	const current = $derived(typeof value === 'string' && value ? value : '');
	const gradientText = $derived(gradient ? (gradient.type === 'linear' ? `${t('paintLinearShort')} · ${Math.round(gradient.angle)}°` : t('paintRadialShort')) : '');
	/** the colour the swatch shows (null = none) */
	const swatch = $derived(gradient ? paintCss(gradient) : HEX.test(current) ? longHex(current) : !current && HEX.test(placeholder) ? longHex(placeholder) : null);

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

	/**
	 * The eyedropper (owner 2026-10-18: "pick the colour from the slice and other things on the page"): the browser's
	 * EyeDropper where it has one (Chromium desktop – any colour on the screen); else the app's picture sampler (the
	 * creator: a point of the slice picture). The popover steps aside first; Esc cancels either; a pick is written as
	 * `#rrggbb` and remembered as a recent colour.
	 */
	type Dropper = { open: () => Promise<{ sRGBHex: string }> };
	const native = typeof window !== 'undefined' && 'EyeDropper' in window;
	const eyedropper = $derived(native ? 'screen' : open && sampler() ? 'picture' : null);
	async function useEyedropper() {
		const kind = eyedropper;
		const fromPicture = sampler();
		pop?.hidePopover();
		let hex: string | null = null;
		try {
			if (kind === 'screen') hex = (await new (window as unknown as { EyeDropper: new () => Dropper }).EyeDropper().open()).sRGBHex;
			else if (fromPicture) hex = await fromPicture();
		} catch {
			hex = null; // Esc / cancelled
		}
		if (hex && HEX.test(hex)) {
			onchange(longHex(hex));
			rememberColour(hex);
		}
		button?.focus();
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

<span bind:this={field} class="sc-color-field sc-paint-field" class:is-unset={!current && !gradient} class:has-reset={(!!current || !!gradient) && !disabled} title={label} data-testid={testid}>
	{#if gradient}
		<!-- a gradient isn't typed: the text names it, a click opens the popover -->
		<input {id} type="text" readonly aria-label={label} {disabled} value={gradientText} onclick={() => pop?.showPopover()} />
	{:else}
		<input {id} type="text" spellcheck="false" autocomplete="off" aria-label={label} {disabled} value={current} {placeholder} oninput={(e) => type(e.currentTarget.value)} onchange={(e) => settle(e.currentTarget)} />
	{/if}
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
	{#if (current || gradient) && !disabled}<FieldReset onreset={() => onchange(undefined)} testid={testid ? `${testid}-reset` : undefined} />{/if}
	<div bind:this={pop} id={popId} class="popover" popover="auto" role="dialog" tabindex="-1" aria-label={label} {ontoggle} onkeydown={(e) => e.key === 'Escape' && e.stopPropagation()} data-testid={testid ? `${testid}-popover` : undefined}>
		{#if open}
			<PaintPopover
				{value}
				fallback={HEX.test(placeholder) ? placeholder : null}
				{palette}
				onpick={(paint) => onchange(paint)}
				oneyedropper={eyedropper ? useEyedropper : undefined}
				eyedropperLabel={eyedropper === 'picture' ? t('eyedropperPicture') : t('eyedropper')}
			/>
		{/if}
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
	input[readonly] {
		cursor: pointer;
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
