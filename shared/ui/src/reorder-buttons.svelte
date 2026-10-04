<script lang="ts">
	import Icon from './icon.svelte';

	/**
	 * Move one row of a list up / down (or left / right in a chip row): two icon buttons, disabled at the ends. The list
	 * moves the item itself (`onmove(from, to)`). Creator lists, the admin catalog's order (COMPONENTS.md step 5).
	 */
	let {
		index,
		count,
		onmove,
		labels,
		axis = 'vertical',
		down = true,
		variant = 'ghost',
		disabled = false,
		testid
	}: {
		index: number;
		count: number;
		onmove: (from: number, to: number) => void;
		/** the buttons' tooltips / accessible names */
		labels: { up: string; down: string };
		/** vertical: arrows up / down; horizontal (a chip row): chevrons left / right */
		axis?: 'vertical' | 'horizontal';
		/** false: only "up" (a list that adds at the end) */
		down?: boolean;
		/** the buttons' class (tools.css / app button variants, e.g. ghost, secondary, sc-round) */
		variant?: string;
		/** both off (e.g. while a reorder is saved) */
		disabled?: boolean;
		/** data-testid prefix: <testid>-up / <testid>-down */
		testid?: string;
	} = $props();
</script>

<button type="button" class={variant} disabled={disabled || index <= 0} onclick={() => onmove(index, index - 1)} title={labels.up} aria-label={labels.up} data-testid={testid ? `${testid}-up` : undefined}>
	<Icon name={axis === 'horizontal' ? 'chevron-left' : 'arrow-up'} />
</button>
{#if down}
	<button type="button" class={variant} disabled={disabled || index >= count - 1} onclick={() => onmove(index, index + 1)} title={labels.down} aria-label={labels.down} data-testid={testid ? `${testid}-down` : undefined}>
		<Icon name={axis === 'horizontal' ? 'chevron-right' : 'arrow-down'} />
	</button>
{/if}
