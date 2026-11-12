<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { ActionBar, Badge, PhoneBar, confirmDialog, region, type PhoneBarItem } from '@scutoons/ui';
	import type { ContentVersion, SceneDocument, WrappedContent } from '@scutoons/api';
	import { scenesApi } from '$lib/api.ts';
	import { NO_TOON, filterRows, publishOutcome, rowHint, sceneHref, sceneRows, sceneTitle, sceneToons, type PublishOutcome, type SceneFilter, type SceneRow } from '$lib/editor/scenes.ts';
	import { getCreatorNav, usePageNav } from '$lib/nav.svelte.ts';
	import { toast } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SceneDetail from '$lib/components/scenes/scene-detail.svelte';

	/**
	 * The creator's Scenes area (#490 – the first slice of draft 3, research/SCENES.md §9; owner 2026-11-12 [scenes]):
	 * the list beside the menu (one search, the toon incl. "no toon", Draft / Published), the open scene's facts and its
	 * versions, and Publish – the server asks the core's check first (422 its problems, listed here; 503 the check is
	 * not reachable). No editing here: the member's AI saves drafts through the MCP, the scene editor comes later.
	 */
	let { id }: { id: string | null } = $props();

	const nav = getCreatorNav();
	let rows = $state<SceneRow[]>([]);
	let loaded = $state(false);
	let loadError = $state<string | null>(null);
	let filter = $state<SceneFilter>({ query: '', toon: '', state: 'all' });
	const visible = $derived(filterRows(rows, filter));
	const toons = $derived(sceneToons(rows));
	const row = $derived(rows.find((r) => r.id === id) ?? null);

	const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
	async function reload() {
		try {
			rows = sceneRows((await scenesApi.list()).scenes);
			loadError = null;
		} catch (e) {
			loadError = message(e);
		} finally {
			loaded = true;
		}
	}
	onMount(() => void reload());

	// the open scene: its newest version (draft or published) and its history
	let draft = $state<WrappedContent<SceneDocument> | null>(null);
	let versions = $state<ContentVersion[]>([]);
	let outcome = $state<PublishOutcome | null>(null);
	let busy = $state(false);
	async function load(sceneId: string) {
		try {
			const [d, v] = await Promise.all([scenesApi.draft(sceneId), scenesApi.versions(sceneId, 50)]);
			if (id !== sceneId) return;
			draft = d;
			versions = v.versions;
		} catch (e) {
			loadError = message(e);
		}
	}
	$effect(() => {
		const current = id;
		untrack(() => {
			outcome = null;
			draft = null;
			versions = [];
			if (current) void load(current);
		});
	});
	// no scene in the URL: the first one opens (like the catalog's first entry)
	$effect(() => {
		const first = visible[0]?.id;
		if (!id && first) void goto(sceneHref(first), { replaceState: true, keepFocus: true, noScroll: true });
	});

	const canPublish = $derived(!!draft && !busy && !!row && row.state !== 'published');
	async function publish() {
		if (!id || !draft || !canPublish) return;
		const sceneId = id;
		busy = true;
		try {
			await scenesApi.publish(sceneId, draft.version.id);
			outcome = publishOutcome(null);
			toast(m['creator.scenes_publish_ok'](), { type: 'success' });
		} catch (e) {
			outcome = publishOutcome(e);
		} finally {
			busy = false;
		}
		await Promise.all([reload(), load(sceneId)]);
	}

	async function restore(v: ContentVersion) {
		if (!id) return;
		const sceneId = id;
		const ok = await confirmDialog({ title: m['creator.scenes_restore_title'](), message: m['creator.scenes_restore_message'](), confirmLabel: m['creator.restore']() });
		if (!ok) return;
		try {
			await scenesApi.restore(sceneId, v.id);
			outcome = null;
			toast(m['creator.scenes_restored'](), { type: 'success' });
		} catch (e) {
			toast(message(e), { type: 'error' });
		}
		await Promise.all([reload(), load(sceneId)]);
	}

	const items = $derived<PhoneBarItem[]>([
		{ id: 'publish', icon: 'upload-cloud', label: m['creator.publish'](), primary: canPublish, disabled: !canPublish, run: () => void publish(), testid: 'scene-publish' }
	]);
	const states = $derived([
		{ value: 'all' as const, label: m['creator.scenes_all']() },
		{ value: 'draft' as const, label: m['creator.scenes_draft']() },
		{ value: 'published' as const, label: m['creator.scenes_published']() }
	]);

	usePageNav({ sidebar: list, extra: () => (row ? [{ href: sceneHref(row.id), label: sceneTitle(row.facts) }] : []) });
</script>

{#snippet badge(state: SceneRow['state'])}
	{#if state === 'published'}<Badge tone="published">{m['creator.scenes_published']()}</Badge>{:else if state === 'draft'}<Badge tone="draft">{m['creator.scenes_draft']()}</Badge>{:else}<span title={m['creator.scenes_unpublished']()}><Badge tone="soon">{m['creator.scenes_draft']()}</Badge></span>{/if}
{/snippet}

{#snippet list()}
	<div class="scene-list" data-testid="scene-list">
		<div class="filters">
			<input type="search" bind:value={filter.query} placeholder={m['creator.scenes_search']()} aria-label={m['creator.scenes_search']()} data-testid="scene-search" />
			<select bind:value={filter.toon} aria-label={m['creator.scenes_filter_toon']()} data-testid="scene-filter-toon">
				<option value="">{m['creator.scenes_all_toons']()}</option>
				{#each toons.toons as t (t)}<option value={t}>{nav.toonName(t)}</option>{/each}
				{#if toons.noToon}<option value={NO_TOON}>{m['creator.scenes_no_toon']()}</option>{/if}
			</select>
			<select bind:value={filter.state} aria-label={m['creator.scenes_filter_state']()} data-testid="scene-filter-state">
				{#each states as s (s.value)}<option value={s.value}>{s.label}</option>{/each}
			</select>
		</div>
		<!-- a selection region (DESIGN.md "Selection"): the open scene is sc-selected -->
		<ul use:region={'scenes'}>
			{#each visible as r (r.id)}
				<li>
					<a class="item" href={sceneHref(r.id)} class:sc-selected={r.id === id} aria-current={r.id === id ? 'page' : undefined} data-nav-child data-drawer-pick data-control-exempt="two-line list row" data-testid="scene-row-{r.id}">
						<span class="name">{sceneTitle(r.facts)}</span>
						<span class="marks">{@render badge(r.state)}</span>
						<span class="hint">{rowHint(r.facts)}</span>
					</a>
				</li>
			{/each}
		</ul>
		{#if loaded && !rows.length}
			<p class="sc-empty pad">{m['creator.scenes_empty']()}</p>
		{:else if loaded && !visible.length}
			<p class="sc-empty pad">{m['creator.scenes_none_match']()}</p>
		{/if}
	</div>
{/snippet}

<div class="creator-page">
	<main class="scene-page" data-testid="scene-page">
		{#if loadError}<p class="error" role="alert">{loadError}</p>{/if}
		{#if row && draft}
			<SceneDetail {row} {versions} published={draft.published?.id ?? null} {outcome} {restore}>
				{#snippet status()}{@render badge(row.state)}{/snippet}
			</SceneDetail>
		{:else if loaded && rows.length && !id}
			<p class="sc-empty">{m['creator.scenes_pick']()}</p>
		{:else if id}
			<p class="sc-empty">{m['creator.loading']()}</p>
		{/if}
	</main>
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
		{#snippet phone()}
			<PhoneBar {items} label={m['creator.bar_actions']()} backLabel={m['creator.bar_back']()} />
		{/snippet}
	</ActionBar>
</div>

<style>
	.scene-page {
		padding: 1.5rem 1rem;
		max-width: 60rem;
	}
	.scene-list {
		display: flex;
		flex-direction: column;
	}
	.filters {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		padding: 0 1rem 0.75rem;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	/* two lines: the name and its state, then where · spot · who */
	.item {
		display: grid;
		grid-template-columns: minmax(0, 1fr) auto;
		align-items: center;
		gap: 0.1rem 0.5rem;
		padding: 0.4rem 1rem;
		color: var(--ink);
		text-decoration: none;
		border-top: var(--border-width-regular) solid var(--border-subtle);
	}
	.item:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.name {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.hint {
		grid-column: 1 / -1;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.marks {
		justify-self: end;
	}
	.pad {
		padding: 0.5rem 1rem;
	}
	.error {
		color: var(--state-error);
	}
</style>
