<script lang="ts">
	import { getVisualNovelContext } from '$lib/contexts/visual-novel.svelte.ts';

	/**
	 * While a showTextBox event with `wait` plays (its text box is drawn by the slice), a tap anywhere continues the
	 * script. Nothing is drawn.
	 */
	const vn = getVisualNovelContext();
	const waiting = $derived(vn.state.activeTextBoxes.some((o) => o.wait));
</script>

{#if waiting}
	<button type="button" class="text-box-wait" aria-label="Continue" onclick={() => vn.continueFromTextBox()}></button>
{/if}

<style>
	.text-box-wait {
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
