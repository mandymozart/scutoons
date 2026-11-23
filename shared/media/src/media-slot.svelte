<script lang="ts">
	import { rootPath, toonRelativePath } from '@scutoons/api';
	import { CloseIcon, Icon } from '@scutoons/ui';
	import { mediaHost, mediaText, mediaUrl as asset } from './setup.ts';
	import { toonAsset } from '@scutoons/content';
	import { slotFit } from './slot-host.ts';
	import { SlotDrop } from './slot-drop.svelte.ts';

	/**
	 * One image or video of a page as a slot: the preview is the button that asks the app's host for a file (the
	 * creator's Library), an empty slot when nothing is set, the path only as the tooltip; a file dragged from the host
	 * drops on it; × clears it.
	 */
	let {
		toon,
		value,
		kind = 'image',
		label,
		place = '',
		onchange,
		testid,
		bust = 0,
		hideLabel = false
	}: {
		/** the toon folder ('' = paths from the CDN root, e.g. the character registry's) */
		toon: string;
		/** toon-relative path, e.g. s1/c2/s1c2-key-visual.webp (from the CDN root without a toon) */
		value: string | undefined;
		kind?: 'image' | 'video';
		label: string;
		/** where a new file for this slot belongs (a CDN-root folder, e.g. catpope/s1/) */
		place?: string;
		onchange: (path: string | undefined) => void;
		testid?: string;
		/** set after the file got new bytes at the same path (a timestamp): the preview loads again */
		bust?: number;
		/** no visible label – the surrounding field shows it (the slot button keeps it as its accessible name) */
		hideLabel?: boolean;
	} = $props();

	const t = $derived(mediaText());
	const fit = $derived(slotFit(kind));
	const take = (path: string) => onchange(toonRelativePath(path, toon));
	const drop = new SlotDrop(() => fit, take);
	const pick = () => mediaHost()?.pick({ ...fit, label, value: value ? rootPath(value, toon) : '', place, onpick: take });
	let size = $state<{ w: number; h: number } | null>(null);
	let failed = $state(false);
	const src = $derived(value ? asset(toonAsset(toon, value)) + (bust ? `?v=${bust}` : '') : '');

	$effect(() => {
		void value;
		size = null;
		failed = false;
	});
</script>

<div class="media-slot" data-testid={testid}>
	{#if !hideLabel}<span class="sc-label">{label}</span>{/if}
	<div class="row">
		<button
			type="button"
			class="slot"
			class:empty={!value}
			class:fits={drop.answer?.ok}
			class:misfit={drop.answer && !drop.answer.ok}
			onclick={pick}
			ondragover={drop.over}
			ondragleave={drop.leave}
			ondrop={drop.drop}
			title={value ? `${value}\n${t.chooseHint}` : t.chooseHint}
			aria-label="{label}: {value ?? t.choose}"
		>
			{#if !value}
				<Icon name="plus" />
			{:else if failed}
				<span class="bad">{t.missing}</span>
			{:else if kind === 'video'}
				<!-- svelte-ignore a11y_media_has_caption -->
				<video {src} muted playsinline preload="metadata" onloadedmetadata={(e) => { const v = e.currentTarget as HTMLVideoElement; size = { w: v.videoWidth, h: v.videoHeight }; }} onerror={() => (failed = true)}></video>
			{:else}
				<img {src} alt="" onload={(e) => { const img = e.currentTarget as HTMLImageElement; size = { w: img.naturalWidth, h: img.naturalHeight }; }} onerror={() => (failed = true)} />
			{/if}
		</button>
		{#if value}
			<!-- icon only → round (owner 2026-10-03) -->
			<button type="button" class="sc-round" aria-label={t.remove} title={t.remove} onclick={() => onchange(undefined)}><CloseIcon /></button>
		{/if}
	</div>
	{#if drop.answer && !drop.answer.ok}<span class="sc-caption misfit" data-testid="slot-misfit">{drop.answer.why}</span>
	{:else if size}<span class="sc-caption">{size.w} × {size.h} px</span>{/if}
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
	.slot.fits {
		outline: var(--border-width-focus) dashed var(--tool-text);
		outline-offset: 2px;
	}
	.slot.misfit {
		outline: var(--border-width-focus) dashed var(--error);
		outline-offset: 2px;
	}
	.misfit {
		max-width: 8rem;
		color: var(--error);
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
