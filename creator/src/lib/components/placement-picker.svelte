<script lang="ts">
	import { VIEWPORT_FITS, VIEWPORT_POSITIONS, viewportObjectFit, type ViewportFit } from '@scutoons/content';
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
	<div class="row" role="radiogroup" aria-label={m['creator.video_fit']()}>
		{#each VIEWPORT_FITS as f (f)}
			<button
				type="button"
				class:active={fit === f}
				role="radio"
				aria-checked={fit === f}
				title={m[`creator.video_fit_${f}_hint`]()}
				onclick={() => onchange(f, position)}
				data-testid={`fit-${f}`}>{m[`creator.video_fit_${f}`]()}</button
			>
		{/each}
	</div>
	{#if positionMatters}
		<div class="row" role="radiogroup" aria-label={m['creator.video_position']()}>
			{#each VIEWPORT_POSITIONS as p (p)}
				<button
					type="button"
					class:active={position === p}
					role="radio"
					aria-checked={position === p}
					onclick={() => onchange(fit, p)}
					data-testid={`position-${p}`}>{m[`creator.video_position_${p}`]()}</button
				>
			{/each}
		</div>
	{/if}
</div>

<style>
	.placement-picker {
		display: flex;
		flex-direction: column;
		gap: 0.4em;
	}
	/* a segmented row: square house buttons, grey outline, hover ink, the chosen one inverted (em = the creator's 13 px) */
	.row {
		display: flex;
	}
	.row button {
		flex: 1;
		min-width: 0;
		margin-left: -2px;
		padding: 0 0.25em;
		font: inherit;
		font-size: 0.9em;
		line-height: var(--type-button-line);
		background: var(--paper);
		color: var(--ink);
		border: 2px solid var(--border-muted);
		border-radius: 0;
		cursor: pointer;
	}
	.row button:first-child {
		margin-left: 0;
	}
	.row button:hover {
		position: relative;
		border-color: var(--ink);
	}
	.row button.active {
		position: relative;
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
</style>
