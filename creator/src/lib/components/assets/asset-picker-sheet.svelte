<script lang="ts" module>
	import type { AssetKind, AssetRendition, AssetSummary } from '@scutoons/api';

	/** What the picker hands back: the asset, the chosen file (a rendition) and its CDN-root path */
	export interface PickedAsset {
		asset: AssetSummary;
		rendition: AssetRendition | null;
		/** CDN-root path, e.g. catpope/s1/s1-trailer-loop.720p.mp4 */
		path: string;
	}
	export type AssetPickerKind = AssetKind | '';

	/** 1234567 → "1.2 MB" */
	export function bytes(n: number): string {
		if (n < 1024) return `${n} B`;
		const units = ['KB', 'MB', 'GB'];
		let v = n / 1024;
		let i = 0;
		while (v >= 1024 && i < units.length - 1) {
			v /= 1024;
			i++;
		}
		return `${v.toFixed(v < 10 ? 1 : 0)} ${units[i]}`;
	}
</script>

<script lang="ts">
	import { untrack } from 'svelte';
	import { ApiError, ASSET_KINDS } from '@scutoons/api';
	import { CloseButton, Icon, Sheet } from '@scutoons/ui';
	import { assetsApi } from '$lib/api.ts';
	import { asset as cdnAsset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The asset picker (agents/ASSETS.md "Asset selector") – like the character picker: a searchable grid of the media
	 * in the asset registry (thumbnails, one line of name, size / dimensions / uses), filters (folder = the toon by
	 * default, kind, unused), the chosen asset's files (renditions: 480p / 720p / 1080p, WebP …) and an upload. Picking
	 * hands back a CDN-root path – the caller writes the same stable paths content always had.
	 * Keyboard: arrows move in the grid, Enter opens / picks, Escape closes.
	 */
	let {
		open = $bindable(false),
		toon = '',
		kind = 'image',
		value = '',
		place = '',
		title,
		onpick
	}: {
		open?: boolean;
		/** the toon folder shown first ('' = every folder) */
		toon?: string;
		/** the kind the field takes ('' = any) */
		kind?: AssetPickerKind;
		/** the field's current file (CDN-root path) – preselected */
		value?: string;
		/** where an upload goes by the content rules (CDN-root folder of the field's place, e.g. catpope/s1/c2/) */
		place?: string;
		title?: string;
		onpick: (picked: PickedAsset) => void;
	} = $props();

	let query = $state('');
	let folder = $state('');
	let kindFilter = $state<AssetPickerKind>('');
	let unusedOnly = $state(false);
	let items = $state<AssetSummary[]>([]);
	let folders = $state<{ folder: string | null; assets: number }[]>([]);
	let total = $state(0);
	let page = $state(1);
	let loading = $state(false);
	let error = $state('');
	let chosen = $state<AssetSummary | null>(null);
	let grid = $state<HTMLElement>();
	let uploadPath = $state('');
	let uploadFile = $state<File | null>(null);
	let uploading = $state(false);
	let uploadError = $state('');
	let uploadInput = $state<HTMLInputElement>();
	let timer: ReturnType<typeof setTimeout> | undefined;
	let request = 0;

	// every time it opens: the toon's folder, the field's kind, the current file preselected
	$effect(() => {
		if (!open) return;
		folder = toon;
		kindFilter = kind;
		query = '';
		unusedOnly = false;
		chosen = null;
		uploadFile = null;
		uploadError = '';
		// untracked: load() reads the search and the filters – tracked, every keystroke re-ran this reset (search and
		// "unused only" looked dead, owner 2026-10-03)
		untrack(() => void load(1, value));
	});

	async function load(next = 1, select = '') {
		const mine = ++request;
		loading = true;
		error = '';
		try {
			const result = await assetsApi.list({
				q: query || undefined,
				folder: folder || undefined,
				kind: kindFilter || undefined,
				state: unusedOnly ? 'unused' : undefined,
				page: next,
				page_size: 60,
				sort: 'slug'
			});
			if (mine !== request) return;
			items = next === 1 ? result.items : [...items, ...result.items];
			folders = result.folders;
			total = result.pagination.total;
			page = next;
			if (select) {
				const found = await assetsApi.lookup(select).catch(() => null);
				if (mine === request && found?.asset) {
					chosen = items.find((a) => a.id === found.asset!.asset.id) ?? summaryOf(found.asset);
				}
			}
		} catch (e) {
			if (mine !== request) return;
			error = e instanceof ApiError && e.status === 503 ? m['creator.assets_unavailable']() : e instanceof Error ? e.message : String(e);
		} finally {
			if (mine === request) loading = false;
		}
	}

	function summaryOf(detail: NonNullable<Awaited<ReturnType<typeof assetsApi.lookup>>['asset']>): AssetSummary {
		const current = detail.versions.find((v) => v.status === 'current') ?? detail.versions[0];
		return {
			id: detail.asset.id, slug: detail.asset.slug, kind: detail.asset.kind, folder: detail.asset.folder, title: detail.asset.title,
			tags: detail.asset.tags, version: current?.number ?? 1, path: current?.path ?? '', width: current?.width ?? null,
			height: current?.height ?? null, duration: current?.duration ?? null, size: current?.renditions.reduce((s, r) => s + r.size, 0) ?? 0,
			usages: detail.usages.length, updated_at: detail.asset.updated_at, renditions: current?.renditions ?? []
		};
	}

	function refilter() {
		clearTimeout(timer);
		timer = setTimeout(() => load(1), 200);
	}

	/** the file a card shows: an image itself, a video's poster rendition, nothing for audio / zip */
	function thumb(a: AssetSummary): string {
		if (a.kind === 'image') {
			const small = [...a.renditions].filter((r) => r.mime?.startsWith('image/')).sort((x, y) => (x.size || 0) - (y.size || 0))[0];
			return small?.path ?? a.path;
		}
		return a.renditions.find((r) => r.label === 'poster')?.path ?? '';
	}
	const fileName = (path: string) => path.split('/').pop() ?? path;
	/** original first, then the quality levels low → high, then sizes (1280w …), then the rest */
	const rank = (r: AssetRendition) =>
		r.label === 'original' ? 0 : r.quality ? 1 + ['low', 'medium', 'high'].indexOf(r.quality) : /^\d+[wpk]$/.test(r.label) ? 10 + parseInt(r.label, 10) / 10000 : 20;
	const ordered = (list: AssetRendition[]) => [...list].sort((x, y) => rank(x) - rank(y) || x.label.localeCompare(y.label));
	const icon = (k: AssetKind) => ({ image: 'image', video: 'film', audio: 'music', archive: 'archive', document: 'file-text', other: 'file' })[k];

	function meta(a: { width: number | null; height: number | null; duration: number | null; size: number }) {
		const parts: string[] = [];
		if (a.width && a.height) parts.push(`${a.width} × ${a.height}`);
		if (a.duration) parts.push(`${Math.floor(a.duration / 60)}:${String(Math.round(a.duration % 60)).padStart(2, '0')}`);
		parts.push(bytes(a.size));
		return parts.join(' · ');
	}
	/** a field that takes one file: the rendition matching the kind (an image field never gets the mp4) */
	const pickable = (r: AssetRendition) => !kind || (kind === 'image' ? !!r.mime?.startsWith('image/') : kind === 'video' ? !!r.mime?.startsWith('video/') : true);

	function pick(a: AssetSummary, rendition: AssetRendition | null = null) {
		const r = rendition ?? a.renditions.find((x) => x.path === a.path && pickable(x)) ?? a.renditions.find(pickable) ?? null;
		onpick({ asset: a, rendition: r, path: r?.path ?? a.path });
		open = false;
	}

	function choose(a: AssetSummary) {
		chosen = a;
		queueMicrotask(() => document.querySelector<HTMLElement>('[data-testid="asset-detail"] [data-file]')?.focus());
	}

	function arrows(event: KeyboardEvent) {
		const keys: Record<string, number> = { ArrowLeft: -1, ArrowRight: 1, ArrowUp: -100, ArrowDown: 100 };
		if (!(event.key in keys)) return;
		const cells = [...(event.currentTarget as HTMLElement).querySelectorAll<HTMLElement>('[data-cell]')];
		const index = cells.indexOf(document.activeElement as HTMLElement);
		if (index < 0) return;
		event.preventDefault();
		const columns = Math.max(1, getComputedStyle(cells[index].parentElement!).gridTemplateColumns.split(' ').length);
		const step = keys[event.key] === -100 ? -columns : keys[event.key] === 100 ? columns : keys[event.key];
		cells[Math.min(cells.length - 1, Math.max(0, index + step))]?.focus();
	}

	// ── upload: a new file into its place in the toon hierarchy (agents/ASSETS.md "Folder convention") ─────────────────
	/** without a `place`: "s1c2-slice-14.webp" → "s1/c2", "s1-cover.webp" → "s1", anything else → the toon's own folder (editable) */
	const placeOf = (name: string) => {
		const chapter = name.match(/^s(\d+)c(\d+)[-_]/);
		if (chapter) return `s${chapter[1]}/c${chapter[2]}/`;
		const season = name.match(/^s(\d+)[-_]/);
		return season ? `s${season[1]}/` : '';
	};
	function chooseFile(event: Event) {
		const file = (event.currentTarget as HTMLInputElement).files?.[0] ?? null;
		uploadFile = file;
		uploadError = '';
		if (file) {
			const clean = file.name.toLowerCase().replace(/[^a-z0-9._-]+/g, '-').replace(/-+/g, '-');
			// the field's place first (owner 2026-10-03: uploads go next to where they are used, by the content rules)
			uploadPath = place ? `${place.replace(/\/?$/, '/')}${clean}` : `${folder || toon || 'scutoons'}/${placeOf(clean)}${clean}`;
		}
	}
	async function upload() {
		if (!uploadFile || !uploadPath) return;
		uploading = true;
		uploadError = '';
		try {
			const result = await assetsApi.upload(uploadFile, uploadPath.trim());
			query = '';
			await load(1, result.uploaded);
			uploadFile = null;
		} catch (e) {
			uploadError = e instanceof ApiError && e.code === 'exists' ? m['creator.assets_upload_exists']() : e instanceof Error ? e.message : String(e);
		} finally {
			uploading = false;
		}
	}
</script>

<Sheet bind:open title={title ?? m['creator.assets_picker_title']()} side="right" closeLabel={m['creator.characters_close']()} resizeLabel={m['creator.resize_panel']()}>
	<div class="picker" data-testid="asset-picker">
		<!-- order (owner 2026-10-03): the selected asset · upload · the library (search, filters, grid) -->
		{#if chosen}
			{@const c = chosen}
			<section class="detail" data-testid="asset-detail" aria-label={c.title}>
				<div class="detail-head">
					<span class="img large">
						{#if thumb(c)}<img src={cdnAsset(thumb(c))} alt="" />{:else}<Icon name={icon(c.kind)} size="2.5em" />{/if}
					</span>
					<div class="detail-text">
						<span class="name" title={c.slug}>{c.title}</span>
						<span class="sc-caption path" title={c.path}>{c.slug}</span>
						<span class="sc-caption">{meta(c)} · {c.usages ? m['creator.assets_used_n']({ count: c.usages }) : m['creator.assets_unused']()}</span>
					</div>
					<CloseButton label={m['creator.characters_picker_back']()} onclick={() => (chosen = null)} />
				</div>
				<p class="sc-label">{m['creator.assets_renditions']()}</p>
				<ul class="files">
					{#each ordered(c.renditions) as r (r.id)}
						<li>
							<button type="button" class="file" class:current={r.path === value} disabled={!pickable(r)} data-file onclick={() => pick(c, r)} title={r.path} data-testid="asset-file-{r.label}">
								<span class="label">{r.quality ? `${r.label} · ${m[`creator.video_quality_${r.quality}`]()}` : r.label}</span>
								<span class="sc-caption">{meta(r)}</span>
							</button>
						</li>
					{/each}
				</ul>
				<button type="button" class="primary use" onclick={() => pick(c)} data-testid="asset-use">{m['creator.assets_use']()}</button>
			</section>
		{/if}

		<!-- upload: a button (owner 2026-10-03: the fold-out was confusing); the path appears once a file is chosen -->
		<div class="upload" data-testid="asset-upload">
			<button type="button" class="secondary" disabled={uploading} onclick={() => uploadInput?.click()} data-testid="asset-upload-choose">{m['creator.assets_upload']()} …</button>
			<input bind:this={uploadInput} type="file" hidden accept={kind === 'image' ? 'image/*' : kind === 'video' ? 'video/mp4,video/webm' : kind === 'audio' ? 'audio/*' : kind === 'archive' ? '.zip' : undefined} onchange={chooseFile} data-testid="asset-upload-file" />
			{#if uploadFile}
				<label class="sc-label" for="asset-upload-path">{m['creator.assets_upload_path']()}</label>
				<input id="asset-upload-path" type="text" bind:value={uploadPath} data-testid="asset-upload-path" />
				<button type="button" class="primary" disabled={uploading || !uploadPath} onclick={upload} data-testid="asset-upload-go">{uploading ? '…' : m['creator.assets_upload']()} · {bytes(uploadFile.size)}</button>
			{/if}
			{#if uploadError}<p class="error" role="alert">{uploadError}</p>{/if}
		</div>

		<div class="filters">
			<!-- svelte-ignore a11y_autofocus -->
			<input type="search" bind:value={query} oninput={refilter} placeholder={m['creator.assets_search']()} autofocus data-testid="asset-search" />
			<!-- one-way (value + onchange): a two-way bind wrote "" back while the options were still loading, so the folder /
			     kind of the field were lost on the next load (owner 2026-10-03) -->
			<select value={folder} onchange={(e) => { folder = e.currentTarget.value; load(1); }} aria-label={m['creator.assets_folder']()} data-testid="asset-folder">
				<option value="">{m['creator.assets_all_folders']()}</option>
				{#if folder && !folders.some((f) => (f.folder ?? '') === folder)}<option value={folder}>{folder}</option>{/if}
				{#each folders as f (f.folder ?? '')}<option value={f.folder ?? ''} selected={(f.folder ?? '') === folder}>{f.folder ?? '/'} ({f.assets})</option>{/each}
			</select>
			<select value={kindFilter} onchange={(e) => { kindFilter = e.currentTarget.value as AssetPickerKind; load(1); }} aria-label={m['creator.assets_kind']()} data-testid="asset-kind">
				<option value="">{m['creator.assets_all_kinds']()}</option>
				{#each ASSET_KINDS as k (k)}<option value={k} selected={k === kindFilter}>{m[`creator.assets_kind_${k}`]()}</option>{/each}
			</select>
			<label class="check"><input type="checkbox" checked={unusedOnly} onchange={(e) => { unusedOnly = e.currentTarget.checked; load(1); }} /> {m['creator.assets_unused_only']()}</label>
		</div>

		{#if error}<p class="error" role="alert">{error}</p>{/if}

		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="cards" onkeydown={arrows} bind:this={grid} aria-busy={loading}>
			{#each items as a (a.id)}
				{@const src = thumb(a)}
				<button
					type="button"
					class="card"
					class:selected={chosen?.id === a.id}
					class:current={a.renditions.some((r) => r.path === value)}
					data-cell
					onclick={() => choose(a)}
					ondblclick={() => pick(a)}
					title={`${a.slug}\n${meta(a)}`}
					data-testid="asset-card"
				>
					<span class="img">
						{#if src}<img src={cdnAsset(src)} alt="" loading="lazy" />{:else}<Icon name={icon(a.kind)} size="2em" />{/if}
					</span>
					<span class="name">{fileName(a.path)}</span>
					<span class="sc-caption meta">{a.width && a.height ? `${a.width}×${a.height}` : bytes(a.size)}{#if !a.usages} · {m['creator.assets_unused']()}{/if}</span>
					{#if a.renditions.length > 1}
						<span class="chips">{#each ordered(a.renditions).filter((r) => r.label !== 'original').slice(0, 5) as r (r.id)}<span class="chip">{r.label}</span>{/each}</span>
					{/if}
				</button>
			{:else}
				{#if !loading && !error}<p class="sc-caption">{m['creator.assets_none']()}</p>{/if}
			{/each}
		</div>
		{#if items.length < total}
			<button type="button" class="secondary more" disabled={loading} onclick={() => load(page + 1)}>{m['creator.assets_more']()} ({items.length} / {total})</button>
		{/if}

	</div>
</Sheet>

<style>
	.picker {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		width: 100%;
		max-width: 100%;
		min-width: 0;
		overflow-x: hidden;
	}
	.filters {
		display: grid;
		grid-template-columns: 2fr 1fr 1fr auto;
		gap: 0.5rem;
		align-items: center;
	}
	@media (max-width: 700px) {
		.filters {
			grid-template-columns: 1fr 1fr;
		}
	}
	.check {
		display: flex;
		gap: 0.25rem;
		align-items: center;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.error {
		margin: 0;
		border-left: 0.5rem solid var(--state-error);
		padding-left: 0.5rem;
	}
	/* the grid: fixed cells, one line of name – nothing overlaps (owner screenshot 2026-10-02) */
	.cards {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(8.5rem, 1fr));
		gap: 0.5rem;
	}
	.card,
	.file {
		all: unset;
		box-sizing: border-box;
		min-width: 0;
		cursor: pointer;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
		padding: 0.35rem;
		border: var(--border-width-regular) solid var(--border-muted);
		background: var(--paper);
		color: var(--ink);
	}
	.card:hover,
	.file:hover:not(:disabled) {
		border-color: var(--ink);
	}
	.card:focus-visible,
	.file:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
	.card.current {
		border-left: 0.5rem solid var(--primary-cyan);
	}
	.card.selected,
	.file.current {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.card.selected .sc-caption,
	.file.current .sc-caption {
		color: inherit;
	}
	.file:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}
	.img {
		aspect-ratio: 1;
		width: 100%;
		background: var(--paper);
		border: 1px solid var(--ink);
		display: flex;
		align-items: center;
		justify-content: center;
		overflow: hidden;
	}
	.img img {
		max-width: 100%;
		max-height: 100%;
		object-fit: contain;
	}
	.img.large {
		width: 6rem;
		flex: none;
	}
	.name,
	.path,
	.meta {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
		min-width: 0;
	}
	.chips {
		display: flex;
		flex-wrap: wrap;
		gap: 0.15rem;
	}
	.chip {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		background: var(--badge-neutral);
		color: var(--badge-neutral-text);
		padding: 0 0.3rem;
	}
	.detail {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		border: var(--border-width-regular) solid var(--ink);
		padding: 0.75rem;
	}
	.detail-head {
		display: flex;
		gap: 0.75rem;
		align-items: flex-start;
		min-width: 0;
	}
	.detail-text {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		flex: 1;
		min-width: 0;
	}
	.files {
		list-style: none;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(10rem, 1fr));
		gap: 0.4rem;
	}
	.use,
	.more {
		align-self: flex-start;
	}
	.upload {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.5rem;
		border-bottom: var(--border-width-regular) solid var(--ink);
		padding-bottom: 0.75rem;
	}
	.upload input[type='text'] {
		width: 100%;
		box-sizing: border-box;
	}
</style>
