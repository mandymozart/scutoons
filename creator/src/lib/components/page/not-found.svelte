<script lang="ts">
	import { Button } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';

	/** An address that leads nowhere: one quiet line in the middle of the stage and the way back Home. */
	let { what = m['shell.not_found']() }: { what?: string } = $props();
</script>

<div class="not-found" data-testid="not-found">
	<p class="sc-empty">{what}</p>
	<Button href="/" icon="home" data-testid="not-found-home">{m['shell.back_home']()}</Button>
</div>

<style>
	/* in the middle of the window, like the stage's other quiet lines; only the way back takes the pointer */
	.not-found {
		position: fixed;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		pointer-events: none;
	}
	.not-found > :global(*) {
		pointer-events: auto;
	}
</style>
