<script lang="ts">
	import { bakePresets, type CollagePiece, type PresetList } from '@scutoons/content';
	import { CollagePieceView } from '@scutoons/reader';

	/**
	 * A look's preview in the piece inspector's grid (owner 2026-10-18: "man könnte in dem Wort auf dem Selector zeigen,
	 * was das dann wird, wie das aussieht"): the selected piece's own file in that look – a code look or a custom preset
	 * (baked like the canvas draws it, research/COLLAGE.md §12) –, drawn by the readers' renderer (CollagePieceView, the
	 * canvas' and the stage's), centred and fitted into a small checker tile, never distorted; its blend reaches the
	 * checker (the tile is its own group). Every effect length is % of the piece's width, so the thumbnail reads like the
	 * piece on the canvas. No look = As drawn: the file plain. The same file the canvas loaded (the browser's cache).
	 */
	let {
		piece,
		src,
		look,
		presets
	}: {
		piece: Pick<CollagePiece, 'src' | 'ratio'>;
		/** the file's URL */
		src: string;
		/** the look's id (none = As drawn) */
		look?: string;
		presets?: PresetList;
	} = $props();

	/** the tile's width ÷ height (`.look-preview`'s aspect-ratio) and how much of it the piece may fill (room for outlines, shadows, glows) */
	const BOX = 3 / 2;
	const FIT = 72;
	const shown = $derived.by(() => {
		const ratio = piece.ratio > 0 ? piece.ratio : 1;
		const width = ratio >= BOX ? FIT : (FIT * ratio) / BOX;
		const one: CollagePiece = { id: `look-${look ?? 'none'}`, src: piece.src, ratio, rect: { x: 50, y: 50, width }, anchor: 'c', ...(look ? { preset: look } : {}) };
		return bakePresets([one], presets)[0];
	});
</script>

<span class="look-preview sc-checker" aria-hidden="true">
	<CollagePieceView piece={shown} {src} left={50} top={50} width={shown.rect.width} />
</span>

<style>
	/* its own group: the piece's blend reaches this checker only (like the slice picture's .slice-media); inset in its
	   tile, whose chosen state shows around it */
	.look-preview {
		--checker-size: 0.5rem;
		position: relative;
		display: block;
		margin-inline: var(--space-025);
		aspect-ratio: 3 / 2;
		overflow: hidden;
		isolation: isolate;
	}
</style>
