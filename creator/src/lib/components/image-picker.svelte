<script lang="ts">
	import { asset } from '$lib/utils/cdn.ts';
	import { DEFAULT_SLICE } from '@scutoons/content';
	import { rootPath, toonRelativePath } from '@scutoons/api';
	import { m } from '$lib/paraglide/messages.js';
	import AssetPickerSheet from './assets/asset-picker-sheet.svelte';

	/**
	 * Image path (relative to the toon's CDN folder) with the known paths as suggestions, a live CDN preview and the
	 * asset picker ("Choose …", agents/ASSETS.md) – it writes the same stable path a typed one would be.
	 */
	let {
		toon,
		value,
		known = [],
		onchange,
		label = 'image',
		hideLabel = false,
		showFormat = false,
		place = ''
	}: {
		toon: string;
		value: string | undefined;
		known?: string[];
		onchange: (path: string) => void;
		label?: string;
		/** the label only for screen readers – when the surrounding field already shows it (owner 2026-10-02: "LOGO" twice) */
		hideLabel?: boolean;
		showFormat?: boolean;
		/** where an upload goes (CDN-root folder, the asset picker's `place`) */
		place?: string;
	} = $props();

	let open = $state(false);
	let size = $state<{ w: number; h: number } | null>(null);
	let failed = $state(false);

	const listId = $derived(`images-${label}`);
	const formatNote = $derived.by(() => {
		if (!size) return '';
		// slices are 390 px wide (1x); the standard height is 625, other heights are fine
		return size.w === DEFAULT_SLICE.width ? 'ok' : 'off';
	});

	$effect(() => {
		void value;
		size = null;
		failed = false;
	});
</script>

<div class="picker">
	<label for="{listId}-input" class="lbl" class:sr-only={hideLabel}>{label}</label>
	<div class="row">
		<input
			id="{listId}-input"
			type="text"
			list={listId}
			value={value ?? ''}
			placeholder="s1/c2/s1c2-slice-1.webp"
			onchange={(e) => onchange(e.currentTarget.value.trim())}
			data-testid="image-path"
		/>
		<button type="button" class="secondary browse" onclick={() => (open = true)} title={m['creator.assets_choose_hint']()} data-testid="image-choose">{m['creator.assets_choose']()}</button>
	</div>
	<datalist id={listId}>
		{#each known as p}<option value={p}></option>{/each}
	</datalist>

	{#if value}
		<div class="preview">
			<img
				src={asset(`${toon}/${value}`)}
				alt=""
				onload={(e) => {
					const img = e.currentTarget as HTMLImageElement;
					size = { w: img.naturalWidth, h: img.naturalHeight };
				}}
				onerror={() => (failed = true)}
			/>
			<span class="meta">
				{#if failed}<span class="bad">not found on the CDN</span>{:else if size}{size.w} × {size.h} px{/if}
				{#if showFormat && formatNote === 'off'}<span class="warn"> · not {DEFAULT_SLICE.width} px wide</span>{/if}
			</span>
		</div>
	{/if}

	<AssetPickerSheet
		bind:open
		{toon}
		kind="image"
		value={value ? rootPath(value, toon) : ''}
		{place}
		title={label}
		onpick={(picked) => onchange(toonRelativePath(picked.path, toon))}
	/>
</div>

<style>
	.picker {
		display: flex;
		flex-direction: column;
		gap: 0.3rem;
	}
	.lbl {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
	.preview {
		display: flex;
		gap: 0.5rem;
		align-items: flex-start;
	}
	.preview img {
		width: 64px;
		height: auto;
		border: 1px solid var(--border-subtle);
	}
	.meta {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.bad {
		color: var(--error);
	}
	.warn {
		color: var(--warn);
	}
	.row {
		display: flex;
		gap: 0.3rem;
		min-width: 0;
	}
	.row input {
		flex: 1;
		min-width: 0;
	}
	.browse {
		flex: none;
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		margin: -1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
	}
</style>
