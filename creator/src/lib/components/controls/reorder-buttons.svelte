<script lang="ts">
	import { ReorderButtons } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/** @scutoons/ui ReorderButtons with the creator's default labels ("Move up" / "Move down") */
	let {
		labels,
		...rest
	}: {
		index: number;
		count: number;
		onmove: (from: number, to: number) => void;
		axis?: 'vertical' | 'horizontal';
		down?: boolean;
		variant?: string;
		disabled?: boolean;
		labels?: { up?: string; down?: string };
	} = $props();
</script>

<ReorderButtons {...rest} labels={{ up: labels?.up ?? m['creator.move_up'](), down: labels?.down ?? m['creator.move_down']() }} />
