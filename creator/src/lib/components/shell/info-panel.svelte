<script lang="ts">
	import { info, infoOf } from '$lib/shell/info.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/** The Info view (deck-kit info.rs): the name and description of what is under the pointer – the member's choice to open it */
	function onpointerover(e: PointerEvent) {
		const said = infoOf(e.target);
		if (said) info.shown = said;
	}
</script>

<svelte:window {onpointerover} />

<div class="info" data-testid="info-view">
	{#if info.shown}
		<p class="name">{info.shown.name}</p>
		{#if info.shown.text}<p class="text">{info.shown.text}</p>{/if}
	{:else}
		<p class="sc-empty">{m['shell.info_empty']()}</p>
	{/if}
</div>

<style>
	.info {
		display: flex;
		flex-direction: column;
		gap: calc(var(--editor-pad) / 2);
		padding: var(--editor-pad);
		font-size: var(--editor-small);
	}
	.name,
	.text {
		margin: 0;
	}
	.text {
		color: var(--tool-muted);
	}
</style>
