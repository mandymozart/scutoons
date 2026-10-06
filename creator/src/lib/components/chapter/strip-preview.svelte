<script lang="ts">
	import { DEFAULT_SLICE, type Path, type Slice as SliceModel } from '@scutoons/content';
	import { Slice } from '@scutoons/reader';
	import { Segmented } from '@scutoons/ui';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { replacedImages } from '$lib/editor/replaced-images.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The preview of a full-width group (`renderer.params.fullWidth`, owner 2026-10-06): one reader screen – a phone or a
	 * desktop – with the strip in it as the reader shows it: every slice at the screen's full height, side by side, the
	 * text boxes and placed items drawn by the reader's own `Slice`. Scrolled sideways here – the mouse wheel down moves
	 * it sideways like the reader's page scroll (owner 2026-10-06); at either end the page scrolls on. A click on a frame
	 * opens that slice.
	 */
	let { doc, groupPath, locale, onopen }: { doc: ContentDoc; groupPath: Path; locale: string; onopen: (index: number) => void } = $props();

	type Screen = 'phone' | 'desktop';
	/** the screens: a common phone (390 × 844) and a desktop window (16 : 9) */
	const RATIOS: Record<Screen, number> = { phone: 390 / 844, desktop: 16 / 9 };
	let screen = $state<Screen>('phone');

	const slices = $derived<SliceModel[]>(doc.resolvedGroup(groupPath, locale)?.slices ?? []);
	const src = (image: string) => replacedImages.src(asset(`${doc.toon}/${image}`), `${doc.toon}/${image}`);
	let screenEl = $state<HTMLElement>();
	/**
	 * The vertical wheel scrolls the strip sideways while the pointer is over it (horizontal deltas keep their native
	 * scroll); at the start / end the event is left alone, so the page scrolls on. Not passive: it prevents the page scroll.
	 */
	$effect(() => {
		const el = screenEl;
		if (!el) return;
		const onwheel = (e: WheelEvent) => {
			if (e.ctrlKey || Math.abs(e.deltaY) <= Math.abs(e.deltaX)) return;
			// lines / pages into pixels (Firefox reports lines)
			const dy = e.deltaMode === 1 ? e.deltaY * parseFloat(getComputedStyle(el).fontSize) : e.deltaMode === 2 ? e.deltaY * el.clientWidth : e.deltaY;
			const max = el.scrollWidth - el.clientWidth;
			if ((dy < 0 && el.scrollLeft <= 0) || (dy > 0 && el.scrollLeft >= max - 1)) return;
			e.preventDefault();
			el.scrollLeft = Math.max(0, Math.min(max, el.scrollLeft + dy));
		};
		el.addEventListener('wheel', onwheel, { passive: false });
		return () => el.removeEventListener('wheel', onwheel);
	});
	const ratio = (slice: SliceModel) => `${slice.width > 0 ? slice.width : DEFAULT_SLICE.width} / ${slice.height > 0 ? slice.height : DEFAULT_SLICE.height}`;
</script>

<section class="strip-preview" data-testid="strip-preview">
	<div class="head">
		<h3 class="sub">{m['creator.strip_preview']()}</h3>
		<Segmented
			label={m['creator.strip_preview_screen']()}
			value={screen}
			onchange={(next) => (screen = next)}
			options={[
				{ value: 'phone', label: m['creator.strip_preview_phone'](), testid: 'strip-preview-phone' },
				{ value: 'desktop', label: m['creator.strip_preview_desktop'](), testid: 'strip-preview-desktop' }
			]}
		/>
	</div>
	<div class="screen" style:--ratio={RATIOS[screen]} bind:this={screenEl} data-testid="strip-preview-screen">
		<div class="track">
			{#each slices as slice, index}
				<button type="button" class="frame" style:aspect-ratio={ratio(slice)} onclick={() => onopen(index)} title={m['creator.strip_preview_open']({ number: index + 1 })} data-control-exempt="strip preview frame">
					{#if slice?.image}<Slice {slice} src={src(slice.image)} lang={locale} />{/if}
					<span class="no">{index + 1}</span>
				</button>
			{/each}
		</div>
	</div>
	<p class="sc-caption">{m['creator.strip_preview_hint']()}</p>
</section>

<style>
	.strip-preview {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		margin-top: var(--space-1);
	}
	.head {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: var(--space-1);
		flex-wrap: wrap;
	}
	.head h3 {
		margin: 0;
	}
	/* one reader screen: as high as the panel allows (at most 32rem), its width from the screen's ratio */
	.screen {
		width: min(100%, calc(min(32rem, 60vh) * var(--ratio)));
		aspect-ratio: var(--ratio);
		overflow-x: auto;
		overflow-y: hidden;
		background: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
	}
	.track {
		display: flex;
		height: 100%;
		width: max-content;
	}
	/* each slice at the screen's full height, its width from its ratio (text boxes scale with it: cqw) */
	.frame {
		position: relative;
		flex: none;
		height: 100%;
		padding: 0;
		border: 0;
		background: transparent;
		cursor: pointer;
		display: block;
	}
	.frame :global(.slice) {
		min-height: 0;
		pointer-events: none;
	}
	.frame:hover,
	.frame:focus-visible {
		outline: var(--border-width-focus) solid var(--cyan);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	.no {
		position: absolute;
		top: var(--space-025);
		left: var(--space-025);
		padding: 0 var(--space-025);
		background: var(--ink);
		color: var(--paper);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
</style>
