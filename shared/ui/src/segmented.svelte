<script lang="ts" module>
	export interface SegmentedOption<V extends string = string> {
		value: V;
		/** the visible word; without one the option is icon-only and `title` is its name */
		label?: string;
		/** a Feather icon name (Icon) */
		icon?: string;
		/** tooltip; the accessible name of an icon-only option */
		title?: string;
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
	 * `min` (a CSS length).
	 */
	let {
		options,
		value,
		onchange,
		label,
		radio = false,
		grid = false,
		min,
		testid
	}: {
		options: SegmentedOption<V>[];
		value: V | null | undefined;
		onchange: (value: V) => void;
		label: string;
		radio?: boolean;
		grid?: boolean;
		min?: string;
		testid?: string;
	} = $props();
</script>

<div class="sc-seg" class:is-grid={grid} role={radio ? 'radiogroup' : 'group'} aria-label={label} style:--seg-min={min} data-testid={testid}>
	{#each options as o (o.value)}
		<button
			type="button"
			class:active={o.value === value}
			role={radio ? 'radio' : undefined}
			aria-checked={radio ? o.value === value : undefined}
			aria-pressed={radio ? undefined : o.value === value}
			title={o.title}
			aria-label={!o.label ? o.title : undefined}
			onclick={() => onchange(o.value)}
			data-testid={o.testid}
		>
			{#if o.icon}<Icon name={o.icon} />{/if}
			{#if o.label}{o.label}{/if}
		</button>
	{/each}
</div>
