<script lang="ts">
	/**
	 * THE close cross (owner rule 2026-10-02, agents/DESIGN.md "Close cross"): one drawn icon in every app – two
	 * straight lines, square, as high as the headline next to it (the cap height of IBM Plex Mono, 0.698 of the
	 * headline's font-size), stroke 2px (= the boxes' 2px border) or 1px on thin-line UI (the camera HUD).
	 * No font glyphs (× ✕ ✖), no Feather "x". The clickable area is the control around it (`CloseButton` or the
	 * app's own --control-height button); the icon is centred in it. Decorative (aria-hidden): the button has the label.
	 */
	type Props = {
		/** font-size of the headline / text it sits next to; default the label size (1rem) */
		size?: string;
		/** stroke width in px: 2 (default, the 2px box border) or 1 (thin-line UI, the font's own stroke) */
		stroke?: 1 | 2;
	};

	let { size = 'var(--type-label-size, 1rem)', stroke = 2 }: Props = $props();
</script>

<svg
	class="sc-close-icon"
	xmlns="http://www.w3.org/2000/svg"
	viewBox="0 0 16 16"
	style:--sc-close-size={size}
	fill="none"
	stroke="currentColor"
	stroke-width={stroke}
	aria-hidden="true"
	focusable="false"
	data-icon="close"
	data-stroke={stroke}
><path d="M0 0L16 16M16 0L0 16" vector-effect="non-scaling-stroke" /></svg>

<style>
	/* the box is the cross: corner to corner, the stroke ends cut by the box, so width = height = the cap height */
	.sc-close-icon {
		display: block;
		flex: none;
		box-sizing: content-box;
		width: calc(var(--sc-close-size) * 0.698);
		height: calc(var(--sc-close-size) * 0.698);
		overflow: hidden;
		pointer-events: none;
	}
</style>
