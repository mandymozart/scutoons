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
	/**
	 * One catalog item placed on a slice (`Slice.items[index]`): the item's cut-out icon at its rect, rotated around its
	 * centre, inside the slice's layer stack (`SliceLayers`). With `ontap` and an `action` other than `none` it is a
	 * button, else just art. The app resolves the icon (`info`).
	 */
	let {
		item,
		index,
		info,
		ontap
	}: {
		item: SliceItem;
		index: number;
		info: SliceItemInfo;
		ontap?: (item: SliceItem, index: number) => void;
	} = $props();

	const tappable = $derived(!!ontap && (item.action ?? 'none') !== 'none');
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
		onclick={() => ontap?.(item, index)}
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

	@media (prefers-reduced-motion: reduce) {
		.tappable img {
			animation: none;
			filter: drop-shadow(0 0 0.12em rgba(0, 0, 0, 0.3)) drop-shadow(0 0 0.35em rgba(255, 255, 255, 0.85));
		}
	}
</style>
