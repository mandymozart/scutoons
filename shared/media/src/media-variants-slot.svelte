<script lang="ts">
	import { AUDIO_QUALITY_LABELS, IMAGE_QUALITY_DENSITIES, MEDIA_QUALITIES, VIDEO_QUALITY_LABELS, type MediaQuality } from '@scutoons/content';
	import { rootPath, toonRelativePath } from '@scutoons/api';
	import { CloseIcon, Icon } from '@scutoons/ui';
	import { mediaText, mediaUrl as asset } from './setup.ts';
	import { defaultLevel, mediaFromAsset, withVariant, type VariantMedia } from './media-variants.ts';
	import AssetPickerSheet, { type PickedAsset } from './asset-picker-sheet.svelte';

	/**
	 * One medium with its quality levels as ONE slot (owner 2026-10-03: "simplify the multiple resolutions situation"):
	 * the preview is the button that opens the asset picker – picking an asset fills every level from its renditions
	 * (480p / 720p / 1080p, the medium one the default); under it one chip per level shows which files exist, a chip
	 * opens the picker on that file's asset (its files listed) to replace just that level, its × clears it. Paths only
	 * as tooltips; × beside the preview clears the whole medium. Same `{ src, variants }` shape as the slice video – the
	 * season trailer maps its `loop` to `src`. Audio (owner 2026-10-04): the preview is ♪ + the file's name (a small player
	 * under it), the levels 96k / 128k / 320k; music tracks and samples map their `path` to `src`.
	 */
	let {
		toon,
		value,
		kind = 'video',
		label,
		place = '',
		poster,
		onchange,
		onmeta,
		testid,
		hideLabel = false,
		clearable = true,
		player = true
	}: {
		toon: string;
		/** toon-relative paths */
		value: VariantMedia;
		kind?: 'video' | 'image' | 'audio';
		label: string;
		/** where uploads go by the content rules (CDN-root folder, e.g. catpope/s1/) */
		place?: string;
		/** a poster (toon-relative) shown in the video preview until it loads */
		poster?: string;
		onchange: (next: VariantMedia) => void;
		/** the default file's state: its size once loaded, `null` when it is missing on the CDN */
		onmeta?: (size: { w: number; h: number } | null) => void;
		testid?: string;
		/** no visible label – a table column says it (the slot keeps it as its accessible name) */
		hideLabel?: boolean;
		/** false: no × for the whole medium (a row with its own remove) */
		clearable?: boolean;
		/** audio: false = no player under the slot (the row has its own ▶) */
		player?: boolean;
	} = $props();

	const t = $derived(mediaText());
	const levelLabel = (level: MediaQuality) =>
		kind === 'image' ? `${IMAGE_QUALITY_DENSITIES[level]}x` : kind === 'audio' ? AUDIO_QUALITY_LABELS[level] : VIDEO_QUALITY_LABELS[level];
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
		if (target === null) onchange(mediaFromAsset(picked.asset.renditions, picked.path, toon, `${kind}/`));
		else onchange(withVariant(value, target, toonRelativePath(picked.path, toon)));
	}

	/** the picker opens on the file the click is about (its asset preselected, its files listed) */
	const pickerValue = $derived.by(() => {
		const file = (target && value.variants?.[target]) || value.src;
		return file ? rootPath(file, toon) : '';
	});
</script>

<div class="variants-slot" data-testid={testid}>
	{#if !hideLabel}<span class="sc-label">{label}</span>{/if}
	<div class="row">
		<button
			type="button"
			class="slot"
			class:image={kind === 'image'}
			class:audio={kind === 'audio'}
			class:empty={!value.src}
			onclick={() => choose(null)}
			title={value.src ? `${value.src}\n${kind === 'audio' ? t.audioVariantsChooseHint : t.variantsChooseHint}` : kind === 'audio' ? t.audioVariantsChooseHint : t.variantsChooseHint}
			aria-label="{label}: {value.src ?? t.choose}"
			data-testid={testid ? `${testid}-choose` : undefined}
		>
			{#if !value.src}
				<Icon name="plus" />
			{:else if kind === 'audio'}
				<Icon name="music" /> <span class="name">{value.src.split('/').pop()}</span>
			{:else if failed}
				<span class="bad">{t.missing}</span>
			{:else if kind === 'video'}
				<!-- svelte-ignore a11y_media_has_caption -->
				<video src={url(value.src)} poster={poster ? url(poster) : undefined} muted playsinline preload="metadata" onloadedmetadata={(e) => { const v = e.currentTarget as HTMLVideoElement; size = { w: v.videoWidth, h: v.videoHeight }; onmeta?.(size); }} onerror={() => { failed = true; onmeta?.(null); }}></video>
			{:else}
				<img src={url(value.src)} alt="" onload={(e) => { const img = e.currentTarget as HTMLImageElement; size = { w: img.naturalWidth, h: img.naturalHeight }; onmeta?.(size); }} onerror={() => { failed = true; onmeta?.(null); }} />
			{/if}
		</button>
		{#if value.src && clearable}
			<!-- icon only → round (owner 2026-10-03) -->
			<button type="button" class="sc-round" aria-label={t.remove} title={t.remove} onclick={() => onchange({})} data-testid={testid ? `${testid}-clear` : undefined}><CloseIcon /></button>
		{/if}
	</div>
	{#if value.src}
		<!-- the levels: which files exist; the default marked; each one replaceable / clearable on its own -->
		<ul class="levels" aria-label={t.files} title={t.filesHint}>
			{#each MEDIA_QUALITIES as level (level)}
				{@const file = value.variants?.[level]}
				<li class="level sc-chip-control" class:missing={!file} data-file={file ?? ''} data-default={main === level ? 'true' : undefined} data-testid={testid ? `${testid}-${level}` : undefined}>
					<button
						type="button"
						onclick={() => choose(level)}
						title={file ? `${file}\n${t.levelReplace}` : t.levelMissing}
						aria-label="{levelLabel(level)}: {file ?? t.levelMissing}"
					>
						{levelLabel(level)}{#if main === level}&nbsp;<span class="default">({t.defaultLevel.toLowerCase()})</span>{/if}
					</button>
					{#if file}
						<button type="button" onclick={() => onchange(withVariant(value, level, undefined))} data-testid={testid ? `${testid}-${level}-clear` : undefined} aria-label={t.levelClear(levelLabel(level))} title={t.levelClear(levelLabel(level))}><CloseIcon /></button>
					{/if}
				</li>
			{/each}
		</ul>
		{#if main === null}
			<span class="sc-caption" title={value.src}>{t.single}</span>
		{/if}
		{#if kind === 'audio' && player}
			<audio controls preload="none" src={url(value.src)}></audio>
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
		min-width: 0;
		align-items: flex-start;
		gap: 0.3rem;
	}
	/* the same slot as media-slot.svelte, wider: a video is landscape */
	.slot {
		display: flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		flex: 0 1 14rem;
		min-width: 0;
		min-height: 5rem;
		padding: 0;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		cursor: pointer;
		overflow: hidden;
	}
	/* an image (a portrait slice) is a small preview, like the single media slot (owner 2026-10-03) */
	.slot.image {
		flex-basis: 6rem;
	}
	/* audio: a one-line slot (♪ + the file's name), the control height, full width */
	.slot.audio {
		flex: 1 1 auto;
		justify-content: flex-start;
		gap: var(--space-05);
		min-height: var(--control-height);
		padding: 0 var(--space-075);
		color: var(--ink);
	}
	.slot.audio.empty {
		justify-content: center;
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	audio {
		width: 100%;
		max-width: 22rem;
	}
	.slot.empty {
		border-style: dashed;
		color: var(--muted);
	}
	/* an empty image / video slot looks like the default empty slice (390 × 625, portrait) – not landscape, which
	   suggested videos are landscape (owner 2026-10-06, BACKLOG M.16); a chosen video shows its own ratio */
	.slot.empty:not(.audio) {
		flex-basis: 6rem;
		aspect-ratio: 390 / 625;
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
