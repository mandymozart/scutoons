<script lang="ts">
	import type { CollagePiece } from '@scutoons/content';
	import { cssUrl, pieceCss } from './collage-css.ts';

	/**
	 * One collage piece – a shape, a character picture or an image – on a slice or a score (agents/research/COLLAGE.md
	 * §6). `left` / `top` = the anchor point and `width` in % of the host box (a slice: the piece's rect; a score: the
	 * bar's place on the page); the height follows the file's `ratio`, never distorted. All CSS comes from
	 * collage-css.ts, so the reader and the creator draw it identically:
	 *
	 *   .piece  box · translate (anchor) · rotate · mix-blend-mode · opacity · z · container-type (cqw = % of the piece)
	 *     .fx   filter: outline ×4 → shadow → blur
	 *       the paint: the file as drawn (<img>), or its alpha as a mask on `.fill` (a colour / gradient)
	 *   a progressive blur: two padded `.band`s – sharp and blurred – crossfaded by inverse masks
	 *
	 * The blend sits on `.piece`, an HTML box (blending SVG children differs in Firefox); it blends with what lies behind
	 * the piece inside its host's isolated group (SlicePicture's `.slice-media`). No inline SVG: nothing to sanitise.
	 */
	let {
		piece,
		src,
		left,
		top,
		width,
		z
	}: {
		piece: CollagePiece;
		/** the file's absolute URL (the host resolves `piece.src`) */
		src: string;
		left: number;
		top: number;
		width: number;
		/** a score piece's own stacking (behind the notes 0, over them 2); slices: none */
		z?: number;
	} = $props();

	const css = $derived(pieceCss(piece, left, top, width, z));
	const url = $derived(cssUrl(src));
</script>

{#snippet paint()}
	{#if css.fill}
		<div class="fill" style:--src={url} style:background={css.fill}></div>
	{:else}
		<img {src} alt="" loading="lazy" decoding="async" draggable="false" />
	{/if}
{/snippet}

<div class="piece" style={css.box} data-piece={piece.id}>
	{#if css.bands}
		{@const pad = css.bands.pad}
		<div class="band" style:inset="{-pad}cqw" style:--mask={css.bands.sharp}>
			<div class="fx" style:inset="{pad}cqw" style:filter={css.filter}>{@render paint()}</div>
		</div>
		<div class="band" style:inset="{-pad}cqw" style:--mask={css.bands.soft}>
			<div class="fx" style:inset="{pad}cqw" style:filter={css.bands.filter}>{@render paint()}</div>
		</div>
	{:else}
		<div class="fx" style:filter={css.filter}>{@render paint()}</div>
	{/if}
</div>

<style>
	/* the piece's own group: its effects composite here, then the whole blends with what lies behind */
	.piece {
		position: absolute;
		container-type: inline-size;
		pointer-events: none;
	}
	.band,
	.fx {
		position: absolute;
		inset: 0;
	}
	/* a mask cuts at its box: the band is padded by the paint's spill (collage-css.ts blurBands) */
	.band {
		-webkit-mask-image: var(--mask);
		mask-image: var(--mask);
	}
	.fx img {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: contain;
		-webkit-user-drag: none;
		user-select: none;
	}
	/* the file's alpha as the silhouette; the colour / gradient is the background */
	.fill {
		position: absolute;
		inset: 0;
		-webkit-mask: var(--src) center / contain no-repeat;
		mask: var(--src) center / contain no-repeat;
	}
</style>
