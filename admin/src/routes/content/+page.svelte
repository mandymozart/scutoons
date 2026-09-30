<script lang="ts">
	import { fileLabel } from '$lib/labels.ts';
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { ActionBar, Badge, Button, Section, Skeleton, ToonAvatar } from '@scutoons/ui';
	import type { AdminContentEntity, AdminContentIndex } from '@scutoons/api';
	import { entityOf, parseEntityKey } from '@scutoons/content';
	import { admin } from '$lib/api.ts';
	import { CREATOR_URL, toonAvatarUrl } from '$lib/config.ts';
	import { errorText, minute } from '$lib/format.ts';
	import { m } from '$lib/paraglide/messages.js';
	import PageHeader from '$lib/components/page-header.svelte';

	/** Read-only overview of the content store (GET /admin/content); editing stays in the creator. */
	let data = $state<AdminContentIndex | null>(null);
	let error = $state<string | null>(null);
	const toonFilter = $derived(page.url.searchParams.get('toon') ?? '');
	const stateFilter = $derived(page.url.searchParams.get('state') ?? '');

	$effect(() => {
		admin
			.content()
			.then((r) => (data = r))
			.catch((e) => (error = errorText(e)));
	});

	type FileState = 'published' | 'ahead' | 'draft';
	const fileState = (f: AdminContentEntity): FileState =>
		f.published === null ? 'draft' : f.published === f.latest ? 'published' : 'ahead';

	/** the column of a stored entity: its locale, `·` for language-neutral ones */
	const NEUTRAL = '·';
	const column = (e: AdminContentEntity) => e.locale ?? NEUTRAL;

	/** toon (then the platform-wide entities) → entity → locale → stored entity */
	const groups = $derived.by(() => {
		if (!data) return [];
		const scopes: { id: string | null; name: string }[] = [
			...data.toons.filter((t) => !toonFilter || t.id === toonFilter),
			...(!toonFilter ? [{ id: null, name: m['content.platform']() }] : [])
		];
		return scopes.map((toon) => {
			const all = data!.entities.filter((e) => e.toon === toon.id && parseEntityKey(e.key));
			const shown = all.filter((e) => !stateFilter || fileState(e) === stateFilter);
			const locales = [...new Set(all.map(column))].sort((a, b) => (a === NEUTRAL ? -1 : b === NEUTRAL ? 1 : a.localeCompare(b)));
			const ids = [...new Set(shown.map((e) => entityOf(e.key)))];
			return {
				toon,
				locales,
				rows: ids.map((entity) => ({ entity, byLocale: Object.fromEntries(shown.filter((e) => entityOf(e.key) === entity).map((e) => [column(e), e])) }))
			};
		});
	});

	/** chapters and their scenes open the chapter editor, characters the characters page, everything else the toon page */
	function creatorLink(entity: string) {
		const ref = parseEntityKey(entity);
		if (!ref) return CREATOR_URL;
		if ((ref.kind === 'chapter' || ref.kind === 'scene') && ref.toon) return `${CREATOR_URL}/${ref.toon}/chapter/${ref.number}`;
		if (ref.kind === 'characters' || ref.kind === 'moods' || ref.kind === 'asset-requests') return `${CREATOR_URL}/characters`;
		return ref.toon ? `${CREATOR_URL}/${ref.toon}` : CREATOR_URL;
	}

	const setParam = (key: string, value: string) => {
		const url = new URL(page.url);
		if (value) url.searchParams.set(key, value);
		else url.searchParams.delete(key);
		goto(url.pathname + url.search, { keepFocus: true, noScroll: true, replaceState: true });
	};
</script>

<PageHeader type={m['nav.group_data']()} title={m['content.title']()} meta={m['content.meta']()} />

<ActionBar label={m['shell.page_actions']()} moreLabel={m['shell.more']()} {...infoBar()} infoTitle={m['content.title']()}>
	{#snippet info()}<PageHelp page="content" />{/snippet}
	{#snippet start()}
		{#if data}
			<div class="toon-chips" role="group" aria-label={m['events.col_toon']()}>
				<button type="button" class="toon-chip" aria-pressed={!toonFilter} onclick={() => setParam('toon', '')}>{m['common.all']()}</button>
				{#each data.toons as t (t.id)}
					<button type="button" class="toon-chip" aria-pressed={toonFilter === t.id} onclick={() => setParam('toon', t.id)}>
						<ToonAvatar src={toonAvatarUrl(t.id)} name={t.name} size="1.5rem" />
						<span>{t.name}</span>
					</button>
				{/each}
			</div>
			<label class="bar-filter">
				<span>{m['content.state']()}</span>
				<select value={stateFilter} onchange={(e) => setParam('state', e.currentTarget.value)}>
					<option value="">{m['common.all']()}</option>
					<option value="published">{m['content.published']()}</option>
					<option value="ahead">{m['content.ahead']()}</option>
					<option value="draft">{m['content.draft_only']()}</option>
				</select>
			</label>
		{/if}
	{/snippet}
	{#snippet end()}
		<Button size="sm" href={CREATOR_URL} target="_blank" rel="noreferrer">{m['content.open_creator']()}</Button>
	{/snippet}
</ActionBar>

{#if error}
	<p class="error-box" role="alert">{error}</p>
{:else if !data}
	<Skeleton variant="table" label={m['shell.loading']()} />
{:else}

	{#each groups as group (group.toon.id ?? '')}
		<Section title={group.toon.name} hint={m['content.files']({ count: group.rows.length })}>
			{#snippet media()}{#if group.toon.id}<ToonAvatar src={toonAvatarUrl(group.toon.id)} name={group.toon.name} />{/if}{/snippet}
			{#snippet actions()}
				<Button size="sm" variant="secondary" href="{CREATOR_URL}/{group.toon.id ?? ''}" target="_blank" rel="noreferrer">{m['content.open_in_creator']()}</Button>
			{/snippet}
			{#if group.rows.length}
				<div class="table-scroll">
					<table>
						<thead>
							<tr>
								<th scope="col">{m['content.path']()}</th>
								{#each group.locales as locale (locale)}<th scope="col">{locale}</th>{/each}
								<th scope="col"></th>
							</tr>
						</thead>
						<tbody>
							{#each group.rows as row (row.entity)}
								<tr>
									<th scope="row">{fileLabel(row.entity)}</th>
									{#each group.locales as locale (locale)}
										{@const f = row.byLocale[locale]}
										<td>
											{#if f}
												{@const state = fileState(f)}
												{#if state === 'published'}<Badge tone="ink">{m['content.published']()}</Badge>
												{:else if state === 'ahead'}<Badge tone="warning">{m['content.ahead']()}</Badge>
												{:else}<Badge>{m['content.draft_only']()}</Badge>{/if}
												<span class="meta">
													{m['content.version_line']({ latest: f.latest, published: f.published ?? '—', versions: f.versions })}
													<br />{minute(f.latest_at)}
												</span>
											{:else}<span class="muted">—</span>{/if}
										</td>
									{/each}
									<td><a href={creatorLink(row.entity)} target="_blank" rel="noreferrer">{m['content.edit']()}</a></td>
								</tr>
							{/each}
						</tbody>
					</table>
				</div>
			{:else}
				<p class="muted">{m['content.empty']()}</p>
			{/if}
		</Section>
	{/each}
{/if}

<style>
	/* toon selector: nav chips (DESIGN.md: 2px border-muted → ink on hover / active) */
	.toon-chips {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	.toon-chip {
		display: inline-flex;
		align-items: center;
		gap: var(--space-05);
		min-height: 2.25rem;
		padding: var(--space-025) var(--space-075) var(--space-025) var(--space-025);
		border: var(--border-width-regular) solid var(--border-muted);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
		white-space: nowrap;
		cursor: pointer;
	}
	.toon-chip:first-child {
		padding-left: var(--space-075);
	}
	.toon-chip:hover,
	.toon-chip[aria-pressed='true'] {
		border-color: var(--ink);
	}
	.table-scroll {
		overflow-x: auto;
		border: var(--border-width-regular) solid var(--ink);
	}
	table {
		width: 100%;
		border-collapse: collapse;
	}
	th,
	td {
		padding: var(--space-05) var(--space-075);
		text-align: left;
		vertical-align: top;
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	thead th {
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	tbody th {
		font-weight: var(--font-weight-regular);
		white-space: nowrap;
	}
	.meta {
		display: block;
		margin-top: var(--space-025);
		color: var(--text-muted);
		font-size: var(--type-caption-size);
	}
</style>
