<script lang="ts" module>
	import type { SliceItem } from '@scutoons/content';

	/** What the app knows about a placed item's catalog entry: its icon URL (resolved) and its name. */
	export interface SliceItemInfo {
		src: string;
		label: string;
	}

	export type ResolveSliceItem = (item: SliceItem) => SliceItemInfo | null | undefined;
</script>

<script lang="ts">
	import { hitsShape, loadShape, type Shape } from '../fx/shape.ts';

	/**
	 * One catalog item placed on a slice (`Slice.items[index]`): the item's cut-out icon at its rect, rotated around its
	 * centre, inside the slice's layer stack (`SliceLayers`). With `ontap` and an `action` other than `none` it is a
	 * button, else just art. The app resolves the icon (`info`).
	 *
	 * Its shape (agents/FX.md): a tap on a transparent pixel of the cut-out misses (0.5rem tolerance; an icon whose
	 * alpha can't be read counts as its box). `ontap` gets the element and the shape for the app's cues; `onidle` fires
	 * once when it has been on screen (60 %) for 4 s without a tap – the app nudges.
	 */
	let {
		item,
		index,
		info,
		ontap,
		onidle
	}: {
		item: SliceItem;
		index: number;
		info: SliceItemInfo;
		ontap?: (item: SliceItem, index: number, el: HTMLElement, shape: Shape | null) => void;
		onidle?: (item: SliceItem, index: number, el: HTMLElement, shape: Shape | null) => void;
	} = $props();

	const tappable = $derived(!!ontap && (item.action ?? 'none') !== 'none');
	let button = $state<HTMLButtonElement>();
	let shape = $state<Shape | null>(null);
	let tapped = false;

	$effect(() => {
		if (!tappable) return;
		let live = true;
		void loadShape(info.src).then((s) => live && (shape = s));
		return () => (live = false);
	});

	const IDLE_MS = 4000;
	$effect(() => {
		const el = button;
		if (!el || !onidle || typeof IntersectionObserver === 'undefined') return;
		let timer: ReturnType<typeof setTimeout> | undefined;
		const observer = new IntersectionObserver(
			([entry]) => {
				clearTimeout(timer);
				if (entry.intersectionRatio >= 0.6 && !tapped)
					timer = setTimeout(() => {
						if (!tapped) onidle?.(item, index, el, shape);
						observer.disconnect();
					}, IDLE_MS);
			},
			{ threshold: [0, 0.6] }
		);
		observer.observe(el);
		return () => {
			clearTimeout(timer);
			observer.disconnect();
		};
	});

	function tap(event: MouseEvent) {
		const el = button!;
		// a real pointer (not Enter / Space) on a transparent pixel: missed
		if (shape && event.detail > 0) {
			const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
			if (!hitsShape(el, shape, item.rotation ?? 0, event.clientX, event.clientY, 0.5 * rem)) return;
		}
		tapped = true;
		ontap?.(item, index, el, shape);
	}
</script>

{#if tappable}
	<button
		type="button"
		class="slice-item tappable"
		data-item={item.item}
		data-index={index}
		style:left="{item.rect.x}%"
		style:top="{item.rect.y}%"
		style:width="{item.rect.width}%"
		style:height={item.rect.height !== undefined ? `${item.rect.height}%` : null}
		style:--rotation="{item.rotation ?? 0}deg"
		aria-label={info.label}
		bind:this={button}
		onclick={tap}
	>
		<img src={info.src} alt="" draggable="false" />
	</button>
{:else}
	<img
		class="slice-item"
		data-item={item.item}
		data-index={index}
		src={info.src}
		alt={info.label}
		draggable="false"
		style:left="{item.rect.x}%"
		style:top="{item.rect.y}%"
		style:width="{item.rect.width}%"
		style:height={item.rect.height !== undefined ? `${item.rect.height}%` : null}
		style:--rotation="{item.rotation ?? 0}deg"
	/>
{/if}

<style>
	.slice-item {
		position: absolute;
		box-sizing: border-box;
		display: block;
		transform: rotate(var(--rotation));
		object-fit: contain;
	}

	img.slice-item {
		height: auto;
	}

	.tappable {
		margin: 0;
		padding: 0;
		border: 0;
		background: none;
		cursor: pointer;
		pointer-events: auto;
		-webkit-tap-highlight-color: transparent;
	}

	.tappable:focus-visible {
		outline: 3px solid #000000;
		outline-offset: 2px;
	}

	.tappable img {
		display: block;
		width: 100%;
		height: 100%;
		object-fit: contain;
	}

	/* a slight hint that it can be tapped (owner 2026-10-03): a slow glow that comes and goes; still with reduced motion */
	.tappable img {
		animation: tap-hint 3.2s ease-in-out infinite;
	}

	@keyframes tap-hint {
		0%,
		100% {
			filter: drop-shadow(0 0 0 rgba(255, 255, 255, 0));
			transform: scale(1);
		}
		50% {
			filter: drop-shadow(0 0 0.12em rgba(0, 0, 0, 0.35)) drop-shadow(0 0 0.45em rgba(255, 255, 255, 0.95));
			transform: scale(1.03);
		}
	}

	/* "Show hints" (account): the contour of the cut-out, not its box (agents/FX.md) */
	:global(html.sc-show-hints) .tappable img {
		animation: none;
		filter: drop-shadow(1px 0 0 #00d1ff) drop-shadow(-1px 0 0 #00d1ff) drop-shadow(0 1px 0 #00d1ff) drop-shadow(0 -1px 0 #00d1ff)
			drop-shadow(0 0 0.35em rgba(0, 209, 255, 0.6));
	}

	@media (prefers-reduced-motion: reduce) {
		.tappable img {
			animation: none;
			filter: drop-shadow(0 0 0.12em rgba(0, 0, 0, 0.3)) drop-shadow(0 0 0.35em rgba(255, 255, 255, 0.85));
		}
	}
</style>
