<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';

	/** Below the creator's minimum width (744 CSS px, RULES H9) this is all it shows. */
</script>

<main class="desktop-notice sc-tool" data-testid="desktop-notice">
	<h1>{m['creator.title']()}</h1>
	<p>{m['creator.desktop_needed']()}</p>
</main>

<style>
	.desktop-notice {
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: var(--space-1);
		min-height: 100dvh;
		padding: var(--space-2);
		background: var(--editor-stage);
	}
	p {
		max-width: 28rem;
		margin: 0;
	}
</style>
