<script lang="ts">
	import { untrack } from 'svelte';
	import type { Group } from '@scutoons/content';
	import { DEFAULT_SLICE } from '@scutoons/content';
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import Slice from '../slice.svelte';

	/**
	 * A horizontal group with `renderer.params.fullWidth` (owner 2026-10-06): the slices side by side as one strip, each
	 * at the screen's full height (its own width from its ratio), out of the reading column across the whole width.
	 * When the strip reaches the top it is pinned (sticky) and scrolling down moves it sideways – 1:1, no snapping, no
	 * hint, no gesture of its own – then reading goes on downwards; scrolling up runs it backwards. A strip narrower than
	 * the screen is shown whole, centred, and not pinned. On a desktop (fine pointer) the strip can go full screen: ← →
	 * (frame by frame), the wheel and the round arrows move it there, Esc goes back to the same place on the page.
	 * It finds its offset from its own place on the page, so it works anywhere in a chain of linked groups too.
	 * `sliceOffset`: the page index of the first slice (linked groups share one page).
	 */
	let { group, sliceOffset = 0 }: { group: Group; sliceOffset?: number } = $props();

	let section = $state<HTMLElement>();
	let pin = $state<HTMLElement>();
	let track = $state<HTMLElement>();
	/** how far the strip can move sideways (track width − visible width), px */
	let travel = $state(0);
	/** how far it has moved, px */
	let offset = $state(0);
	let fullscreen = $state(false);
	let canFullscreen = $state(false);

	const frames = $derived(
		(group.slices ?? []).map((slice) => ({
			slice,
			width: slice.width > 0 ? slice.width : DEFAULT_SLICE.width,
			height: slice.height > 0 ? slice.height : DEFAULT_SLICE.height
		}))
	);

	function measure() {
		if (!pin || !track) return;
		const fraction = travel > 0 ? offset / travel : 0;
		travel = Math.max(0, track.scrollWidth - pin.clientWidth);
		offset = fraction * travel;
		if (!fullscreen) follow();
	}

	/** on the page: the scroll position decides the offset */
	function follow() {
		if (!section || fullscreen) return;
		offset = Math.min(Math.max(-section.getBoundingClientRect().top, 0), travel);
	}

	$effect(() => {
		if (!pin || !track) return;
		const observer = new ResizeObserver(measure);
		observer.observe(pin);
		observer.observe(track);
		let frame = 0;
		const onScroll = () => {
			if (frame) return;
			frame = requestAnimationFrame(() => {
				frame = 0;
				follow();
			});
		};
		window.addEventListener('scroll', onScroll, { passive: true });
		canFullscreen = document.fullscreenEnabled && matchMedia('(hover: hover) and (pointer: fine)').matches;
		// the setup depends on the elements only (measure reads and writes travel / offset)
		untrack(measure);
		return () => {
			observer.disconnect();
			window.removeEventListener('scroll', onScroll);
			if (frame) cancelAnimationFrame(frame);
		};
	});

	// ── full screen (desktop) ─────────────────────────────────────────────────────────────────────────────
	const move = (to: number) => (offset = Math.min(Math.max(to, 0), travel));

	/** the left edges of the frames (px, in the track) */
	const edges = () => [...(track?.children ?? [])].map((el) => (el as HTMLElement).offsetLeft);
	function step(direction: 1 | -1) {
		const at = edges();
		const next = direction > 0 ? at.find((x) => x > offset + 1) : [...at].reverse().find((x) => x < offset - 1);
		move(next ?? (direction > 0 ? travel : 0));
	}

	async function toggleFullscreen() {
		try {
			if (document.fullscreenElement) await document.exitFullscreen();
			else await pin?.requestFullscreen();
		} catch {
			// refused (e.g. an embedded browser): the strip stays on the page
		}
	}

	$effect(() => {
		const onChange = () => {
			const was = fullscreen;
			fullscreen = document.fullscreenElement === pin && !!pin;
			if (was && !fullscreen) {
				// back on the page: scroll to where the strip was
				requestAnimationFrame(() => {
					measure();
					const fraction = travel > 0 ? offset / travel : 0;
					if (section) window.scrollTo({ top: window.scrollY + section.getBoundingClientRect().top + fraction * travel, behavior: 'instant' });
				});
			}
		};
		document.addEventListener('fullscreenchange', onChange);
		return () => document.removeEventListener('fullscreenchange', onChange);
	});

	function onWheel(event: WheelEvent) {
		if (!fullscreen) return;
		event.preventDefault();
		move(offset + event.deltaY + event.deltaX);
	}

	function onKey(event: KeyboardEvent) {
		if (!fullscreen) return;
		if (event.key === 'ArrowRight' || event.key === 'PageDown' || event.key === ' ') step(1);
		else if (event.key === 'ArrowLeft' || event.key === 'PageUp') step(-1);
		else if (event.key === 'Home') move(0);
		else if (event.key === 'End') move(travel);
		else return;
		event.preventDefault();
	}
</script>

<svelte:window onkeydown={onKey} />

<section class="strip" data-group-label={group.label} bind:this={section} style:--travel="{travel}px">
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div class="pin" class:fits={travel === 0} class:fullscreen bind:this={pin} onwheel={onWheel}>
		<div class="track" bind:this={track} style:transform="translate3d({-offset}px, 0, 0)">
			{#each frames as frame, index}
				<div class="frame" style:aspect-ratio="{frame.width} / {frame.height}">
					<Slice slice={frame.slice} sliceIndex={index} pageIndex={sliceOffset + index} allSlices={group.slices ?? []} />
				</div>
			{/each}
		</div>
		{#if canFullscreen}
			<div class="controls">
				{#if fullscreen}
					<button type="button" class="round" onclick={() => step(-1)} disabled={offset <= 0} aria-label={m['chapter.strip_prev']()} title={m['chapter.strip_prev']()}>
						<Icon name="chevron-left" />
					</button>
					<button type="button" class="round" onclick={() => step(1)} disabled={offset >= travel} aria-label={m['chapter.strip_next']()} title={m['chapter.strip_next']()}>
						<Icon name="chevron-right" />
					</button>
				{/if}
				<button
					type="button"
					class="round"
					onclick={toggleFullscreen}
					aria-label={fullscreen ? m['chapter.strip_fullscreen_exit']() : m['chapter.strip_fullscreen']()}
					title={fullscreen ? m['chapter.strip_fullscreen_exit']() : m['chapter.strip_fullscreen']()}
				>
					<Icon name={fullscreen ? 'minimize' : 'maximize'} />
				</button>
			</div>
		{/if}
	</div>
</section>

<style>
	/* the whole width of the viewport, out of the reading column (like a full-viewport video slice); as tall as the
	   screen plus the sideways travel, so scrolling through it moves the pinned strip 1:1 */
	.strip {
		width: var(--viewport-width, 100vw);
		margin-left: calc(50% - var(--viewport-width, 100vw) / 2);
		height: calc(100svh + var(--travel, 0px));
		background: var(--ink, #000);
	}
	.pin {
		position: sticky;
		top: 0;
		height: 100vh;
		height: 100svh;
		overflow: hidden;
		background: var(--ink, #000);
	}
	.pin.fullscreen {
		height: 100vh;
	}
	.track {
		display: flex;
		height: 100%;
		width: max-content;
		will-change: transform;
	}
	/* narrower than the screen: the whole strip, centred */
	.pin.fits .track {
		width: 100%;
		justify-content: center;
		transform: none !important;
	}
	/* each slice at the full height, its width from its ratio (the slice's text boxes scale with its width: cqw) */
	.frame {
		flex: none;
		height: 100%;
	}
	.frame :global(.slice) {
		min-height: 0;
	}
	.controls {
		position: absolute;
		right: 1rem;
		bottom: 1rem;
		display: flex;
		gap: 0.5rem;
		opacity: 0.4;
		transition: opacity 0.2s;
	}
	.pin:hover .controls,
	.controls:focus-within {
		opacity: 1;
	}
	.round {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 2.75rem;
		height: 2.75rem;
		padding: 0;
		border-radius: 50%;
		border: var(--border-width-regular, 2px) solid var(--paper, #fff);
		background: var(--ink, #000);
		color: var(--paper, #fff);
		cursor: pointer;
	}
	.round:disabled {
		opacity: 0.3;
		cursor: default;
	}
</style>
