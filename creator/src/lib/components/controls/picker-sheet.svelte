<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Sheet } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The frame of the creator's picker sheets (character, item; COMPONENTS.md step 7 – the asset picker lives in @scutoons/media with the same vocabulary): the shared Sheet from the
	 * right (drag wider, up to half the window), one column that never scrolls sideways. Inside, the pickers use the
	 * tools.css vocabulary: `.sc-picker-filters` (search + selects), `.sc-cards` + `.sc-card` (+ `.sc-card-img`) with
	 * `data-cell` and `onkeydown={gridArrows}` (@scutoons/ui).
	 */
	let { open = $bindable(false), title, testid, children }: { open?: boolean; title: string; testid: string; children: Snippet } = $props();
</script>

<Sheet bind:open {title} side="right" widthKey="picker" closeLabel={m['creator.close']()} resizeLabel={m['creator.resize_panel']()}>
	<div class="picker" data-testid={testid}>
		{@render children()}
	</div>
</Sheet>

<style>
	.picker {
		display: flex;
		flex-direction: column;
		gap: var(--space-075);
		/* fill the sheet, never wider: the lists wrap into rows instead of scrolling sideways */
		width: 100%;
		max-width: 100%;
		min-width: 0;
		overflow-x: hidden;
	}
</style>
