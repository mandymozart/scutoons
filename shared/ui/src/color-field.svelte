<script lang="ts">
	/**
	 * An inline colour field for property rows (agents/research/EDITOR-INPUTS.md): a swatch (the native colour picker lies
	 * over it) and the hex text in one framed box. Empty = unset (`undefined`): the swatch shows the placeholder colour,
	 * or – when the placeholder isn't a colour ("none") – a diagonal line. Typed values are passed on as they are
	 * (`#fff`, `#00d1ff`), the picker writes `#rrggbb`.
	 */
	let {
		value,
		onchange,
		label,
		placeholder = '',
		pickerLabel,
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
		/** the picker's accessible name (default: label) */
		pickerLabel?: string;
		disabled?: boolean;
		id?: string;
		testid?: string;
	} = $props();

	const HEX = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i;
	const long = (c: string) => (c.length === 4 ? `#${c[1]}${c[1]}${c[2]}${c[2]}${c[3]}${c[3]}` : c).toLowerCase();
	const current = $derived(typeof value === 'string' && value ? value : '');
	/** the colour the swatch shows (null = none) */
	const swatch = $derived(HEX.test(current) ? long(current) : !current && HEX.test(placeholder) ? long(placeholder) : null);

	function type(raw: string) {
		const v = raw.trim();
		onchange(v === '' ? undefined : v);
	}
</script>

<span class="sc-color-field" class:is-unset={!current} title={label} data-testid={testid}>
	<input {id} type="text" spellcheck="false" autocomplete="off" aria-label={label} {disabled} value={current} {placeholder} oninput={(e) => type(e.currentTarget.value)} />
	<span class="swatch" class:is-none={!swatch} style:--swatch={swatch ?? 'transparent'}>
		<input type="color" aria-label={pickerLabel ?? label} {disabled} value={swatch ?? '#000000'} oninput={(e) => onchange(e.currentTarget.value)} />
	</span>
</span>

<style>
	.sc-color-field {
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
	.sc-color-field:hover input[type='text']:not(:disabled),
	input[type='text']:focus {
		border-color: var(--ink);
	}
	/* the swatch: a square in the field's left padding, 2px ink frame (paper colours stay visible) */
	.swatch {
		position: absolute;
		top: 50%;
		left: var(--space-05);
		box-sizing: border-box;
		width: 1.25rem;
		height: 1.25rem;
		transform: translateY(-50%);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--swatch);
	}
	.is-unset .swatch {
		border-color: var(--border-muted);
	}
	.swatch.is-none {
		background: linear-gradient(to top right, transparent calc(50% - 0.0625rem), var(--text-muted) calc(50% - 0.0625rem), var(--text-muted) calc(50% + 0.0625rem), transparent calc(50% + 0.0625rem));
	}
	/* the native picker lies over the swatch, invisible; its focus ring goes on the swatch */
	.swatch input[type='color'] {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		min-height: 0;
		margin: 0;
		padding: 0;
		border: 0;
		opacity: 0;
		cursor: pointer;
	}
	.swatch:has(input:focus-visible) {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
	@media (prefers-reduced-motion: reduce) {
		input[type='text'] {
			transition: none;
		}
	}
</style>
