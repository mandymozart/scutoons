<script lang="ts">
	import { IMAGE_QUALITY_DENSITIES, MEDIA_QUALITIES, VIDEO_QUALITY_LABELS, type MediaQuality } from '@scutoons/content';
	import { rootPath, toonRelativePath } from '@scutoons/api';
	import { CloseButton, CloseIcon, Icon } from '@scutoons/ui';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { defaultLevel, mediaFromAsset, withVariant, type VariantMedia } from '$lib/editor/media-variants.ts';
	import AssetPickerSheet, { type PickedAsset } from './assets/asset-picker-sheet.svelte';

	/**
	 * One medium with its quality levels as ONE slot (owner 2026-10-03: "simplify the multiple resolutions situation"):
	 * the preview is the button that opens the asset picker – picking an asset fills every level from its renditions
	 * (480p / 720p / 1080p, the medium one the default); under it one chip per level shows which files exist, a chip
	 * opens the picker on that file's asset (its files listed) to replace just that level, its × clears it. Paths only
	 * as tooltips; × beside the preview clears the whole medium. Same `{ src, variants }` shape as the slice video – the
	 * season trailer maps its `loop` to `src`.
	 */
	let {
		toon,
		value,
		kind = 'video',
		label,
		place = '',
		poster,
		onchange,
		testid
	}: {
		toon: string;
		/** toon-relative paths */
		value: VariantMedia;
		kind?: 'video' | 'image';
		label: string;
		/** where uploads go by the content rules (CDN-root folder, e.g. catpope/s1/) */
		place?: string;
		/** a poster (toon-relative) shown in the video preview until it loads */
		poster?: string;
		onchange: (next: VariantMedia) => void;
		testid?: string;
	} = $props();

	const levelLabel = (level: MediaQuality) => (kind === 'image' ? `${IMAGE_QUALITY_DENSITIES[level]}x` : VIDEO_QUALITY_LABELS[level]);
	const url = (path: string) => asset(`${toon}/${path}`);
	const main = $derived(defaultLevel(value));

	/** null = the picker fills the whole medium; a level = it replaces only that file */
	let target = $state<MediaQuality | null>(null);
	let open = $state(false);
	let size = $state<{ w: number; h: number } | null>(null);
	let failed = $state(false);

	$effect(() => {
		void value.src;
		size = null;
		failed = false;
	});

	function choose(level: MediaQuality | null) {
		target = level;
		open = true;
	}

	function pick(picked: PickedAsset) {
		if (target === null) onchange(mediaFromAsset(picked.asset.renditions, picked.path, toon, kind === 'image' ? 'image/' : 'video/'));
		else onchange(withVariant(value, target, toonRelativePath(picked.path, toon)));
	}

	/** the picker opens on the file the click is about (its asset preselected, its files listed) */
	const pickerValue = $derived.by(() => {
		const file = (target && value.variants?.[target]) || value.src;
		return file ? rootPath(file, toon) : '';
	});
</script>

<div class="variants-slot" data-testid={testid}>
	<span class="sc-label">{label}</span>
	<div class="row">
		<button
			type="button"
			class="slot"
			class:empty={!value.src}
			onclick={() => choose(null)}
			title={value.src ? `${value.src}\n${m['creator.variants_choose_hint']()}` : m['creator.variants_choose_hint']()}
			aria-label="{label}: {value.src ?? m['creator.assets_choose']()}"
			data-testid={testid ? `${testid}-choose` : undefined}
		>
			{#if !value.src}
				<Icon name="plus" />
			{:else if failed}
				<span class="bad">{m['creator.media_missing']()}</span>
			{:else if kind === 'video'}
				<!-- svelte-ignore a11y_media_has_caption -->
				<video src={url(value.src)} poster={poster ? url(poster) : undefined} muted playsinline preload="metadata" onloadedmetadata={(e) => { const v = e.currentTarget as HTMLVideoElement; size = { w: v.videoWidth, h: v.videoHeight }; }} onerror={() => (failed = true)}></video>
			{:else}
				<img src={url(value.src)} alt="" onload={(e) => { const img = e.currentTarget as HTMLImageElement; size = { w: img.naturalWidth, h: img.naturalHeight }; }} onerror={() => (failed = true)} />
			{/if}
		</button>
		{#if value.src}
			<CloseButton label={m['creator.remove']()} onclick={() => onchange({})} />
		{/if}
	</div>
	{#if value.src}
		<!-- the levels: which files exist; the default marked; each one replaceable / clearable on its own -->
		<ul class="levels" aria-label={m['creator.video_files']()} title={m['creator.video_files_hint']()}>
			{#each MEDIA_QUALITIES as level (level)}
				{@const file = value.variants?.[level]}
				<li class="level sc-chip-control" class:missing={!file} data-testid={testid ? `${testid}-${level}` : undefined}>
					<button
						type="button"
						onclick={() => choose(level)}
						title={file ? `${file}\n${m['creator.variants_level_replace']()}` : m['creator.variants_level_missing']()}
						aria-label="{levelLabel(level)}: {file ?? m['creator.variants_level_missing']()}"
					>
						{levelLabel(level)}{#if main === level}&nbsp;<span class="default">({m['creator.video_default']().toLowerCase()})</span>{/if}
					</button>
					{#if file}
						<button type="button" onclick={() => onchange(withVariant(value, level, undefined))} aria-label={m['creator.variants_level_clear']({ level: levelLabel(level) })} title={m['creator.variants_level_clear']({ level: levelLabel(level) })}><CloseIcon /></button>
					{/if}
				</li>
			{/each}
		</ul>
		{#if main === null}
			<span class="sc-caption" title={value.src}>{m['creator.variants_single']()}</span>
		{/if}
		{#if size}<span class="sc-caption">{size.w} × {size.h} px</span>{/if}
	{/if}
	<AssetPickerSheet
		bind:open
		{toon}
		{kind}
		value={pickerValue}
		{place}
		title={target ? `${label} · ${levelLabel(target)}` : label}
		onpick={pick}
	/>
</div>

<style>
	.variants-slot {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
		min-width: 0;
	}
	.row {
		display: flex;
		align-items: flex-start;
		gap: 0.3rem;
	}
	/* the same slot as media-slot.svelte, wider: a video is landscape */
	.slot {
		display: flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: 14rem;
		min-height: 5rem;
		padding: 0;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		cursor: pointer;
		overflow: hidden;
	}
	.slot.empty {
		border-style: dashed;
		color: var(--muted);
	}
	.slot:hover,
	.slot:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: 2px;
	}
	.slot img,
	.slot video {
		display: block;
		width: 100%;
		height: auto;
	}
	.bad {
		padding: 0.5rem;
		color: var(--error);
		font-size: var(--type-caption-size);
	}
	.levels {
		display: flex;
		flex-wrap: wrap;
		gap: 0.3rem;
		margin: 0;
		padding: 0;
		list-style: none;
	}
	/* an interactive chip (tools.css .sc-chip-control); its first button is the level itself */
	.level {
		padding-left: 0;
	}
	.level.missing {
		border-style: dashed;
		border-color: var(--border-muted);
	}
	.level.missing button {
		color: var(--text-muted);
	}
	.default {
		color: var(--text-muted);
	}
</style>
