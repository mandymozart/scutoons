<script lang="ts">
	import { VIEWPORT_FITS, VIEWPORT_POSITIONS, viewportObjectFit, type ViewportFit } from '@scutoons/content';
	import { Segmented } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * How a video sits in its box (full viewport, or a frame in the column) – basic on purpose (owner 2026-10-01): one
	 * segmented row "Ganz · Breite · Höhe · Füllen", and "Oben · Mitte · Unten" only when the position changes anything
	 * on one of the screens (bars or a crop above/below).
	 */
	type Position = (typeof VIEWPORT_POSITIONS)[number];
	let {
		fit,
		position,
		mediaRatio,
		boxRatios,
		onchange
	}: {
		fit: ViewportFit;
		position: Position;
		/** the video's own width / height */
		mediaRatio: number;
		/** the screens it is shown on: a phone + a desktop (full viewport) or the frame (column) */
		boxRatios: number[];
		onchange: (fit: ViewportFit, position: Position) => void;
	} = $props();

	/** slack above/below: contain with a flatter video (bars) or cover with a taller one (cropped top/bottom) */
	const positionMatters = $derived(
		boxRatios.some((box) => {
			if (Math.abs(mediaRatio - box) / box < 0.01) return false;
			const fitted = viewportObjectFit(fit, mediaRatio, box);
			return fitted === 'contain' ? mediaRatio > box : mediaRatio < box;
		})
	);
</script>

<div class="placement-picker" data-testid="placement-picker">
	<!-- one row each, the choices sharing its width -->
	<Segmented
		radio
		grid
		min="0px"
		label={m['creator.video_fit']()}
		value={fit}
		onchange={(f) => onchange(f, position)}
		options={VIEWPORT_FITS.map((f) => ({ value: f, label: m[`creator.video_fit_${f}`](), title: m[`creator.video_fit_${f}_hint`](), testid: `fit-${f}` }))}
	/>
	{#if positionMatters}
		<Segmented
			radio
			grid
			min="0px"
			label={m['creator.video_position']()}
			value={position}
			onchange={(p) => onchange(fit, p)}
			options={VIEWPORT_POSITIONS.map((p) => ({ value: p, label: m[`creator.video_position_${p}`](), testid: `position-${p}` }))}
		/>
	{/if}
</div>

<style>
	.placement-picker {
		display: flex;
		flex-direction: column;
		gap: 0.4em;
	}
</style>
