<script lang="ts">
	import { DEFAULT_PLACE_ANCHOR, DEFAULT_PLACE_IMAGE_SIZE, placeImagePath, type MapPlace } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { asset } from '$lib/utils/cdn.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { cdnFolder, cdnHas, cdnSize, editPlace, numberOrUnset } from '$lib/editor/map.ts';

	/**
	 * A place's illustration (agents/MAP.md §4.3): the path (or a file of the place folder), its size on the map, the ground
	 * point (drag on the picture) and previews at 50 / 100 / 200 %, with warnings for a missing, large or blurry file.
	 */
	let { doc, place }: { doc: ContentDoc; place: MapPlace } = $props();

	/** the place illustrations' folder (owner 2026-10-01): `<toon>/map/places/` (`scutoons/…` for a shared place) */
	let folder = $state('');
	$effect(() => {
		folder = placeImagePath(place).replace(/[^/]+$/, '');
	});
	let folderFiles = $state<string[] | null>(null);
	let browsing = $state(false);
	async function browse() {
		browsing = !browsing;
		if (browsing) folderFiles = await cdnFolder(folder);
	}
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
	function setImage(src: string) {
		editPlace(doc, place.id, (p) => {
			if (!src) delete p.image;
			else p.image = { ...(p.image ?? {}), src };
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
	<input type="text" value={place.image?.src ?? ''} placeholder={placeImagePath(place)} onchange={(e) => setImage(e.currentTarget.value.trim())} aria-label={m['creator.map_image']()} data-testid="place-image-path" />
	<div class="row">
		<label class="field"><span>{m['creator.map_image_folder']()}</span><input type="text" bind:value={folder} /></label>
		<button type="button" class="ghost" onclick={browse} aria-expanded={browsing}>{folderFiles ? m['creator.map_image_browse']({ count: folderFiles.length }) : '…'}</button>
	</div>
	{#if browsing}
		{#if folderFiles?.length}
			<ul class="grid">
				{#each folderFiles as f (f)}
					<li><button type="button" class="thumb" data-control-exempt="image thumbnail card" class:current={f === place.image?.src} title={f} onclick={() => setImage(f)}><img src={asset(f)} alt="" loading="lazy" /><span>{f.split('/').pop()}</span></button></li>
				{/each}
			</ul>
		{:else}
			<p class="hint">{m['creator.map_image_none']()}</p>
		{/if}
	{/if}
	{#if place.image?.src}
		<p class="hint">
			{#if imageState.missing}<span class="bad">{m['creator.map_image_missing']()}</span>{/if}
			{#if natural}{natural.w} × {natural.h} px{/if}
			{#if imageState.kb !== null} · {imageState.kb} KB{/if}
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
			<img src={asset(place.image.src)} alt="" draggable="false" onload={(e) => { const img = e.currentTarget as HTMLImageElement; natural = { w: img.naturalWidth, h: img.naturalHeight }; }} />
			<span class="cross" style:left="{anchor.x}%" style:top="{anchor.y}%"></span>
		</div>
		<span class="lbl">{m['creator.map_image_previews']()}</span>
		<div class="previews">
			{#each [0.5, 1, 2] as z (z)}
				<div class="preview" style:height="{size * z * 1.1 + 12}px">
					<img src={asset(place.image.src)} alt="" style:width="{size * z}px" style:left="calc(50% - {size * z * (anchor.x / 100)}px)" style:bottom="{6 + size * z * ((100 - anchor.y) / 100) * (natural ? natural.h / natural.w : 1)}px" />
					<span class="dot"></span>
					<span class="z">{z * 100} %</span>
				</div>
			{/each}
		</div>
		<button type="button" class="ghost" onclick={() => setImage('')}>{m['creator.map_image_remove']()}</button>
	{/if}
</fieldset>

<style>
	.grid { /* grid-ok: the image folder thumbnails, not fields */
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(4.5rem, 1fr));
		gap: 0.25rem;
		max-height: 14rem;
		overflow: auto;
	}
	.thumb {
		all: unset;
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
		align-items: center;
		width: 100%;
		padding: 0.2rem;
		border: 1px solid var(--border-subtle);
		cursor: pointer;
		font-size: var(--type-caption-size);
	}
	.thumb.current {
		border: 2px solid var(--ink);
	}
	.thumb img {
		width: 100%;
		height: 3rem;
		object-fit: contain;
	}
	.thumb span {
		max-width: 100%;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
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
