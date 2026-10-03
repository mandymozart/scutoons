<script lang="ts">
	import { rootPath, toonRelativePath } from '@scutoons/api';
	import { CloseIcon, Icon } from '@scutoons/ui';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import AssetPickerSheet from '$lib/components/media/asset-picker-sheet.svelte';

	/**
	 * One image or video of a page as a slot (owner 2026-10-03: "the text fields for the images are not needed, the
	 * preview does the job; if something is missing an empty slot will do"): the preview is the button that opens the
	 * asset picker, an empty slot when nothing is set, the path only as the tooltip; × clears it.
	 */
	let {
		toon,
		value,
		kind = 'image',
		label,
		place = '',
		onchange,
		testid,
		bust = 0
	}: {
		/** the toon folder ('' = paths from the CDN root, e.g. the character registry's) */
		toon: string;
		/** toon-relative path, e.g. s1/c2/s1c2-key-visual.webp (from the CDN root without a toon) */
		value: string | undefined;
		kind?: 'image' | 'video';
		label: string;
		place?: string;
		onchange: (path: string | undefined) => void;
		testid?: string;
		/** set after the file got new bytes at the same path (a timestamp): the preview loads again */
		bust?: number;
	} = $props();

	let open = $state(false);
	let size = $state<{ w: number; h: number } | null>(null);
	let failed = $state(false);
	const src = $derived(value ? asset(toon ? `${toon}/${value}` : value) + (bust ? `?v=${bust}` : '') : '');

	$effect(() => {
		void value;
		size = null;
		failed = false;
	});
</script>

<div class="media-slot" data-testid={testid}>
	<span class="sc-label">{label}</span>
	<div class="row">
		<button type="button" class="slot" class:empty={!value} onclick={() => (open = true)} title={value ? `${value}\n${m['creator.assets_choose_hint']()}` : m['creator.assets_choose_hint']()} aria-label="{label}: {value ?? m['creator.assets_choose']()}">
			{#if !value}
				<Icon name="plus" />
			{:else if failed}
				<span class="bad">{m['creator.media_missing']()}</span>
			{:else if kind === 'video'}
				<!-- svelte-ignore a11y_media_has_caption -->
				<video {src} muted playsinline preload="metadata" onloadedmetadata={(e) => { const v = e.currentTarget as HTMLVideoElement; size = { w: v.videoWidth, h: v.videoHeight }; }} onerror={() => (failed = true)}></video>
			{:else}
				<img {src} alt="" onload={(e) => { const img = e.currentTarget as HTMLImageElement; size = { w: img.naturalWidth, h: img.naturalHeight }; }} onerror={() => (failed = true)} />
			{/if}
		</button>
		{#if value}
			<!-- icon only → round (owner 2026-10-03) -->
			<button type="button" class="sc-round" aria-label={m['creator.remove']()} title={m['creator.remove']()} onclick={() => onchange(undefined)}><CloseIcon /></button>
		{/if}
	</div>
	{#if size}<span class="sc-caption">{size.w} × {size.h} px</span>{/if}
	<AssetPickerSheet
		bind:open
		{toon}
		{kind}
		value={value ? rootPath(value, toon) : ''}
		{place}
		title={label}
		onpick={(picked) => onchange(toonRelativePath(picked.path, toon))}
	/>
</div>

<style>
	.media-slot {
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
	.slot {
		display: flex;
		align-items: center;
		justify-content: center;
		box-sizing: border-box;
		width: 8rem;
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
</style>
