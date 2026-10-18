<script lang="ts">
	import { onMount } from 'svelte';
	import { gridArrows } from '@scutoons/ui';
	import { filterShapes, shapePick, shapeTags, type PickedPiece } from '$lib/editor/collage-pick.ts';
	import { shapeLibrary } from '$lib/editor/shapes.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The collage picker's Shapes tab (research/COLLAGE.md §5, VECTORS §6 step 8): the vector library of the asset manager
	 * (loaded once per session) as picture cards on a checker – a one-colour shape (`mask`) as an ink silhouette, since
	 * its colour is the look's, not the file's – with a search (title, file, tags) and a tag select by frequency.
	 * Keyboard: arrows move in the grid, Enter picks.
	 */
	let { onpick }: { onpick: (picked: PickedPiece) => void } = $props();

	let query = $state('');
	let tag = $state('');
	onMount(() => void shapeLibrary.load());

	const all = $derived(shapeLibrary.items ?? []);
	const tags = $derived(shapeTags(all));
	const shown = $derived(filterShapes(all, query, tag));
	const fileName = (path: string) => path.split('/').pop()?.replace(/\.svg$/i, '') ?? path;
</script>

{#if shapeLibrary.error}
	<p class="sc-error" data-testid="collage-shapes-error">{m['creator.collage_shapes_error']({ error: shapeLibrary.error })}</p>
{:else if shapeLibrary.items && !shapeLibrary.items.length}
	<p class="sc-empty" data-testid="collage-shapes-empty">{m['creator.collage_shapes_empty']()}</p>
{:else if shapeLibrary.items}
	<div class="sc-picker-filters">
		<!-- svelte-ignore a11y_autofocus -->
		<input type="search" bind:value={query} placeholder={m['creator.collage_shapes_search']()} aria-label={m['creator.collage_shapes_search']()} autofocus data-testid="collage-shape-search" />
		<select bind:value={tag} aria-label={m['creator.collage_tag']()} data-testid="collage-shape-tag">
			<option value="">{m['creator.collage_all_tags']()}</option>
			{#each tags as t (t.tag)}<option value={t.tag}>{t.tag} ({t.count})</option>{/each}
		</select>
	</div>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="sc-cards" style:--cards-min="5.5rem" onkeydown={gridArrows} data-testid="collage-shapes">
		{#each shown as shape (shape.id)}
			<button type="button" class="sc-card" data-control-exempt="picture card" data-cell title={shape.title} onclick={() => onpick(shapePick(shape))} data-testid="collage-shape-{fileName(shape.path)}">
				<span class="sc-card-img checker"><img class:ink={shape.shape?.mask} src={asset(shape.path)} alt="" loading="lazy" decoding="async" /></span>
				<span class="name">{shape.title}</span>
			</button>
		{/each}
	</div>
{:else}
	<p class="sc-caption">{m['creator.loading']()}</p>
{/if}

<style>
	/* transparency shows as a checker (the shapes are cut out) */
	.checker {
		background-color: var(--paper);
		background-image: conic-gradient(var(--surface-hover) 25%, transparent 0 50%, var(--surface-hover) 0 75%, transparent 0);
		background-size: 1rem 1rem;
	}
	.sc-card-img img {
		width: 80%;
		height: 80%;
	}
	/* a one-colour shape: its silhouette in ink (the colour comes from the look) */
	.ink {
		filter: brightness(0);
	}
	.name {
		overflow: hidden;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
