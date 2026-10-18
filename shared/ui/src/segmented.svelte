<script lang="ts" module>
	export interface SegmentedOption<V extends string = string> {
		value: V;
		/** the visible word; without one the option is icon-only and `title` is its name */
		label?: string;
		/** a Feather icon name (Icon) */
		icon?: string;
		/** tooltip; the accessible name of an icon-only option */
		title?: string;
		/** not choosable now (its `title` says why) */
		disabled?: boolean;
		testid?: string;
	}
</script>

<script lang="ts" generics="V extends string">
	import Icon from './icon.svelte';

	/**
	 * One choice out of a few (owner 2026-10-03, COMPONENTS.md step 6): the tools.css `.sc-seg` vocabulary – a framed row,
	 * the chosen option inverted. `value` = the chosen option (none chosen: null, e.g. a preset that no longer matches);
	 * `onchange` gets the clicked option (also the chosen one again – a preset is re-applied). `radio` makes it a
	 * radiogroup (aria-checked) instead of toggle buttons (aria-pressed). `grid` wraps into equal columns of at least
	 * `min` (a CSS length). `vertical` stacks the options in a column (a tool strip – the slice editor's, E.4).
	 * Compact property rows (agents/research/EDITOR-INPUTS.md): `stretch` fills the row with equal options, `square`
	 * makes icon-only options control-height squares; `fallback` = the option that applies while nothing is set (an ink
	 * frame, not inverted; `fallbackNote` is added to its tooltip); with `onclear`, a click on the chosen option unsets it.
	 */
	let {
		options,
		value,
		onchange,
		label,
		radio = false,
		grid = false,
		min,
		vertical = false,
		stretch = false,
		square = false,
		fallback,
		fallbackNote = 'default',
		onclear,
		testid
	}: {
		options: SegmentedOption<V>[];
		value: V | null | undefined;
		onchange: (value: V) => void;
		label: string;
		radio?: boolean;
		grid?: boolean;
		min?: string;
		vertical?: boolean;
		stretch?: boolean;
		square?: boolean;
		fallback?: V;
		fallbackNote?: string;
		onclear?: () => void;
		testid?: string;
	} = $props();

	const unset = $derived(value === null || value === undefined);
	const tip = (o: SegmentedOption<V>) => (unset && o.value === fallback ? `${o.title ?? o.label ?? ''} (${fallbackNote})` : o.title);
</script>

<div class="sc-seg" class:is-grid={grid} class:is-column={vertical} class:is-stretch={stretch} role={radio ? 'radiogroup' : 'group'} aria-label={label} style:--seg-min={min} data-testid={testid}>
	{#each options as o (o.value)}
		<button
			type="button"
			class:active={o.value === value}
			class:is-square={square && !o.label}
			class:is-fallback={unset && o.value === fallback}
			role={radio ? 'radio' : undefined}
			aria-checked={radio ? o.value === value : undefined}
			aria-pressed={radio ? undefined : o.value === value}
			title={tip(o)}
			aria-label={!o.label ? tip(o) : undefined}
			disabled={o.disabled}
			onclick={() => (onclear && o.value === value ? onclear() : onchange(o.value))}
			data-testid={o.testid}
		>
			{#if o.icon}<Icon name={o.icon} />{/if}
			{#if o.label}{o.label}{/if}
		</button>
	{/each}
</div>
