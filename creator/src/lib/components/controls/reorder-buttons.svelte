<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Move one row of a list up / down (or left / right in a chip row): two icon buttons, disabled at the ends. The list
	 * moves the item itself (`onmove(from, to)`, usually with `moveAt` from `$lib/editor/move.ts`).
	 */
	let {
		index,
		count,
		onmove,
		axis = 'vertical',
		down = true,
		variant = 'ghost',
		labels
	}: {
		index: number;
		count: number;
		onmove: (from: number, to: number) => void;
		/** vertical: arrows up / down; horizontal (a chip row): chevrons left / right */
		axis?: 'vertical' | 'horizontal';
		/** false: only "up" (a list that adds at the end) */
		down?: boolean;
		/** the buttons' class (tools.css / app button variants) */
		variant?: string;
		labels?: { up?: string; down?: string };
	} = $props();

	const up = $derived(labels?.up ?? m['creator.move_up']());
	const downLabel = $derived(labels?.down ?? m['creator.move_down']());
</script>

<button type="button" class={variant} disabled={index <= 0} onclick={() => onmove(index, index - 1)} title={up} aria-label={up}>
	<Icon name={axis === 'horizontal' ? 'chevron-left' : 'arrow-up'} />
</button>
{#if down}
	<button type="button" class={variant} disabled={index >= count - 1} onclick={() => onmove(index, index + 1)} title={downLabel} aria-label={downLabel}>
		<Icon name={axis === 'horizontal' ? 'chevron-right' : 'arrow-down'} />
	</button>
{/if}
