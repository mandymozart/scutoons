<script lang="ts">
	import type { CollagePiece } from '@scutoons/content';
	import CollagePieceView from '../collage/collage-piece.svelte';
	import { drawablePiece } from '../collage/collage-css.ts';
	import { pageFrame, placeScorePiece, type PageGeometry } from './score-anchors.ts';

	/**
	 * A score page's collage pieces (agents/research/COLLAGE.md §4.3): the pieces whose bar is on this page, placed by
	 * score-anchors.ts in % of the page – the layer lies exactly on the music (`pageFrame`: the SVG's viewBox in the
	 * page's leaf, whose corner is the SVG element's). Each piece carries its own z: behind the notes 0, over them 2 (the
	 * view lifts the music to 1 while there are pieces). The layer itself must never become a group (z-index, transform,
	 * opacity, filter, will-change, isolation, container-type): the pieces blend with the music and the paper of the
	 * whole view. A piece whose bar this page doesn't hold, a broken one, one without a file: not drawn here (the stage
	 * lists missing bars).
	 */
	let {
		pieces,
		geometry,
		pieceSrc
	}: {
		pieces: CollagePiece[];
		geometry: PageGeometry;
		/** a piece's absolute file URL from its `src` (a catalog row's CDN-root path); null = not drawn */
		pieceSrc: (src: string) => string | null | undefined;
	} = $props();

	const frame = $derived(pageFrame(geometry));
	const drawn = $derived.by(() => {
		const seen = new Set<string>();
		return pieces.flatMap((piece) => {
			if (!drawablePiece(piece) || seen.has(piece.id)) return [];
			const place = placeScorePiece(piece, geometry);
			const url = place && pieceSrc(piece.src);
			if (!place || !url) return [];
			seen.add(piece.id);
			return [{ piece, url, place }];
		});
	});
</script>

{#if drawn.length}
	<div class="score-pieces" style:width={frame.width} style:aspect-ratio={frame['aspect-ratio']} style:margin-left={frame['margin-left']} style:margin-top={frame['margin-top']} aria-hidden="true" data-testid="score-pieces">
		{#each drawn as { piece, url, place } (piece.id)}
			<CollagePieceView {piece} src={url} left={place.left} top={place.top} width={place.width} z={piece.behind ? 0 : 2} />
		{/each}
	</div>
{/if}

<style>
	/* never a stacking context or container of its own (see above – the blends are load-bearing); clipped to the page,
	   so a piece over its edge never makes the leaf scroll (overflow makes no group) */
	.score-pieces {
		position: absolute;
		left: 0;
		top: 0;
		overflow: hidden;
		pointer-events: none;
	}
</style>
