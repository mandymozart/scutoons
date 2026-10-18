<script lang="ts">
	import { onMount } from 'svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { picturePick } from '$lib/editor/picture-pick.svelte.ts';
	import { picturePixels, sampleAt, type Pixels } from '$lib/editor/picture-pick.ts';

	/**
	 * The eyedropper's fallback over the slice canvas (owner 2026-10-18: "pick the colour from the slice", where the
	 * browser has no EyeDropper): while a ColorField waits (`picturePick.active`), the whole canvas is a picking surface –
	 * the pointer shows the colour under it (a chip with the swatch and its hex), a tap / click takes it, Esc cancels.
	 * The colour is the slice picture's own pixel (picture-pick.ts) – pieces and text on top aren't part of it.
	 */
	let { image }: { /** the slice picture's URL (the CDN, or a local replacement) */ image: string } = $props();

	let pixels = $state<Pixels | null | undefined>(undefined);
	let hover = $state<{ x: number; y: number; hex: string } | null>(null);

	onMount(() => {
		if (!image) pixels = null;
		else picturePixels(image).then((p) => (pixels = p));
	});

	function at(e: PointerEvent) {
		const r = (e.currentTarget as HTMLElement).getBoundingClientRect();
		const fx = (e.clientX - r.left) / r.width;
		const fy = (e.clientY - r.top) / r.height;
		return { x: fx * 100, y: fy * 100, hex: pixels ? sampleAt(pixels, fx, fy) : null };
	}

	function onpointermove(e: PointerEvent) {
		const p = at(e);
		hover = p.hex ? { x: p.x, y: p.y, hex: p.hex } : null;
	}

	function onpointerdown(e: PointerEvent) {
		e.preventDefault();
		e.stopPropagation();
		const p = at(e);
		if (p.hex) picturePick.finish(p.hex);
	}

	/** Esc cancels – before the canvas' own Esc (which drops the selection) */
	function onkeydown(e: KeyboardEvent) {
		if (e.key !== 'Escape') return;
		e.preventDefault();
		e.stopPropagation();
		picturePick.finish(null);
	}
</script>

<svelte:window onkeydowncapture={onkeydown} />

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div class="picture-pick" {onpointermove} {onpointerdown} onpointerleave={() => (hover = null)} data-testid="picture-pick">
	<span class="sc-chip hint" role="status">{pixels === null ? m['creator.pick_unreadable']() : m['creator.pick_hint']()}</span>
	{#if hover}
		<span class="chip" style:left="{hover.x}%" style:top="{hover.y}%" data-testid="picture-pick-chip">
			<span class="swatch" style:background={hover.hex}></span>{hover.hex}
		</span>
	{/if}
</div>

<style>
	/* over everything on the canvas while a colour is picked */
	.picture-pick {
		position: absolute;
		inset: 0;
		z-index: 5;
		cursor: crosshair;
		touch-action: none;
	}
	.hint {
		position: absolute;
		top: var(--space-05);
		left: 50%;
		transform: translateX(-50%);
		white-space: nowrap;
		pointer-events: none;
	}
	/* the colour under the pointer: a paper chip with an ink frame, beside the pointer */
	.chip {
		position: absolute;
		display: inline-flex;
		align-items: center;
		gap: var(--space-025);
		padding: var(--space-025) var(--space-05);
		transform: translate(var(--space-05), var(--space-05));
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		pointer-events: none;
		white-space: nowrap;
	}
	.swatch {
		width: 1rem;
		height: 1rem;
		border: var(--border-width-regular) solid var(--ink);
	}
</style>
