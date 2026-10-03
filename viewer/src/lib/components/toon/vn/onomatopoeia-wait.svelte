<script lang="ts">
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte.ts';

	/**
	 * While an onomatopoeia event with `wait` plays (its text box is drawn by the slice), a tap anywhere continues the
	 * script. Nothing is drawn.
	 */
	const vn = getVisualNovelContext();
	const waiting = $derived(vn.state.activeOnomatopoeia.some((o) => o.wait));
</script>

{#if waiting}
	<button type="button" class="onomatopoeia-wait" aria-label="Continue" onclick={() => vn.continueFromOnomatopoeia()}></button>
{/if}

<style>
	.onomatopoeia-wait {
		position: absolute;
		inset: 0;
		z-index: 50;
		margin: 0;
		padding: 0;
		border: 0;
		background: none;
		cursor: pointer;
	}
</style>
