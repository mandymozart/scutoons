<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAttributes } from 'svelte/elements';

	/**
	 * A card on the stage (deck-kit editor home.rs): the floating panel's ground, twice a control's corner, its line, its
	 * text with the halo; `title` small uppercase and muted at its top, `count` beside it in the warn colour.
	 */
	type Props = { title?: string; count?: number; aside?: Snippet; children: Snippet } & Omit<HTMLAttributes<HTMLElement>, 'children' | 'title'>;
	let { title, count, aside, children, ...rest }: Props = $props();
</script>

<section {...rest} class="kit-card {rest.class ?? ''}">
	{#if title}<h2 class="title">{title}{#if count}<span class="count">{count}</span>{/if}{@render aside?.()}</h2>{/if}
	{@render children()}
</section>

<style>
	.kit-card {
		display: flex;
		flex-direction: column;
		gap: calc(var(--editor-pad) / 2);
		box-sizing: border-box;
		min-width: 0;
		padding: var(--editor-pad) var(--editor-pad) var(--editor-pad);
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--kit-radius-box);
		background: var(--tool-panel);
		color: var(--tool-text);
		text-shadow: var(--tool-halo);
	}
	.kit-card :global(svg) {
		filter: var(--tool-icon-halo);
	}
	.title {
		display: flex;
		align-items: center;
		gap: var(--kit-icon-gap);
		min-height: var(--control-height);
		margin: 0;
		padding: 0 calc(var(--editor-pad) / 2);
		color: var(--tool-muted);
		font-size: var(--editor-label);
		font-weight: inherit;
		letter-spacing: var(--editor-label-spacing);
		text-transform: uppercase;
	}
	.count {
		color: var(--tool-warn);
	}
</style>
