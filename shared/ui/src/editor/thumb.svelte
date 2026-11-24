<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from '../icon.svelte';

	/**
	 * A thumbnail (deck-kit's still, deck/editor/src/library.rs STILL): the 264 × 168 shape, a tile's corner and line, the
	 * picture cut to the shape (`fit` contain shows a whole picture that is not a still yet). Without a picture the
	 * placeholder: `children` (an avatar …) or the kind's icon. `over` lies on top (a count, a badge).
	 */
	let {
		src = null,
		icon = 'image',
		fit = 'cover',
		size = '1.6rem',
		class: extra = '',
		children,
		over
	}: {
		src?: string | null;
		icon?: string;
		fit?: 'cover' | 'contain';
		/** the placeholder icon's size */
		size?: string;
		class?: string;
		children?: Snippet;
		over?: Snippet;
	} = $props();

	let failed = $state<string | null>(null);
	const shown = $derived(src && failed !== src ? src : null);
</script>

<span class="kit-thumb {extra}" data-testid="kit-thumb" data-filled={shown ? '' : undefined}>
	{#if shown}
		<img src={shown} alt="" loading="lazy" draggable="false" class:contain={fit === 'contain'} onerror={() => (failed = shown)} />
	{:else if children}
		<span class="place">{@render children()}</span>
	{:else}
		<span class="place"><Icon name={icon} {size} strokeWidth={1.25} /></span>
	{/if}
	{#if over}{@render over()}{/if}
</span>

<style>
	.kit-thumb {
		position: relative;
		display: grid;
		place-items: center;
		box-sizing: border-box;
		width: 100%;
		aspect-ratio: 264 / 168;
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--kit-radius-tile);
		overflow: hidden;
		color: var(--tool-muted);
	}
	img {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		object-fit: cover;
		border-radius: inherit;
	}
	img.contain {
		object-fit: contain;
	}
	.place {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--editor-gap);
	}
</style>
