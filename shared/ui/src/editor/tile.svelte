<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLAnchorAttributes } from 'svelte/elements';
	import Icon from '../icon.svelte';

	/**
	 * A tile (deck-kit fold.rs `tile`): a picture over its word, its line, a tile's corner (1.4 of a control's); the one
	 * `on` in the accent, its ground the faint accent; the hover ground under the pointer. The picture is `icon` or
	 * `picture` (a snippet: an avatar, a thumbnail); `sub` and `meta` are quiet lines under the word.
	 */
	type Props = {
		label: string;
		icon?: string;
		sub?: string;
		meta?: string;
		href?: string;
		onclick?: () => void;
		on?: boolean;
		picture?: Snippet;
	} & Omit<HTMLAnchorAttributes, 'children' | 'onclick'>;

	let { label, icon, sub, meta, href, onclick, on = false, picture, ...rest }: Props = $props();
</script>

{#snippet inside()}
	<span class="picture">{#if picture}{@render picture()}{:else if icon}<Icon name={icon} size="1.3rem" strokeWidth={1.5} />{/if}</span>
	<span class="word">{label}</span>
	{#if sub}<span class="quiet">{sub}</span>{/if}
	{#if meta}<span class="quiet">{meta}</span>{/if}
{/snippet}

{#if href}
	<a {href} {...rest} class="kit-tile {rest.class ?? ''}" class:on>{@render inside()}</a>
{:else}
	<button type="button" {onclick} aria-pressed={on} {...rest as Record<string, unknown>} class="kit-tile {rest.class ?? ''}" class:on>{@render inside()}</button>
{/if}

<style>
	.kit-tile {
		display: flex;
		flex-direction: column;
		align-items: stretch;
		gap: calc(var(--editor-pad) / 2);
		box-sizing: border-box;
		min-width: 0;
		margin: 0;
		padding: var(--editor-pad);
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--kit-radius-tile);
		background: none;
		color: var(--tool-text);
		font: inherit;
		text-align: left;
		text-decoration: none;
		cursor: pointer;
	}
	.picture {
		display: flex;
		align-items: center;
		justify-content: space-between;
		color: var(--tool-muted);
	}
	.word,
	.quiet {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.quiet {
		color: var(--tool-muted);
		font-size: var(--editor-small);
	}
	@media (hover: hover) {
		.kit-tile:hover:not(.on) {
			background: var(--tool-hover);
		}
	}
	.on {
		border-color: var(--tool-accent);
		background: color-mix(in srgb, var(--tool-accent) 14%, transparent);
	}
	.on .picture {
		color: var(--tool-accent);
	}
	.kit-tile:focus-visible {
		outline: 2px solid var(--tool-accent);
		outline-offset: 1px;
	}
</style>
