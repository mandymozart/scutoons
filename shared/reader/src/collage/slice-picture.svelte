<script lang="ts">
	import { untrack, type Snippet } from 'svelte';
	import type { CollagePiece } from '@scutoons/content';
	import CollagePieceView from './collage-piece.svelte';
	import { drawablePiece } from './collage-css.ts';

	/**
	 * A slice picture and its collage pieces (agents/research/COLLAGE.md §3 "The picture group"): `children` is the host's
	 * own <img> (a snippet, so the host's `img` styles still apply), the pieces lie on it – under the lettering (the
	 * slice's `.slice-layers` paints above), clipped to the picture, in array order (back → front).
	 *
	 * `.slice-media` is THE blend group: `isolation: isolate` makes the backdrop of every blend the slice picture alone –
	 * the same in the column, the strip, the endless band, old iOS, the creator canvas. It paints at the z-0 level, so the
	 * previous slice's `.slice-layers` (z 1) still lies over it, and an fx impact moves it as one child of the slice.
	 * `.slice-pieces` must never become a group of its own (z-index, transform, opacity, filter, will-change, isolation,
	 * container-type): the pieces would blend with nothing.
	 *
	 * Drawn only with pieces and a resolver (`pieceSrc`; none = no art, like items without `itemInfo`) – a slice without
	 * pieces keeps today's DOM (no wrapper). `always` = the wrapper even without pieces (the creator canvas: no remount
	 * when the first piece arrives). The pieces mount once the slice comes near (one screen ahead) and then stay – CSS
	 * masks don't load lazily; `eager` mounts them at once (the first slices, the creator).
	 */
	let {
		pieces,
		pieceSrc,
		always = false,
		eager = false,
		children
	}: {
		pieces?: CollagePiece[];
		/** a piece's absolute file URL from its `src` (the host's path form); null = not drawn */
		pieceSrc?: (src: string) => string | null | undefined;
		always?: boolean;
		eager?: boolean;
		/** the picture */
		children: Snippet;
	} = $props();

	const wanted = $derived(!!pieceSrc && Array.isArray(pieces) && pieces.length > 0);
	let near = $state(false);

	/** the pieces with a file, each id once (a broken document draws what it can – the validator names the rest) */
	const drawn = $derived.by(() => {
		if (!wanted || !near) return [];
		const seen = new Set<string>();
		return pieces!.flatMap((piece) => {
			if (!drawablePiece(piece) || seen.has(piece.id)) return [];
			const url = pieceSrc!(piece.src);
			if (!url) return [];
			seen.add(piece.id);
			return [{ piece, url }];
		});
	});

	/** near = within one screen of the viewport, once */
	function nearing(node: HTMLElement) {
		if (untrack(() => near)) return;
		if (eager || typeof IntersectionObserver === 'undefined') {
			near = true;
			return;
		}
		const observer = new IntersectionObserver(
			(entries) => {
				if (!entries.some((entry) => entry.isIntersecting)) return;
				near = true;
				observer.disconnect();
			},
			{ rootMargin: '100%' }
		);
		observer.observe(node);
		return () => observer.disconnect();
	}
</script>

{#if always || wanted}
	<div class="slice-media" {@attach nearing}>
		{@render children()}
		{#if drawn.length}
			<div class="slice-pieces" aria-hidden="true">
				{#each drawn as { piece, url } (piece.id)}
					<CollagePieceView {piece} src={url} left={piece.rect.x} top={piece.rect.y} width={piece.rect.width} />
				{/each}
			</div>
		{/if}
	</div>
{:else}
	{@render children()}
{/if}

<style>
	/* the one blend group: the picture + its pieces (see above – isolation is load-bearing) */
	.slice-media {
		position: relative;
		isolation: isolate;
		overflow: hidden;
	}
	/* never a stacking context or container of its own: the pieces must blend with the picture */
	.slice-pieces {
		position: absolute;
		inset: 0;
		pointer-events: none;
	}
</style>
