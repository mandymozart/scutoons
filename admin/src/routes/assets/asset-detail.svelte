<script lang="ts">
	import { Badge, Button, DataTable, Section, type Column } from '@scutoons/ui';
	import type { AssetDeleteCheck, AssetDetail, AssetJob, AssetRendition, AssetUsage } from '@scutoons/api';
	import { ApiError, assets } from '$lib/api.ts';
	import { bytes, errorText, minute } from '$lib/format.ts';
	import { tableLabels } from '$lib/labels.ts';
	import { contentHref } from '$lib/creator-links.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { ModelPreview } from '@scutoons/media';

	/**
	 * One asset (agents/ASSETS.md "Operations"): preview, the files of every version, where it is used, and the
	 * operations – title / tags, replace a file in place (new version, purge), rename (copies + relinks every use),
	 * relink one file to another, optimization jobs, delete (only when nothing references it).
	 */
	let { id, cdn, onchange, ondeleted }: { id: string; cdn: string; onchange: () => void; ondeleted: () => void } = $props();

	let detail = $state<AssetDetail | null>(null);
	let error = $state<string | null>(null);
	let notice = $state('');
	let busy = $state('');
	let title = $state('');
	let tags = $state('');
	let tile = $state<'' | 'x' | 'y'>('');
	let replacePath = $state('');
	let replaceFile = $state<File | null>(null);
	let replaceNote = $state('');
	let newSlug = $state('');
	let publish = $state(false);
	let relinkFrom = $state('');
	let relinkTo = $state('');
	let jobKind = $state('');
	let jobLevels = $state<number[]>([480, 720, 1080]);
	let jobAt = $state(1);
	let check = $state<AssetDeleteCheck | null>(null);
	let force = $state(false);

	async function load() {
		try {
			detail = await assets.get(id);
			title = detail.asset.title;
			tags = detail.asset.tags.join(', ');
			tile = detail.asset.shape?.tile ?? '';
			newSlug = detail.asset.slug;
			const current = detail.versions.find((v) => v.status === 'current');
			replacePath = current?.path ?? '';
			relinkFrom = current?.renditions[0]?.path ?? '';
			jobKind = detail.asset.kind === 'video' ? 'video-renditions' : detail.asset.kind === 'image' ? 'image-webp' : '';
		} catch (e) {
			error = errorText(e);
		}
	}
	$effect(() => {
		void load();
	});

	const current = $derived(detail?.versions.find((v) => v.status === 'current') ?? null);
	const url = (path: string) => `${cdn}/${path}`;
	const preview = $derived.by(() => {
		if (!current || !detail) return null;
		const files = current.renditions;
		if (detail.asset.kind === 'video') return files.find((r) => r.quality === 'low') ?? files.find((r) => r.mime?.startsWith('video/')) ?? null;
		if (detail.asset.kind === 'image') return files.find((r) => r.mime?.startsWith('image/')) ?? null;
		if (detail.asset.kind === 'audio') return files.find((r) => r.quality === 'medium') ?? files[0] ?? null;
		if (detail.asset.kind === 'model') return files.find((r) => r.label === 'original') ?? files[0] ?? null;
		return null;
	});

	async function run(name: string, work: () => Promise<string | void>) {
		busy = name;
		error = null;
		notice = '';
		try {
			notice = (await work()) || m['assets.done']();
			await load();
			onchange();
		} catch (e) {
			error = e instanceof ApiError && e.body?.usages ? `${e.message} (${e.body.usages.length})` : errorText(e);
		} finally {
			busy = '';
		}
	}

	const saveShape = () =>
		run('shape', async () => {
			const shape = detail?.asset.shape;
			if (shape) await assets.update(id, { shape: { ...shape, tile: tile || null } });
		});
	const saveMeta = () => run('meta',async () => void (await assets.update(id, { title, tags: tags.split(',').map((t) => t.trim()).filter(Boolean) })));
	const replace = () =>
		run('replace', async () => {
			if (!replaceFile) return;
			const r = await assets.replace(id, replaceFile, replacePath, replaceNote || undefined);
			replaceFile = null;
			return m['assets.replaced']({ path: r.replaced, purged: r.purged ? m['common.yes']() : m['common.no'](), stale: r.stale_renditions.length });
		});
	const rename = () =>
		run('rename', async () => {
			const r = await assets.rename(id, newSlug.trim(), publish);
			return m['assets.renamed']({ files: Object.keys(r.renamed).length, uses: r.relink.usages, drafts: r.relink.content.length });
		});
	const relink = () =>
		run('relink', async () => {
			const r = await assets.relink(relinkFrom, relinkTo.trim(), publish);
			return m['assets.relinked']({ uses: r.usages, docs: r.content.length + r.catalog.length, manual: r.manual.length });
		});
	const enqueue = () =>
		run('job', async () => {
			const params = jobKind === 'video-renditions' ? { levels: jobLevels } : jobKind === 'poster' ? { at: jobAt } : {};
			const r = await assets.enqueue(id, jobKind, params);
			return m['assets.job_queued']({ id: r.job.id });
		});
	async function deleteCheck() {
		busy = 'check';
		try {
			check = await assets.deleteCheck(id);
		} catch (e) {
			error = errorText(e);
		} finally {
			busy = '';
		}
	}
	async function remove() {
		busy = 'delete';
		try {
			await assets.remove(id, force);
			ondeleted();
		} catch (e) {
			error = e instanceof ApiError && e.code === 'in_use' ? m['assets.delete_refused']() : errorText(e);
			await deleteCheck();
		} finally {
			busy = '';
		}
	}

	const fileColumns = $derived<Column[]>([
		{ id: 'label', label: m['assets.col_file']() },
		{ id: 'path', label: m['assets.col_path']() },
		{ id: 'size', label: m['assets.col_size'](), align: 'end' },
		{ id: 'dims', label: m['assets.col_dims']() },
		{ id: 'origin', label: m['assets.col_origin']() }
	]);
	const usageColumns = $derived<Column[]>([
		{ id: 'ref', label: m['assets.col_used_by']() },
		{ id: 'field', label: m['assets.col_field']() },
		{ id: 'state', label: m['assets.col_status']() }
	]);
	const dims = (r: AssetRendition) =>
		[r.width && r.height ? `${r.width}×${r.height}` : '', r.duration ? `${Math.floor(r.duration / 60)}:${String(Math.round(r.duration % 60)).padStart(2, '0')}` : ''].filter(Boolean).join(' · ') || '—';
	const stateTone = (u: AssetUsage) => (u.state === 'published' ? 'published' : u.state === 'config' ? 'info' : 'draft');
	/** a content use → the creator page that edits it */
	const editHref = (u: AssetUsage): string | null => (u.source === 'content' ? contentHref(u.ref) : null);
	const jobTone = (j: AssetJob) => (j.status === 'done' ? 'published' : j.status === 'failed' ? 'error' : 'draft');
</script>

<div class="detail" data-testid="asset-detail">
	{#if error}<p class="error-box" role="alert">{error}</p>{/if}
	{#if notice}<p class="notice" role="status">{notice}</p>{/if}
	{#if !detail}
		<p>{m['shell.loading']()}</p>
	{:else}
		<header class="head">
			<div class="preview">
				{#if preview && detail.asset.kind === 'image'}<img src={url(preview.path)} alt="" />
				{:else if preview && detail.asset.kind === 'video'}<!-- svelte-ignore a11y_media_has_caption --><video src={url(preview.path)} controls muted preload="metadata"></video>
				{:else if preview && detail.asset.kind === 'audio'}<audio src={url(preview.path)} controls preload="none"></audio>
				{:else if preview && detail.asset.kind === 'model'}<ModelPreview path={preview.path} testid="asset-model-preview" />{/if}
			</div>
			<div class="text">
				<p class="sc-label">{detail.asset.kind} · v{current?.number ?? '–'} · {detail.asset.folder ?? '/'}</p>
				<p class="slug"><code>{detail.asset.slug}</code></p>
				<p class="sc-caption">{m['assets.updated']({ when: minute(detail.asset.updated_at) })}</p>
			</div>
		</header>

		<Section title={m['assets.meta_title']()}>
			<div class="sc-fields" data-cols="2">
				<label class="field"><span class="sc-label">{m['assets.field_title']()}</span><input type="text" bind:value={title} /></label>
				<label class="field"><span class="sc-label">{m['assets.field_tags']()}</span><input type="text" bind:value={tags} placeholder="cover, prologue" /></label>
			</div>
			<Button size="sm" variant="secondary" disabled={busy === 'meta'} onclick={saveMeta}>{m['assets.save']()}</Button>
		</Section>

		{#if detail.asset.shape}
			{@const shape = detail.asset.shape}
			<!-- the vector library (research/VECTORS.md): the shape's facts; colour is a preset the page applies, not edited here -->
			<Section title={m['assets.shape_title']()}>
				<label class="field"><span class="sc-label">{m['assets.shape_tile']()}</span>
					<select bind:value={tile}>
						<option value="">{m['assets.shape_tile_none']()}</option>
						<option value="x">{m['assets.shape_tile_x']()}</option>
						<option value="y">{m['assets.shape_tile_y']()}</option>
					</select>
				</label>
				<p class="sc-caption">
					{shape.mask ? m['assets.shape_mask']() : m['assets.shape_asis']()}
					{#each shape.colors as c (c)}<span class="swatch" style:background={c} title={c}></span>{/each}
				</p>
				{#if shape.source}<p class="sc-caption">{m['assets.shape_source']()} {[shape.source.file, shape.source.page, shape.source.node].filter(Boolean).join(' · ')}</p>{/if}
				<Button size="sm" variant="secondary" disabled={busy === 'shape'} onclick={saveShape}>{m['assets.save']()}</Button>
			</Section>
		{/if}

		{#each detail.versions as v (v.id)}
			<Section title={m['assets.version']({ number: v.number })}>
				<p class="sc-caption">
					<Badge tone={v.status === 'current' ? 'published' : 'draft'}>{v.status}</Badge>
					{minute(v.created_at)} · {v.storage}{v.note ? ` · ${v.note}` : ''}{v.archive_path ? ` · ${m['assets.archived_as']()} ${v.archive_path}` : ''}
				</p>
				<DataTable columns={fileColumns} rows={v.renditions} rowKey={(r) => String(r.id)} labels={tableLabels()}>
					{#snippet cell(r, column)}
						{#if column.id === 'label'}{r.label}{#if r.quality} <Badge tone="info">{r.quality}</Badge>{/if}
						{:else if column.id === 'path'}<a href={url(r.path)} target="_blank" rel="noreferrer">{r.path}</a>
						{:else if column.id === 'size'}{bytes(r.size)}
						{:else if column.id === 'dims'}{dims(r)}
						{:else}{r.origin}{/if}
					{/snippet}
				</DataTable>
			</Section>
		{/each}

		<Section title={m['assets.usages_title']({ count: detail.usages.length })}>
			<DataTable columns={usageColumns} rows={detail.usages} rowKey={(u) => `${u.ref}|${u.field}|${u.state}`} labels={tableLabels()}>
				{#snippet cell(u, column)}
					{#if column.id === 'ref'}{#if editHref(u)}<a href={editHref(u)} target="_blank" rel="noreferrer">{u.ref}</a>{:else}{u.ref}{/if}
					{:else if column.id === 'field'}<code title={u.raw}>{u.field}</code> <span class="sc-caption">{u.path.split('/').pop()}</span>
					{:else}<Badge tone={stateTone(u)}>{u.state}</Badge>{/if}
				{/snippet}
			</DataTable>
		</Section>

		{#if current}
			<Section title={m['assets.replace_title']()}>
				<p class="sc-caption">{m['assets.replace_hint']()}</p>
				<div class="sc-fields" data-cols="2">
					<label class="field"><span class="sc-label">{m['assets.col_file']()}</span>
						<select bind:value={replacePath}>{#each current.renditions as r (r.id)}<option value={r.path}>{r.label} – {r.path.split('/').pop()}</option>{/each}</select>
					</label>
					<label class="field"><span class="sc-label">{m['assets.new_file']()}</span><input type="file" onchange={(e) => (replaceFile = e.currentTarget.files?.[0] ?? null)} data-testid="asset-replace-file" /></label>
				</div>
				<label class="field"><span class="sc-label">{m['assets.note']()}</span><input type="text" bind:value={replaceNote} /></label>
				<Button size="sm" variant="secondary" disabled={!replaceFile || busy === 'replace'} onclick={replace} data-testid="asset-replace">{m['assets.replace']()}</Button>
			</Section>

			<Section title={m['assets.rename_title']()}>
				<p class="sc-caption">{m['assets.rename_hint']()}</p>
				<label class="field"><span class="sc-label">{m['assets.new_name']()}</span><input type="text" bind:value={newSlug} data-testid="asset-rename-slug" /></label>
				<label class="check"><input type="checkbox" bind:checked={publish} /> {m['assets.publish_too']()}</label>
				<Button size="sm" variant="secondary" disabled={busy === 'rename' || newSlug.trim() === detail.asset.slug} onclick={rename}>{m['assets.rename']()}</Button>
			</Section>

			<Section title={m['assets.relink_title']()}>
				<p class="sc-caption">{m['assets.relink_hint']()}</p>
				<div class="sc-fields" data-cols="2">
					<label class="field"><span class="sc-label">{m['assets.relink_from']()}</span>
						<select bind:value={relinkFrom}>{#each current.renditions as r (r.id)}<option value={r.path}>{r.path}</option>{/each}</select>
					</label>
					<label class="field"><span class="sc-label">{m['assets.relink_to']()}</span><input type="text" bind:value={relinkTo} placeholder="catpope/s1/c0/other.720p.mp4" /></label>
				</div>
				<Button size="sm" variant="secondary" disabled={!relinkTo.trim() || busy === 'relink'} onclick={relink}>{m['assets.relink']()}</Button>
			</Section>

			{#if jobKind}
				<Section title={m['assets.optimize_title']()}>
					<p class="sc-caption">{m['assets.optimize_hint']()}</p>
					<label class="field"><span class="sc-label">{m['assets.job_kind']()}</span>
						<select bind:value={jobKind}>
							{#if detail.asset.kind === 'video'}<option value="video-renditions">{m['assets.job_video']()}</option><option value="poster">{m['assets.job_poster']()}</option>{/if}
							{#if detail.asset.kind === 'image'}<option value="image-webp">{m['assets.job_webp']()}</option>{/if}
						</select>
					</label>
					{#if jobKind === 'video-renditions'}
						<p class="levels">{#each [480, 720, 1080] as level (level)}<label class="check"><input type="checkbox" checked={jobLevels.includes(level)} onchange={(e) => (jobLevels = e.currentTarget.checked ? [...jobLevels, level].sort((a, b) => a - b) : jobLevels.filter((l) => l !== level))} /> {level}p</label>{/each}</p>
					{:else if jobKind === 'poster'}
						<label class="field"><span class="sc-label">{m['assets.poster_at']()}</span><input type="number" min="0" step="0.1" bind:value={jobAt} /></label>
					{/if}
					<Button size="sm" variant="secondary" disabled={busy === 'job'} onclick={enqueue}>{m['assets.queue_job']()}</Button>
					{#if detail.jobs.length}
						<ul class="jobs">{#each detail.jobs as j (j.id)}<li>#{j.id} {j.kind} <Badge tone={jobTone(j)}>{j.status}</Badge> <span class="sc-caption">{minute(j.created_at)}</span></li>{/each}</ul>
					{/if}
				</Section>
			{/if}
		{/if}

		<Section title={m['assets.delete_title']()}>
			<p class="sc-caption">{m['assets.delete_hint']()}</p>
			{#if !check}
				<Button size="sm" variant="secondary" disabled={busy === 'check'} onclick={deleteCheck} data-testid="asset-delete-check">{m['assets.delete_check']()}</Button>
			{:else}
				{#if check.can_delete}
					<p>{m['assets.delete_ok']({ files: check.files.length })}</p>
				{:else}
					<p class="blocked"><strong class="sc-emphasis">{m['assets.delete_blocked']({ blocking: check.blocking, drafts: check.drafts })}</strong></p>
					{#if check.blocking === 0}<label class="check"><input type="checkbox" bind:checked={force} /> {m['assets.delete_force']()}</label>{/if}
				{/if}
				<p class="sc-caption">{check.note}</p>
				<Button size="sm" variant="secondary" disabled={busy === 'delete' || (!check.can_delete && (check.blocking > 0 || !force))} onclick={remove} data-testid="asset-delete">{m['assets.delete']()}</Button>
			{/if}
		</Section>
	{/if}
</div>

<style>
	.detail {
		display: flex;
		flex-direction: column;
		gap: var(--space-15);
		max-width: 52rem;
		min-width: 0;
	}
	.head {
		display: flex;
		gap: var(--space-1);
		align-items: flex-start;
		flex-wrap: wrap;
	}
	.preview {
		width: 14rem;
		max-width: 100%;
		border: var(--border-width-regular) solid var(--ink);
		display: flex;
		justify-content: center;
	}
	.preview img,
	.preview video {
		max-width: 100%;
		max-height: 14rem;
		display: block;
	}
	.preview audio {
		width: 100%;
	}
	.text {
		min-width: 0;
		flex: 1;
	}
	.text p {
		margin: 0 0 var(--space-025);
	}
	.slug {
		overflow-wrap: anywhere;
	}
	@media (max-width: 700px) {
		.sc-fields[data-cols] {
			grid-template-columns: minmax(0, 1fr);
		}
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		margin-bottom: var(--space-1);
	}
	.field input,
	.field select {
		width: 100%;
		box-sizing: border-box;
	}
	.check {
		display: inline-flex;
		gap: var(--space-025);
		align-items: center;
		margin-right: var(--space-1);
	}
	.levels {
		margin: 0 0 var(--space-1);
	}
	.swatch {
		display: inline-block;
		width: 0.75rem;
		height: 0.75rem;
		margin-left: var(--space-025);
		vertical-align: middle;
		border: var(--border-width-regular) solid var(--ink);
	}
	.jobs {
		list-style: none;
		padding: 0;
	}
	.notice {
		border-left: 0.5rem solid var(--primary-cyan);
		padding-left: var(--space-05);
	}
	.blocked {
		border-left: 0.5rem solid var(--state-error);
		padding-left: var(--space-05);
	}
</style>
