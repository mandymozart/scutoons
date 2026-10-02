<script lang="ts">
	import type { Snippet } from 'svelte';

	/** Entity-detail section (admin UI pattern): header with title + one-line hint, content, no frame. */
	let {
		title,
		hint,
		media,
		actions,
		children
	}: {
		title: string;
		hint?: string;
		/** leading visual before the title (e.g. a ToonAvatar) */
		media?: Snippet;
		actions?: Snippet;
		children: Snippet;
	} = $props();
</script>

<section class="section">
	<header>
		<div class="heading">
			{#if media}{@render media()}{/if}
			<div>
				<h2>{title}</h2>
				{#if hint}<p class="hint">{hint}</p>{/if}
			</div>
		</div>
		{#if actions}<div class="actions">{@render actions()}</div>{/if}
	</header>
	{@render children()}
</section>

<style>
	.section {
		padding: var(--space-15) 0;
		border-top: var(--border-width-regular) solid var(--ink);
	}
	header {
		display: flex;
		align-items: flex-start;
		justify-content: space-between;
		gap: var(--space-1);
		margin-bottom: var(--space-1);
	}
	.heading {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		min-width: 0;
	}
	h2 {
		margin: 0;
		font-size: var(--type-label-size);
		line-height: var(--type-label-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.hint {
		margin: var(--space-025) 0 0;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.actions {
		display: flex;
		gap: var(--space-05);
		flex: none;
	}
</style>
