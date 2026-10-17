<script lang="ts">
	import { untrack } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { decodeImages } from '$lib/components/toon/renderer/strip-dom.ts';
	import { slidesPager, type SlidesPager, type SlidesState } from './slides-pager.ts';

	/**
	 * An edition's images on the landing page (DESIGN.md "Landing page"): 1 per view below 768px, 3 from 768px, looping,
	 * dragged sideways, one slide on every 5 s until the reader takes over; prev / pause / next below (slides-pager.ts –
	 * the endless band's utility, owner 2026-10-17). Keyboard: the three buttons (the WAI-ARIA carousel pattern) – the
	 * viewport takes no focus, so the utility's ← / → stay unused here. Without JS the CSS alone shows the first slides.
	 */
	let { images, label }: { images: string[]; label: string } = $props();

	let region = $state<HTMLElement>();
	let viewport = $state<HTMLElement>();
	let track = $state<HTMLElement>();
	let toggleButton = $state<HTMLElement>();
	let view = $state.raw<SlidesState>({ index: 0, count: 0, perView: 1, movable: false, playing: false, dragging: false });
	/** the slider is near the screen: every image loads now and is decoded (a slide comes in by a drag, not the scroll) */
	let near = $state(false);
	let pager: SlidesPager | undefined;

	$effect(() => {
		if (!region || !viewport || !track) return;
		const made = slidesPager(region, viewport, track, { autoplay: 5000, toggle: () => toggleButton, onstate: (s) => (view = s) });
		pager = made;
		const nearing = new IntersectionObserver((entries) => entries.some((e) => e.isIntersecting) && ((near = true), nearing.disconnect()), { rootMargin: '100% 0px' });
		nearing.observe(region);
		return () => {
			nearing.disconnect();
			made.destroy();
			pager = undefined;
		};
	});

	// another variant's images: measured anew once they are in the DOM
	$effect(() => {
		void images;
		untrack(() => pager?.update());
	});

	$effect(() => {
		if (near && viewport) return decodeImages(viewport);
	});
</script>

<div
	class="slides"
	role="region"
	aria-roledescription={m['home.slides_role']()}
	aria-label={label}
	data-slides
	data-index={view.index}
	data-per-view={view.perView}
	data-playing={view.playing}
	bind:this={region}
>
	<div class="viewport" class:movable={view.movable} class:dragging={view.dragging} bind:this={viewport}>
		<div class="track" bind:this={track}>
			{#each images as image, i (image)}
				<div class="slide" role="group" aria-roledescription={m['home.slide_role']()} aria-label={m['home.slide_of']({ number: i + 1, count: images.length })}>
					<img src={asset(image)} alt={label} loading={near ? 'eager' : 'lazy'} draggable="false" />
				</div>
			{/each}
		</div>
	</div>
	{#if view.movable}
		<div class="controls">
			<button type="button" aria-label={m['home.slides_prev']()} title={m['home.slides_prev']()} onclick={() => pager?.step(-1)} data-slides-prev>
				<Icon name="chevron-left" />
			</button>
			<button
				type="button"
				aria-label={view.playing ? m['home.slides_pause']() : m['home.slides_play']()}
				title={view.playing ? m['home.slides_pause']() : m['home.slides_play']()}
				onclick={() => pager?.toggle()}
				bind:this={toggleButton}
				data-slides-toggle
			>
				<Icon name={view.playing ? 'pause' : 'play'} />
			</button>
			<button type="button" aria-label={m['home.slides_next']()} title={m['home.slides_next']()} onclick={() => pager?.step(1)} data-slides-next>
				<Icon name="chevron-right" />
			</button>
		</div>
	{/if}
</div>

<style>
	/* the layout options live here: slides-pager.ts reads them (per view a number, the gap in rem): the gap is 1rem below
	   768px (14px up to 480px, 16px above – shared/tokens/base.css), 16px from 768px (the root is 24px there) */
	.slides {
		--slides-per-view: 1;
		--slides-gap: 1rem;
		--slides-icon: 1.25rem;
		margin-bottom: 1rem;
	}
	@media (min-width: 768px) {
		.slides {
			--slides-per-view: 3;
			--slides-gap: 0.6667rem;
			--slides-icon: 0.8333rem;
		}
	}
	.viewport {
		overflow: clip;
		background: var(--paper);
		user-select: none;
		-webkit-user-select: none;
		-webkit-touch-callout: none;
	}
	.viewport.movable {
		cursor: grab;
	}
	.viewport.dragging {
		cursor: grabbing;
	}
	.track {
		display: flex;
		will-change: transform;
	}
	/* one pitch wide with the gap inside, after the image: the gap after the last slide in view falls outside the
	   window; the JS sets the exact width on the device grid, a looping slide's `left` */
	.slide {
		position: relative;
		flex: none;
		box-sizing: border-box;
		width: calc((100% + var(--slides-gap)) / var(--slides-per-view));
		padding-inline-end: var(--slides-gap);
		display: flex;
		align-items: center;
		justify-content: center;
	}
	.slide img {
		display: block;
		width: 100%;
		height: auto;
		object-fit: contain;
		-webkit-user-drag: none;
	}
	/* grouped controls are square (DESIGN "Round vs square"), the borders shared */
	.controls {
		display: flex;
		justify-content: center;
		margin-top: 0.5rem;
	}
	.controls button {
		display: inline-grid;
		place-items: center;
		box-sizing: border-box;
		width: var(--control-height);
		height: var(--control-height);
		margin-left: calc(-1 * var(--border-width-regular));
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		cursor: pointer;
		touch-action: manipulation;
	}
	.controls button:first-child {
		margin-left: 0;
	}
	.controls button:hover {
		background: var(--ink);
		color: var(--paper);
	}
	.controls button :global(.icon) {
		width: var(--slides-icon);
		height: var(--slides-icon);
	}
	.controls button:focus-visible {
		position: relative;
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
</style>
