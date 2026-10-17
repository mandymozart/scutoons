<script lang="ts">
	import { untrack } from 'svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import { decodeImages } from '$lib/components/toon/renderer/strip-dom.ts';
	import { slidesPager, type SlidesPager, type SlidesState } from './slides-pager.ts';

	/**
	 * An edition's images on the landing page (DESIGN.md "Landing page"): 1 per view below 768px, 3 from 768px, looping,
	 * dragged sideways, one slide on every 5 s – held while the reader hovers, touches, drags or focuses it, on again after
	 * (slides-pager.ts – the endless band's utility, owner 2026-10-17). No buttons under the picture (owner 2026-10-17):
	 * the viewport takes focus and ← / → move it. Without JS the CSS alone shows the first slides.
	 */
	let { images, label }: { images: string[]; label: string } = $props();

	let region = $state<HTMLElement>();
	let viewport = $state<HTMLElement>();
	let track = $state<HTMLElement>();
	let view = $state.raw<SlidesState>({ index: 0, count: 0, perView: 1, movable: false, playing: false, dragging: false });
	/** the slider is near the screen: every image loads now and is decoded (a slide comes in by a drag, not the scroll) */
	let near = $state(false);
	let pager: SlidesPager | undefined;

	$effect(() => {
		if (!region || !viewport || !track) return;
		const made = slidesPager(region, viewport, track, { autoplay: 5000, onstate: (s) => (view = s) });
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
	<!-- focusable so ← / → reach the slider without buttons (strip-drag.ts listens on the viewport) -->
	<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
	<div class="viewport" class:movable={view.movable} class:dragging={view.dragging} tabindex={view.movable ? 0 : undefined} bind:this={viewport}>
		<div class="track" bind:this={track}>
			{#each images as image, i (image)}
				<div class="slide" role="group" aria-roledescription={m['home.slide_role']()} aria-label={m['home.slide_of']({ number: i + 1, count: images.length })}>
					<img src={asset(image)} alt={label} loading={near ? 'eager' : 'lazy'} draggable="false" />
				</div>
			{/each}
		</div>
	</div>
</div>

<style>
	/* the layout options live here: slides-pager.ts reads them (per view a number, the gap in rem): the gap is 1rem below
	   768px (14px up to 480px, 16px above – shared/tokens/base.css), 16px from 768px (the root is 24px there) */
	.slides {
		--slides-per-view: 1;
		--slides-gap: 1rem;
		margin-bottom: 1rem;
	}
	@media (min-width: 768px) {
		.slides {
			--slides-per-view: 3;
			--slides-gap: 0.6667rem;
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
	.viewport:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
</style>
