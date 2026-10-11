<script lang="ts">
	import { DEFAULT_PLACE_ANCHOR, DEFAULT_PLACE_IMAGE_SIZE, placeImagePath, type MapPlace } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { cdnHas, cdnSize, editPlace, numberOrUnset } from '$lib/editor/map.ts';
	import { MediaVariantsSlot, type VariantMedia } from '@scutoons/media';

	/**
	 * A place's illustration (agents/MAP.md §4.3): a slot with the asset picker (owner 2026-10-03; uploads go to the place
	 * illustrations' folder `<toon>/map/places/`), its size on the map, the ground point (drag on the picture) and
	 * previews at 50 / 100 / 200 %, with warnings for a missing, large or blurry file. `bust` reloads after an import.
	 */
	let { doc, place, bust = 0 }: { doc: ContentDoc; place: MapPlace; bust?: number } = $props();

	let natural = $state<{ w: number; h: number } | null>(null);
	let imageState = $state<{ missing: boolean; kb: number | null }>({ missing: false, kb: null });
	$effect(() => {
		const src = place.image?.src;
		natural = null;
		imageState = { missing: false, kb: null };
		if (!src) return;
		cdnHas(src).then((ok) => {
			if (place.image?.src === src) imageState = { ...imageState, missing: !ok };
		});
		cdnSize(src).then((bytes) => {
			if (place.image?.src === src) imageState = { ...imageState, kb: bytes === null ? null : Math.round(bytes / 1024) };
		});
	});
	// ── the painted image and the plaster model, each with its 1x / 2x / 3x files (owner 2026-10-11: 256 / 512 / 1024) –
	//    one resolution slot each, like a slice's background: 1x = `src` / `plaster`, the rest in `variants` ──────────
	const media = (src: string | undefined, variants: Record<string, string> | undefined): VariantMedia => ({ src: src || undefined, variants: variants ?? (src ? { low: src } : undefined) });
	const paintedMedia = $derived(media(place.image?.src, place.image?.variants));
	const plasterMedia = $derived(media(place.image?.plaster, place.image?.plasterVariants));
	function setPainted(next: VariantMedia) {
		const src = next.variants?.low ?? next.src;
		const extra = next.variants?.medium || next.variants?.high ? next.variants : undefined;
		editPlace(doc, place.id, (p) => {
			if (!src) return void delete p.image;
			p.image = { ...(p.image ?? {}), src };
			if (extra) p.image.variants = extra;
			else delete p.image.variants;
		});
	}
	/** the plaster model (owner 2026-10-11): the unlock reveal fades from it to the painted image – same framing */
	function setPlaster(next: VariantMedia) {
		const src = next.variants?.low ?? next.src;
		const extra = next.variants?.medium || next.variants?.high ? next.variants : undefined;
		editPlace(doc, place.id, (p) => {
			if (!p.image) return;
			if (!src) {
				delete p.image.plaster;
				delete p.image.plasterVariants;
				return;
			}
			p.image.plaster = src;
			if (extra) p.image.plasterVariants = extra;
			else delete p.image.plasterVariants;
		});
	}
	function setImageField(field: 'size', value: number | undefined) {
		if (!place.image) return;
		editPlace(doc, place.id, (p) => {
			if (!p.image) return;
			if (value === undefined || !Number.isFinite(value)) delete p.image[field];
			else p.image[field] = value;
		}, `image-${field}`);
	}
	let anchorBox = $state<HTMLDivElement>();
	let anchorDrag = false;
	function setAnchor(e: PointerEvent) {
		if (!place.image || !anchorBox) return;
		const r = anchorBox.getBoundingClientRect();
		const x = Math.round(Math.max(0, Math.min(100, ((e.clientX - r.left) / r.width) * 100)));
		const y = Math.round(Math.max(0, Math.min(100, ((e.clientY - r.top) / r.height) * 100)));
		editPlace(doc, place.id, (p) => {
			if (!p.image) return;
			if (x === DEFAULT_PLACE_ANCHOR.x && y === DEFAULT_PLACE_ANCHOR.y) delete p.image.anchor;
			else p.image.anchor = { x, y };
		}, 'image-anchor');
	}
	const anchor = $derived(place.image?.anchor ?? DEFAULT_PLACE_ANCHOR);
	const size = $derived(place.image?.size ?? DEFAULT_PLACE_IMAGE_SIZE);
	/** blurry at 200 %: the file is narrower than twice its drawn width there */
	const blurry = $derived(!!natural && natural.w < size * 2);
</script>

<fieldset data-testid="place-image">
	<legend>{m['creator.map_image']()}</legend>
	<!-- a slot with the asset picker (owner 2026-10-03); uploads go to the place illustrations' folder (`<toon>/map/places/`) -->
	<div class="slots">
		<MediaVariantsSlot toon="" kind="image" label={m['creator.map_image']()} value={paintedMedia} place={placeImagePath(place).replace(/[^/]+$/, '')} onchange={setPainted} testid="place-image-slot" />
		{#if place.image?.src}
			<!-- the plaster model: the same diorama unpainted – the reader's unlock reveal fades from it to the painted one -->
			<MediaVariantsSlot toon="" kind="image" label={m['creator.map_image_plaster']()} value={plasterMedia} place={placeImagePath(place).replace(/[^/]+$/, '')} onchange={setPlaster} testid="place-plaster-slot" />
		{/if}
	</div>
	{#if place.image?.src}<p class="hint">{m['creator.map_image_plaster_hint']()}</p>{/if}
	{#if place.image?.src}
		<p class="hint">
			{#if imageState.missing}<span class="bad">{m['creator.map_image_missing']()}</span>{/if}
			{#if imageState.kb !== null}{imageState.kb} KB{/if}
			{#if imageState.kb !== null && imageState.kb > 60}<span class="warn"> · {m['creator.map_image_large']({ kb: imageState.kb })}</span>{/if}
			{#if blurry}<span class="warn"> · {m['creator.map_image_blurry']({ px: size * 2 })}</span>{/if}
		</p>
		<label class="field small"><span>{m['creator.map_image_size']()}</span><input type="number" min="8" value={place.image.size ?? ''} placeholder={String(DEFAULT_PLACE_IMAGE_SIZE)} onchange={(e) => setImageField('size', numberOrUnset(e.currentTarget.value))} data-testid="place-image-size" /></label>
		<span class="lbl">{m['creator.map_image_anchor']()} ({anchor.x} / {anchor.y} %)</span>
		<div
			class="anchor checker"
			bind:this={anchorBox}
			role="slider"
			tabindex="0"
			aria-label={m['creator.map_image_anchor']()}
			aria-valuenow={anchor.y}
			title={m['creator.map_image_anchor_hint']()}
			onpointerdown={(e) => { anchorDrag = true; (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId); setAnchor(e); }}
			onpointermove={(e) => anchorDrag && setAnchor(e)}
			onpointerup={() => (anchorDrag = false)}
			data-testid="place-anchor"
		>
			<img src={asset(place.image.src) + (bust ? `?v=${bust}` : '')} alt="" draggable="false" onload={(e) => { const img = e.currentTarget as HTMLImageElement; natural = { w: img.naturalWidth, h: img.naturalHeight }; }} />
			<span class="cross" style:left="{anchor.x}%" style:top="{anchor.y}%"></span>
		</div>
		<span class="lbl">{m['creator.map_image_previews']()}</span>
		<div class="previews">
			{#each [0.5, 1, 2] as z (z)}
				<div class="preview" style:height="{size * z * 1.1 + 12}px">
					<img src={asset(place.image.src) + (bust ? `?v=${bust}` : '')} alt="" style:width="{size * z}px" style:left="calc(50% - {size * z * (anchor.x / 100)}px)" style:bottom="{6 + size * z * ((100 - anchor.y) / 100) * (natural ? natural.h / natural.w : 1)}px" />
					<span class="dot"></span>
					<span class="z">{z * 100} %</span>
				</div>
			{/each}
		</div>
	{/if}
</fieldset>

<style>
	.slots {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
	}
	.checker {
		background-color: #fff;
		background-image: linear-gradient(45deg, #ddd 25%, transparent 25%), linear-gradient(-45deg, #ddd 25%, transparent 25%),
			linear-gradient(45deg, transparent 75%, #ddd 75%), linear-gradient(-45deg, transparent 75%, #ddd 75%);
		background-size: 16px 16px;
		background-position: 0 0, 0 8px, 8px -8px, -8px 0;
	}
	.anchor {
		position: relative;
		width: 160px;
		border: 1px solid var(--ink);
		cursor: crosshair;
		touch-action: none;
	}
	.anchor img {
		display: block;
		width: 100%;
		height: auto;
		pointer-events: none;
	}
	.cross {
		position: absolute;
		width: 18px;
		height: 18px;
		transform: translate(-50%, -50%);
		pointer-events: none;
		background:
			linear-gradient(var(--error, #e00), var(--error, #e00)) center / 2px 100% no-repeat,
			linear-gradient(var(--error, #e00), var(--error, #e00)) center / 100% 2px no-repeat;
	}
	.previews {
		display: flex;
		gap: 0.4rem;
		align-items: flex-end;
	}
	.preview {
		position: relative;
		flex: 1;
		min-width: 3rem;
		background: var(--primary-cyan, #00d1ff);
		overflow: hidden;
	}
	.preview img {
		position: absolute;
		height: auto;
	}
	.preview .dot {
		position: absolute;
		left: calc(50% - 3px);
		bottom: 3px;
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--ink);
	}
	.preview .z {
		position: absolute;
		top: 2px;
		left: 3px;
		font-size: var(--type-caption-size);
	}
</style>
