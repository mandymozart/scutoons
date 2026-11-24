<script lang="ts">
	import type { Snippet } from 'svelte';
	import Icon from '../icon.svelte';

	/**
	 * A thumbnail (deck-kit's still, deck/editor/src/library.rs STILL; kit/tokens.toml `[component.thumb.*]`): the 264 × 168
	 * shape, a tile's corner and line, the picture cut to the shape (`fit` contain shows a whole picture that is not a still
	 * yet). Without a picture: `children` (an avatar …) or the kind's icon on the see-through ground, `empty` as its hover
	 * text. `fill` covers the parent instead (a card's whole ground). `over` lies on top (a count, a badge). `on`: picked
	 * (the text-colour ring), `disabled` faded; `data-kit-state` shows hover, focus or dragged on the kit page.
	 */
	let {
		src = null,
		icon = 'image',
		fit = 'cover',
		size = '1.6rem',
		fill = false,
		empty,
		on = false,
		disabled = false,
		'data-kit-state': kitState,
		class: extra = '',
		children,
		over
	}: {
		src?: string | null;
		icon?: string;
		fit?: 'cover' | 'contain';
		/** the placeholder icon's size */
		size?: string;
		fill?: boolean;
		/** the placeholder's hover text (a still that does not exist yet) */
		empty?: string;
		on?: boolean;
		disabled?: boolean;
		'data-kit-state'?: string;
		class?: string;
		children?: Snippet;
		over?: Snippet;
	} = $props();

	let failed = $state<string | null>(null);
	const shown = $derived(src && failed !== src ? src : null);
</script>

<span class="kit-thumb {extra}" class:fill class:on class:off={disabled} data-kit-state={kitState} data-testid="kit-thumb" data-filled={shown ? '' : undefined}>
	{#if shown}
		<img src={shown} alt="" loading="lazy" draggable="false" class:contain={fit === 'contain'} onerror={() => (failed = shown)} />
	{:else}
		<span class="place" title={empty}>
			{#if children}{@render children()}{:else}<Icon name={icon} {size} strokeWidth={1.25} />{/if}
		</span>
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
		border: var(--editor-edge) solid var(--kit-thumb-rest-border);
		border-radius: var(--kit-radius-tile);
		overflow: hidden;
		color: var(--tool-muted);
	}
	@media (hover: hover) {
		.kit-thumb:not(.fill):hover {
			border-color: var(--kit-thumb-hover-border);
		}
	}
	.kit-thumb[data-kit-state~='hover'] {
		border-color: var(--kit-thumb-hover-border);
	}
	.kit-thumb.on {
		border-color: var(--kit-thumb-on-ring);
		box-shadow: 0 0 0 var(--editor-edge) var(--kit-thumb-on-ring);
	}
	.kit-thumb[data-kit-state~='focus'] {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: 1px;
	}
	.kit-thumb.off {
		opacity: var(--kit-thumb-disabled-opacity);
	}
	.kit-thumb[data-kit-state~='dragged'] {
		opacity: var(--kit-thumb-dragged-opacity);
	}
	.kit-thumb.fill {
		position: absolute;
		inset: 0;
		width: auto;
		aspect-ratio: auto;
		border: 0;
		border-radius: inherit;
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
