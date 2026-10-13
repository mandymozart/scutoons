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
	import { Button, CloseButton, confirmDialog, gridArrows, Icon, Sheet } from '@scutoons/ui';
	import { mediaAssets, mediaText, mediaUrl as cdnAsset } from './setup.ts';
	import { AUDIO_QUALITY_LABELS, type MediaQuality } from '@scutoons/content';
	import { bestAudioSource, decodeAudio, encodeMp3, nearestKbps, planAudioUpload, type AudioUploadPlan } from './audio-levels.ts';
	import { uploadOrReplace } from './media-upload.ts';
	import { checkMedia, standardName, toStandard } from './media-standard.ts';

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
		onpick,
		webp = false
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
		/**
		 * uploads are ALWAYS brought to the media standard (media-standard.ts: a PNG / JPEG becomes a WebP – same name with
		 * .webp, transparency kept; SVG and WebP as they are) – owner 2026-10-11, first for the map's layers. Without it the
		 * standard is the default for every upload and can be unticked (owner 2026-10-11: "die Leute können andere
		 * benutzen, aber wir empfehlen das Umkonvertieren")
		 */
		webp?: boolean;
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
	const assetsApi = mediaAssets();
	const t = $derived(mediaText());

	// the folder it opens on: the toon's, else a character field's character with its personas (place
	// characters/catpope/ → "characters/catpope", a path prefix: catpope/, catpope-human/ …; owner 2026-10-05: a search
	// for "human" found nothing under "catpope" – the toon folder, character images all live under characters/)
	const home = $derived(toon || (/^characters\/[^/]+\/$/.test(place) ? place.slice(0, -1) : ''));

	// every time it opens: the home folder, the field's kind, the current file preselected
	$effect(() => {
		if (!open) return;
		folder = home;
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
			error = e instanceof ApiError && e.status === 503 ? t.unavailable : e instanceof Error ? e.message : String(e);
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
	const icon = (k: AssetKind) => ({ image: 'image', video: 'film', audio: 'music', archive: 'archive', document: 'file-text', model: 'box', other: 'file' })[k];

	/** the asset's own file (its current path) – the size a reader loads, not the master's (BACKLOG 10.20) */
	const mainSize = (a: { path?: string; size: number; renditions?: AssetRendition[] }) => a.renditions?.find((r) => r.path === a.path)?.size || a.size;
	function meta(a: { width: number | null; height: number | null; duration: number | null; size: number; path?: string; renditions?: AssetRendition[] }) {
		const parts: string[] = [];
		if (a.width && a.height) parts.push(`${a.width} × ${a.height}`);
		if (a.duration) parts.push(`${Math.floor(a.duration / 60)}:${String(Math.round(a.duration % 60)).padStart(2, '0')}`);
		const files = a.renditions?.length ?? 0;
		parts.push(files > 1 ? `${bytes(mainSize(a))} (${t.allFiles(files, bytes(a.size))})` : bytes(a.size));
		return parts.join(' · ');
	}
	/** a field that takes one file: the rendition matching the kind (an image field never gets the mp4) */
	const pickable = (r: AssetRendition) => !kind || (kind === 'image' ? !!r.mime?.startsWith('image/') : kind === 'video' ? !!r.mime?.startsWith('video/') : true);

	/**
	 * "Use" without a file: the field's current file when it is one of this asset's, else for audio the medium stream
	 * (128k – the reader plays exactly the path, audio has no quality variants yet; the source is the 320k), else the source.
	 */
	function pick(a: AssetSummary, rendition: AssetRendition | null = null) {
		const r =
			rendition ??
			a.renditions.find((x) => x.path === value && pickable(x)) ??
			(a.kind === 'audio' ? a.renditions.find((x) => x.quality === 'medium') : undefined) ??
			a.renditions.find((x) => x.path === a.path && pickable(x)) ??
			a.renditions.find(pickable) ??
			null;
		onpick({ asset: a, rendition: r, path: r?.path ?? a.path });
		open = false;
	}

	function choose(a: AssetSummary) {
		chosen = a;
		queueMicrotask(() => document.querySelector<HTMLElement>('[data-testid="asset-detail"] [data-file]')?.focus());
	}

	// ── upload: a new file into its place in the toon hierarchy (agents/ASSETS.md "Folder convention") ─────────────────
	/** without a `place`: "s1c2-slice-14.webp" → "s1/c2", "s1-cover.webp" → "s1", anything else → the toon's own folder (editable) */
	const placeOf = (name: string) => {
		const chapter = name.match(/^s(\d+)c(\d+)[-_]/);
		if (chapter) return `s${chapter[1]}/c${chapter[2]}/`;
		const season = name.match(/^s(\d+)[-_]/);
		return season ? `s${season[1]}/` : '';
	};
	// ── audio: the quality levels below the uploaded file, encoded here (audio-levels.ts) ──
	let uploadAudio = $state<AudioBuffer | null>(null);
	let audioProgress = $state('');
	const isAudioFile = (file: File) => file.type.startsWith('audio/') || /\.(mp3|m4a|aac|ogg|opus|wav|wave|flac|aiff?)$/i.test(file.name);
	const uploadPlan = $derived<AudioUploadPlan | null>(uploadFile && uploadAudio && uploadPath ? planAudioUpload(uploadPath.trim(), uploadFile.size, uploadAudio.duration) : null);
	const levelNames = (levels: { level: MediaQuality }[]) => levels.map((l) => AUDIO_QUALITY_LABELS[l.level]).join(', ');

	/** encode the plan's levels from `audio` and upload them (a file already there gets the new bytes) */
	async function makeLevels(audio: AudioBuffer, plan: AudioUploadPlan) {
		for (const l of plan.levels) {
			const label = AUDIO_QUALITY_LABELS[l.level];
			const blob = await encodeMp3(audio, l.kbps, (share) => (audioProgress = t.audioEncoding(label, Math.round(share * 100))));
			audioProgress = t.audioUploading(l.path.split('/').pop() ?? l.path);
			await uploadOrReplace(blob, l.path, `${label} encoded in the browser`);
		}
		audioProgress = '';
	}

	/** an existing audio asset: its missing levels from its best file (fetched from the CDN, which sends CORS headers) */
	const missingPlan = (a: AssetSummary): { plan: AudioUploadPlan; source: AssetRendition } | null => {
		if (a.kind !== 'audio') return null;
		const source = bestAudioSource(a.renditions);
		if (!source) return null;
		const rate = Number(source.label.match(/^(\d+)k$/)?.[1] ?? 0) || (source.size && source.duration ? nearestKbps((source.size * 8) / source.duration / 1000) : 0);
		const existing = Object.fromEntries(a.renditions.filter((r) => r.quality).map((r) => [r.quality, r.path]));
		// the plan from the source's path and its known rate (size / duration chosen so that the estimate is that rate)
		const plan = planAudioUpload(source.path, rate * 125, rate ? 1 : 0, existing);
		return plan.levels.length ? { plan, source } : null;
	};
	let making = $state(false);
	async function makeMissing(a: AssetSummary) {
		const missing = missingPlan(a);
		if (!missing) return;
		making = true;
		uploadError = '';
		try {
			audioProgress = '…';
			const response = await fetch(cdnAsset(missing.source.path));
			if (!response.ok) throw new Error(`${missing.source.path}: ${response.status}`);
			const audio = await decodeAudio(await response.arrayBuffer());
			await makeLevels(audio, missing.plan);
			await load(1, value || missing.source.path);
			const fresh = items.find((x) => x.id === a.id);
			if (fresh) chosen = fresh;
		} catch (e) {
			uploadError = e instanceof Error ? e.message : String(e);
			audioProgress = '';
		} finally {
			making = false;
		}
	}

	/** the chosen file against the media standard; `standard` = convert it (always with `webp`, else the default) */
	const verdict = $derived(uploadFile ? checkMedia(uploadFile) : null);
	let standard = $state(true);
	const converting = $derived(!!verdict?.convertible && (webp || standard));
	/** the file in the SCUTOONS media standard (media-standard.ts – PNG / JPEG → WebP) when converting, else as it is */
	const asWebp = (file: File): Promise<File> => (converting ? toStandard(file) : Promise.resolve(file));
	/** the path's extension follows the choice (`a.png` ⇄ `a.webp`) */
	function setStandard(on: boolean) {
		standard = on;
		if (!uploadFile) return;
		const original = uploadFile.name.match(/\.[a-z0-9]+$/i)?.[0].toLowerCase() ?? '';
		uploadPath = on ? standardName(uploadPath, uploadFile) : uploadPath.replace(/\.webp$/i, original);
	}

	function chooseFile(event: Event) {
		const file = (event.currentTarget as HTMLInputElement).files?.[0] ?? null;
		uploadFile = file;
		uploadError = '';
		uploadAudio = null;
		if (file && isAudioFile(file)) {
			file.arrayBuffer()
				.then(decodeAudio)
				.then((audio) => {
					if (uploadFile === file) uploadAudio = audio;
				})
				.catch(() => {
					if (uploadFile === file) uploadError = t.audioUndecodable;
				});
		}
		if (file) {
			let clean = file.name.toLowerCase().replace(/[^a-z0-9._-]+/g, '-').replace(/-+/g, '-');
			standard = true;
			clean = standardName(clean, file);
			// the field's place first (owner 2026-10-03: uploads go next to where they are used, by the content rules)
			uploadPath = place ? `${place.replace(/\/?$/, '/')}${clean}` : `${folder || toon || 'scutoons'}/${placeOf(clean)}${clean}`;
		}
	}
	async function upload() {
		if (!uploadFile || !uploadPath) return;
		uploading = true;
		uploadError = '';
		const plan = uploadPlan;
		const audio = uploadAudio;
		// audio: the original under its rate's name (s1c1-bells.320.mp3); a lossless source isn't uploaded itself
		const path = plan ? (plan.original ?? '') : uploadPath.trim();
		try {
			let uploaded = '';
			if (path) {
				try {
					uploaded = (await assetsApi.upload(await asWebp(uploadFile), path)).uploaded;
				} catch (e) {
					if (!(e instanceof ApiError && e.code === 'exists')) throw e;
					// a new version at the same path (owner 2026-10-03: updating media meant renaming): replace in place, the old
					// bytes stay as the asset's previous version
					if (!(await confirmDialog({ title: t.replaceTitle, message: t.replaceMessage(path), confirmLabel: t.replace }))) return;
					uploaded = (await assetsApi.replaceAt(await asWebp(uploadFile), path)).replaced;
				}
			}
			if (plan && audio && plan.levels.length) {
				await makeLevels(audio, plan);
				// the medium file is what a field takes by default (the picker's "Use")
				uploaded = plan.levels.find((l) => l.level === 'medium')?.path || uploaded || plan.levels[0].path;
			}
			query = '';
			await load(1, uploaded);
			uploadFile = null;
			uploadAudio = null;
		} catch (e) {
			uploadError = e instanceof ApiError && e.code === 'unchanged' ? t.replaceUnchanged : e instanceof Error ? e.message : String(e);
			audioProgress = '';
		} finally {
			uploading = false;
		}
	}
</script>

<Sheet bind:open title={title ?? t.pickerTitle} side="right" widthKey="assets" closeLabel={t.close} resizeLabel={t.resize}>
	<div class="picker" data-testid="asset-picker">
		<!-- order (owner 2026-10-03): the selected asset · upload · the library (search, filters, grid) -->
		{#if chosen}
			{@const c = chosen}
			<section class="detail" data-testid="asset-detail" aria-label={c.title}>
				<div class="detail-head">
					<span class="sc-card-img large">
						{#if thumb(c)}<img src={cdnAsset(thumb(c))} alt="" />{:else}<Icon name={icon(c.kind)} size="2.5em" />{/if}
					</span>
					<div class="detail-text">
						<span class="name" title={c.slug}>{c.title}</span>
						<span class="sc-caption path" title={c.path}>{c.slug}</span>
						<span class="sc-caption">{meta(c)} · {c.usages ? t.usedN(c.usages) : t.unused}</span>
					</div>
					<CloseButton label={t.back} onclick={() => (chosen = null)} />
				</div>
				<p class="sc-label">{t.renditions}</p>
				<ul class="files">
					{#each ordered(c.renditions) as r (r.id)}
						<li>
							<button type="button" class="sc-card file" class:is-selected={r.path === value} disabled={!pickable(r)} data-file onclick={() => pick(c, r)} title={r.path} data-testid="asset-file-{r.label}">
								<span class="label">{r.quality ? `${r.label} · ${t.quality[r.quality]}` : r.label}</span>
								<span class="sc-caption">{meta(r)}</span>
							</button>
						</li>
					{/each}
				</ul>
				{#if missingPlan(c)}
					{@const missing = missingPlan(c)!}
					<Button size="sm" variant="secondary" disabled={making || uploading} onclick={() => makeMissing(c)} title={t.audioLevelsHint} data-testid="asset-audio-levels">{making ? audioProgress || '…' : t.audioLevelsMake(levelNames(missing.plan.levels))}</Button>
				{/if}
				<Button size="sm" class="use" onclick={() => pick(c)} data-testid="asset-use">{t.use}</Button>
			</section>
		{/if}

		<!-- upload: a button (owner 2026-10-03: the fold-out was confusing); the path appears once a file is chosen -->
		<div class="upload" data-testid="asset-upload">
			<Button size="sm" variant="secondary" disabled={uploading} onclick={() => uploadInput?.click()} data-testid="asset-upload-choose">{t.upload} …</Button>
			<input bind:this={uploadInput} type="file" hidden accept={kind === 'image' ? 'image/*' : kind === 'video' ? 'video/mp4,video/webm' : kind === 'audio' ? 'audio/*' : kind === 'archive' ? '.zip' : kind === 'model' ? '.glb' : undefined} onchange={chooseFile} data-testid="asset-upload-file" />
			{#if uploadFile}
				<label class="sc-label" for="asset-upload-path">{t.uploadPath}</label>
				<input id="asset-upload-path" type="text" bind:value={uploadPath} data-testid="asset-upload-path" />
				{#if verdict?.convertible && !webp}
					<label class="check" data-testid="asset-upload-standard"><input type="checkbox" checked={standard} onchange={(e) => setStandard(e.currentTarget.checked)} /> {t.uploadStandard(verdict.advice ?? '')}</label>
				{:else if verdict && !verdict.ok}
					<p class="sc-caption" data-testid="asset-upload-off-standard">{t.uploadOffStandard(verdict.advice ?? '')}</p>
				{/if}
				{#if uploadPlan}
					<p class="sc-caption" data-testid="asset-upload-levels">
						{#if !uploadPlan.original}{t.audioLossless}<br />{/if}
						{#if uploadPlan.original}{uploadPlan.original.split('/').pop()}{/if}{#if uploadPlan.levels.length}{uploadPlan.original ? ' · ' : ''}{t.audioLevelsPlan(levelNames(uploadPlan.levels))}{/if}
					</p>
				{/if}
				<Button size="sm" disabled={uploading || !uploadPath} onclick={upload} data-testid="asset-upload-go">{uploading ? audioProgress || '…' : t.upload} · {bytes(uploadFile.size)}</Button>
			{/if}
			{#if uploadError}<p class="error" role="alert">{uploadError}</p>{/if}
		</div>

		<div class="sc-picker-filters">
			<!-- svelte-ignore a11y_autofocus -->
			<input type="search" bind:value={query} oninput={refilter} placeholder={t.search} autofocus data-testid="asset-search" />
			<!-- one-way (value + onchange): a two-way bind wrote "" back while the options were still loading, so the folder /
			     kind of the field were lost on the next load (owner 2026-10-03) -->
			<select value={folder} onchange={(e) => { folder = e.currentTarget.value; load(1); }} aria-label={t.folder} data-testid="asset-folder">
				<option value="">{t.allFolders}</option>
				{#if home && !folders.some((f) => (f.folder ?? '') === home)}<option value={home}>{home}</option>{/if}
				{#each folders as f (f.folder ?? '')}<option value={f.folder ?? ''} selected={(f.folder ?? '') === folder}>{f.folder ?? '/'} ({f.assets})</option>{/each}
			</select>
			<select value={kindFilter} onchange={(e) => { kindFilter = e.currentTarget.value as AssetPickerKind; load(1); }} aria-label={t.kind} data-testid="asset-kind">
				<option value="">{t.allKinds}</option>
				{#each ASSET_KINDS as k (k)}<option value={k} selected={k === kindFilter}>{t.kinds[k]}</option>{/each}
			</select>
			<label class="check"><input type="checkbox" checked={unusedOnly} onchange={(e) => { unusedOnly = e.currentTarget.checked; load(1); }} /> {t.unusedOnly}</label>
		</div>

		{#if error}<p class="error" role="alert">{error}</p>{/if}

		<!-- svelte-ignore a11y_no_static_element_interactions -->
		<div class="sc-cards" onkeydown={gridArrows} bind:this={grid} aria-busy={loading}>
			{#each items as a (a.id)}
				{@const src = thumb(a)}
				<button
					type="button"
					class="sc-card"
					class:is-selected={chosen?.id === a.id}
					class:is-current={a.renditions.some((r) => r.path === value)}
					data-cell
					onclick={() => choose(a)}
					ondblclick={() => pick(a)}
					title={`${a.slug}\n${meta(a)}`}
					data-testid="asset-card"
				>
					<span class="sc-card-img">
						{#if src}<img src={cdnAsset(src)} alt="" loading="lazy" />{:else}<Icon name={icon(a.kind)} size="2em" />{/if}
					</span>
					<span class="name">{fileName(a.path)}</span>
					<span class="sc-caption meta">{a.width && a.height ? `${a.width}×${a.height}` : bytes(mainSize(a))}{#if !a.usages} · {t.unused}{/if}</span>
					{#if a.renditions.length > 1}
						<span class="chips">{#each ordered(a.renditions).filter((r) => r.label !== 'original').slice(0, 5) as r (r.id)}<span class="sc-chip">{r.label}</span>{/each}</span>
					{/if}
				</button>
			{:else}
				{#if !loading && !error}<p class="sc-caption">{t.none}</p>{/if}
			{/each}
		</div>
		{#if items.length < total}
			<Button size="sm" variant="secondary" class="more" disabled={loading} onclick={() => load(page + 1)}>{t.more} ({items.length} / {total})</Button>
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
	/* the cards are tools.css .sc-card (the picker sheets' vocabulary); here the picture has a hairline frame */
	.sc-card-img {
		background: var(--paper);
		border: 1px solid var(--ink);
	}
	/* selected (the card inverts): the picture box too – a kind icon in the card's colour stays visible (BACKLOG 10.20) */
	.sc-card.is-selected .sc-card-img {
		background: var(--ink);
		border-color: var(--paper);
	}
	.sc-card-img.large {
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
	.picker :global(.use),
	.picker :global(.more) {
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
