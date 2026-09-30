<script lang="ts">
	import { goto } from '$app/navigation';
	import { page } from '$app/state';
	import { toonLanguages, type Chapter, type ToonLanguages } from '@scutoons/content';
	import { Slice } from '@scutoons/reader';
	import { ActionBar, Badge, Tabs } from '@scutoons/ui';
	import { library } from '$lib/api.ts';
	import { loadToonLanguages } from '$lib/editor/languages.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { infoBar } from '$lib/info.ts';
	import PageHelp from '$lib/components/page-help.svelte';
	import { usePageNav } from '$lib/nav.svelte.ts';

	/**
	 * Draft preview: the newest saved version of the chapter with its scenes in one locale (the library in draft mode,
	 * the same composition the reader gets), rendered with the reader's Slice / text boxes.
	 * Visual novel events are not played; horizontal groups are shown one slice below the other.
	 */
	const toon = $derived(page.params.toonId as string);
	const chapterNumber = $derived(Number(page.params.chapterNumber));
	/** the toon's languages (newest draft of its index entry); `?locale=` must be one of them, else the source */
	let languages = $state<ToonLanguages | null>(null);
	$effect(() => {
		loadToonLanguages(toon)
			.then((l) => (languages = l))
			.catch(() => (languages = toonLanguages(null)));
	});
	const locale = $derived.by(() => {
		if (!languages) return null;
		const wanted = page.url.searchParams.get('locale') ?? '';
		return (languages.languages as string[]).includes(wanted) ? wanted : languages.source;
	});
	const label = $derived(page.url.searchParams.get('label'));

	let chapter = $state<Chapter | null>(null);
	let fromDraft = $state(false);
	let error = $state<string | null>(null);
	let loading = $state(true);

	async function load(toonId: string, number: number, lang: string) {
		loading = true;
		error = null;
		try {
			const data = await library.fetchChapter(toonId, number, lang, { draft: true }).catch(() => {
				throw new Error(m['creator.preview_not_found']({ number, locale: lang }));
			});
			const draft = true;
			chapter = data;
			fromDraft = draft;
		} catch (e) {
			chapter = null;
			error = e instanceof Error ? e.message : String(e);
		} finally {
			loading = false;
		}
	}

	$effect(() => {
		if (locale) void load(toon, chapterNumber, locale);
	});

	const groups = $derived(chapter?.groups ?? []);
	const shown = $derived(label ? groups.filter((g) => g.label === label) : groups);

	function setParam(key: string, value: string | null) {
		const url = new URL(page.url);
		if (value) url.searchParams.set(key, value);
		else url.searchParams.delete(key);
		goto(url.pathname + url.search, { replaceState: true, keepFocus: true, noScroll: true });
	}

	// the chapter's children in the sidebar: its groups (a click shows only that group)
	usePageNav({ sidebar: groupNav });
</script>

{#snippet groupNav()}
	<p class="nav-head">{m['creator.groups']()}</p>
	<ul class="nav-list">
		<li><button type="button" class:current={!label} onclick={() => setParam('label', null)}>{m['creator.preview_title']()}</button></li>
		{#each groups as group, index (index)}
			{#if group.label}
				<li><button type="button" class:current={label === group.label} onclick={() => setParam('label', group.label ?? null)} data-nav-child>{group.label}</button></li>
			{/if}
		{/each}
	</ul>
{/snippet}

<svelte:head>
	<title>{m['creator.preview_title']()} · {toon} {chapterNumber} ({locale ?? '…'})</title>
</svelte:head>

<div class="preview-page" data-testid="draft-preview">
	<p class="meta">
		{#if chapter}
			<Badge tone={fromDraft ? 'accent' : 'neutral'}>
				{fromDraft ? m['creator.preview_source_draft']() : m['creator.preview_source_published']()}
			</Badge>
		{/if}
	</p>

	{#if loading && !chapter}
		<p class="status">{m['creator.loading']()}</p>
	{:else if error}
		<p class="status error">{error}</p>
	{:else if chapter}
		<main class="column" lang={locale ?? undefined}>
			<h1>{chapter.title}</h1>
			{#each shown as group, g (g)}
				{#if group.label}<h2 class="group-label">{group.label}</h2>{/if}
				{#each group.slices ?? [] as slice, s (s)}
					<Slice
							{slice}
							src={asset(`${toon}/${slice.image}`)}
							videoSrc={slice.video?.src ? asset(`${toon}/${slice.video.src}`) : undefined}
							eager={g === 0 && s < 2}
							lang={locale ?? undefined}
						/>
				{/each}
			{/each}
		</main>
	{/if}

	<!-- Bottom action bar: locale + group filter · back to the editor -->
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()} {...infoBar()} infoTitle={m['creator.preview_title']()}>
		{#snippet info()}<PageHelp page="preview" />{/snippet}
		{#snippet start()}
			<Tabs
				label={m['creator.preview_locale']()}
				tabs={(languages?.languages ?? []).map((l) => ({ id: l, label: l }))}
				value={locale ?? ''}
				onchange={(id) => setParam('locale', id)}
			/>
			<label class="group-select">
				<span>{m['creator.preview_group']()}</span>
				<select value={label ?? ''} onchange={(e) => setParam('label', e.currentTarget.value || null)}>
					<option value="">–</option>
					{#each groups as group, index (index)}
						{#if group.label}<option value={group.label}>{group.label}</option>{/if}
					{/each}
				</select>
			</label>
		{/snippet}
		{#snippet end()}
			<a class="btn" href="/{toon}/chapter/{chapterNumber}">{m['creator.preview_back']()}</a>
		{/snippet}
	</ActionBar>
</div>

<style>
	.preview-page {
		min-height: 100dvh;
		background: var(--surface-panel);
	}
	.preview-page {
		--action-bar-height: 64px;
	}
	.preview-page :global(.sc-action-bar .list) {
		margin: 0;
	}
	.group-select {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}
	.group-select select {
		width: auto;
	}
	.meta {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: 0.5rem;
		max-width: 500px;
		margin: 0.75rem auto;
		padding: 0 1rem;
		color: var(--muted);
	}
	.status {
		max-width: 500px;
		margin: 2rem auto;
		padding: 0 1rem;
	}
	.error {
		color: var(--ink);
		border-left: 2px solid var(--error);
		padding-left: 0.5rem;
	}
	/* the reader's chapter column */
	.column {
		max-width: 500px;
		margin: 0 auto 4rem;
		background: var(--paper);
	}
	h1 {
		margin: 0;
		padding: 1rem;
		font-size: 1rem;
	}
	.group-label {
		margin: 0;
		padding: 0.4rem 1rem;
		font-size: 11px;
		font-weight: 400;
		color: var(--muted);
		background: var(--surface-panel);
		border-top: 1px solid var(--border-subtle);
	}
	.nav-head {
		margin: 0;
		padding: 0.75rem 0.75rem 0.25rem;
		font-size: 10px;
		text-transform: uppercase;
		color: var(--muted);
	}
	.nav-list {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	.nav-list button {
		display: block;
		width: 100%;
		text-align: left;
		padding: 0.35rem 0.75rem;
		background: transparent;
		color: var(--ink);
		border: 0;
		border-left: 3px solid transparent;
	}
	.nav-list button:hover {
		background: var(--surface-hover);
	}
	.nav-list button.current {
		border-left-color: var(--ink);
		font-weight: 700;
		background: var(--surface-hover);
	}
</style>
