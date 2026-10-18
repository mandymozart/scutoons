<script lang="ts">
	import { untrack } from 'svelte';
	import { on } from 'svelte/events';
	import type { Group } from '@scutoons/content';
	import { DEFAULT_SLICE } from '@scutoons/content';
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import Slice from '../slice.svelte';
	import { ratioSum, stripFrames, stripOffset, stripScroll } from './strip-scroll.ts';
	import { nearScreen, viewportWidth } from './strip-dom.ts';

	/**
	 * A horizontal group (owner 2026-10-06; an `endless` one is strip-endless.svelte): the slices side by side as one
	 * strip, each at the screen's full height (its own width from its ratio), out of the reading column across the width.
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
	/**
	 * how far the page scrolls while the strip is pinned (section − pin), px. The section's height comes from CSS (the
	 * frames' ratios × the large viewport height), never from `travel`: the track is drawn at the visible height (dvh),
	 * which changes when the mobile toolbar slides in or out – a section height following it made the page above the
	 * reader grow / shrink by hundreds of px per strip and the scroll jumped (owner 2026-10-10, Firefox Android)
	 */
	let range = 0;
	/** how far it has moved, px */
	let offset = $state(0);
	let fullscreen = $state(false);
	let canFullscreen = $state(false);
	/** within a screen of the viewport: every frame loads now and mounts its pieces (strip-dom.ts nearScreen) */
	let near = $state(false);

	const frames = $derived(stripFrames(group.slices ?? [], DEFAULT_SLICE));

	/** the frames' width / height ratios summed (the section's height in CSS) */
	const ratios = $derived(ratioSum(frames));

	function measure() {
		if (!pin || !track || !section) return;
		const fraction = travel > 0 ? offset / travel : 0;
		travel = Math.max(0, track.scrollWidth - pin.clientWidth);
		range = Math.max(0, section.offsetHeight - pin.offsetHeight);
		offset = fraction * travel;
		if (!fullscreen) follow();
	}

	/** on the page: the scroll position decides the offset */
	function follow() {
		if (!section || fullscreen) return;
		offset = stripOffset(-section.getBoundingClientRect().top, range, travel);
	}

	$effect(() => {
		if (!pin || !track) return;
		const observer = new ResizeObserver(measure);
		observer.observe(pin);
		observer.observe(track);
		if (section) observer.observe(section);
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

	/**
	 * The whole page goes full screen and the strip covers it (`.pin.fullscreen`) – not the strip element alone: only the
	 * full-screen element is drawn, so a reader effect, a worn tint, a VN message stayed outside (owner 2026-10-13: "the
	 * LSD effect is not applied … a way to cheat the LSD").
	 */
	let mine = false;
	async function toggleFullscreen() {
		try {
			if (document.fullscreenElement) await document.exitFullscreen();
			else {
				mine = true;
				await document.documentElement.requestFullscreen();
			}
		} catch {
			// refused (e.g. an embedded browser): the strip stays on the page
			mine = false;
		}
	}

	$effect(() => {
		const onChange = () => {
			const was = fullscreen;
			fullscreen = mine && document.fullscreenElement === document.documentElement;
			if (!document.fullscreenElement) mine = false;
			if (was && !fullscreen) {
				// back on the page: scroll to where the strip was
				requestAnimationFrame(() => {
					measure();
					if (section) window.scrollTo({ top: window.scrollY + section.getBoundingClientRect().top + stripScroll(offset, range, travel), behavior: 'instant' });
				});
			}
		};
		document.addEventListener('fullscreenchange', onChange);
		return () => document.removeEventListener('fullscreenchange', onChange);
	});

	function onWheel(event: WheelEvent) {
		// the page's scroll is stopped (a VN message waits: freeze-scroll.ts)
		if (event.defaultPrevented && !fullscreen) return;
		if (fullscreen) {
			event.preventDefault();
			move(offset + event.deltaY + event.deltaX);
			return;
		}
		// on the page the page's scroll moves the strip (down → right, up → left); a sideways wheel / trackpad swipe
		// (or Shift + wheel) moves it too – by scrolling the page the same distance (owner 2026-10-06, BACKLOG M.8)
		if (travel === 0 || Math.abs(event.deltaX) <= Math.abs(event.deltaY)) return;
		event.preventDefault();
		const px = event.deltaX * (event.deltaMode === 1 ? 16 : event.deltaMode === 2 ? window.innerHeight : 1);
		window.scrollBy({ top: (px * range) / travel, behavior: 'instant' });
	}

	/** the strip fills the window (pinned) – the page's scroll is the strip's */
	function pinned() {
		const r = section?.getBoundingClientRect();
		return !!r && travel > 0 && r.top <= 1 && r.bottom >= window.innerHeight - 1;
	}
	const editable = (t: EventTarget | null) => t instanceof HTMLElement && (t.isContentEditable || /^(INPUT|TEXTAREA|SELECT)$/.test(t.tagName));

	function onKey(event: KeyboardEvent) {
		if (!fullscreen) {
			if (event.defaultPrevented) return;
			// pinned on the page: ← / → step frame by frame (the page scrolls to the frame's edge; ↑ / ↓ stay the page's)
			if ((event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') || event.altKey || event.ctrlKey || event.metaKey || editable(event.target) || !pinned()) return;
			// where the strip is now (the scroll position itself – `offset` follows it a frame later)
			const now = stripOffset(-(section?.getBoundingClientRect().top ?? 0), range, travel);
			const at = edges();
			const next = event.key === 'ArrowRight' ? at.find((x) => x > now + 1) : [...at].reverse().find((x) => x < now - 1);
			const to = Math.min(Math.max(next ?? (event.key === 'ArrowRight' ? travel : 0), 0), travel);
			const smooth = !matchMedia('(prefers-reduced-motion: reduce)').matches;
			window.scrollBy({ top: stripScroll(to, range, travel) - stripScroll(now, range, travel), behavior: smooth ? 'smooth' : 'instant' });
			event.preventDefault();
			return;
		}
		if (event.key === 'ArrowRight' || event.key === 'PageDown' || event.key === ' ') step(1);
		else if (event.key === 'ArrowLeft' || event.key === 'PageUp') step(-1);
		else if (event.key === 'Home') move(0);
		else if (event.key === 'End') move(travel);
		else return;
		event.preventDefault();
	}
</script>

<svelte:window onkeydown={onKey} />

<section class="strip" data-group-label={group.label} bind:this={section} style:--ratios={ratios} {@attach viewportWidth} {@attach nearScreen(() => (near = true))}>
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<!-- data-fx-screen: in full screen a screen shake / punch moves the strip itself (fx/cues.ts onScreen) -->
	<div class="pin" class:fits={travel === 0} class:fullscreen data-fx-screen={fullscreen || undefined} bind:this={pin} {@attach (el) => on(el, 'wheel', onWheel, { passive: false })}>
		<div class="track" bind:this={track} style:transform="translate3d({-offset}px, 0, 0)">
			{#each frames as frame, index}
				<div class="frame" style:aspect-ratio="{frame.width} / {frame.height}">
					<Slice slice={frame.slice} sliceIndex={index} pageIndex={sliceOffset + index} allSlices={group.slices ?? []} preload={near} />
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
	   screen plus the sideways travel, so scrolling through it moves the pinned strip (about 1:1).
	   The height is the LARGE viewport's (lvh; vh is the same on phones) and the travel at that height, from the frames'
	   ratios – stable from the first paint (no measuring) and never following the mobile toolbar: a dvh height here
	   changed every strip above by ≈ (1 + ratios) × the toolbar's height whenever it slid in, the page jumped while
	   scrolling back up (owner 2026-10-10; it came with the svh → dvh change of 2026-10-08). Only the pin follows the
	   visible area (dvh): once the toolbar slides away the visible area grows – an svh pin left a black bar under it
	   (owner 2026-10-08). The track's travel at the visible height is mapped onto this range (strip-scroll.ts). */
	.strip {
		width: var(--viewport-width, 100vw);
		margin-left: calc(50% - var(--viewport-width, 100vw) / 2);
		height: calc(100vh + max(0px, var(--ratios, 0) * 100vh - var(--viewport-width, 100vw)));
		height: calc(100lvh + max(0px, var(--ratios, 0) * 100lvh - var(--viewport-width, 100vw)));
		background: var(--ink, #000);
	}
	.pin {
		position: sticky;
		top: 0;
		height: 100vh;
		height: 100dvh;
		overflow: hidden;
		background: var(--ink, #000);
	}
	/* full screen: over the page (its menus, the VN box, a reader effect stay above – z-index 100 and more) */
	.pin.fullscreen {
		position: fixed;
		inset: 0;
		z-index: 50;
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
