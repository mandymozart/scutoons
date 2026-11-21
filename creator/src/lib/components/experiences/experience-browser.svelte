<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { ActionBar, Badge, GroupBar, region, type GroupBarItem } from '@scutoons/ui';
	import type { XrExperience } from '@scutoons/content';
	import { experiencesApi, getDraft, listVersions, publishVersion, type ContentVersion, type WrappedContent } from '$lib/api.ts';
	import { experienceHref, experienceKey, experienceRows, titleIn, type ExperienceRow, type ExperienceState } from '$lib/editor/experiences.ts';
	import { publishOutcome, type PublishOutcome } from '$lib/editor/scenes.ts';
	import { getCreatorNav, usePageNav } from '$lib/nav.svelte.ts';
	import { toast } from '$lib/toasts.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import ExperienceDetail from '$lib/components/experiences/experience-detail.svelte';

	/**
	 * The Experiences workspace, first slice: the list in the Tree panel, the open experience's facts, contents and
	 * versions, and Publish of its newest draft. No stage and no editing: experiences come from OSCT through the MCP.
	 */
	let { id }: { id: string | null } = $props();

	const nav = getCreatorNav();
	const locale = getLocale();
	let rows = $state<ExperienceRow[]>([]);
	let loaded = $state(false);
	let loadError = $state<string | null>(null);
	const row = $derived(rows.find((r) => r.id === id) ?? null);

	const TYPES: Record<string, () => string> = {
		book: () => m['creator.experiences_type_book'](),
		record: () => m['creator.experiences_type_record'](),
		poster: () => m['creator.experiences_type_poster'](),
		exhibition: () => m['creator.experiences_type_exhibition'](),
		'concert-reading': () => m['creator.experiences_type_concert_reading']()
	};
	const typeName = (type: string) => TYPES[type]?.() ?? type;
	const toonNames = (toons: string[]) => toons.map((t) => nav.toonName(t)).join(' · ');

	const message = (e: unknown) => (e instanceof Error ? e.message : String(e));
	async function reload() {
		try {
			rows = experienceRows((await experiencesApi.list()).entries, locale);
			loadError = null;
		} catch (e) {
			loadError = message(e);
		} finally {
			loaded = true;
		}
	}
	onMount(() => void reload());

	let draft = $state.raw<WrappedContent<XrExperience> | null>(null);
	let versions = $state<ContentVersion[]>([]);
	let outcome = $state<PublishOutcome | null>(null);
	let busy = $state(false);
	async function load(experienceId: string) {
		const key = experienceKey(experienceId);
		try {
			const [d, v] = await Promise.all([getDraft<XrExperience>(key), listVersions(key)]);
			if (id !== experienceId) return;
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
	// no experience in the URL: the first one opens
	$effect(() => {
		const first = rows[0]?.id;
		if (!id && first) void goto(experienceHref(first), { replaceState: true, keepFocus: true, noScroll: true });
	});

	const canPublish = $derived(!!draft && !busy && !!row && row.state !== 'published');
	async function publish() {
		if (!id || !draft || !canPublish) return;
		const experienceId = id;
		busy = true;
		try {
			await publishVersion(experienceKey(experienceId), draft.version.id);
			outcome = publishOutcome(null);
			toast(m['creator.experiences_publish_ok'](), { type: 'success' });
		} catch (e) {
			outcome = publishOutcome(e);
		} finally {
			busy = false;
		}
		await Promise.all([reload(), load(experienceId)]);
	}

	const items = $derived<GroupBarItem[]>([
		{ id: 'publish', icon: 'upload-cloud', label: m['creator.publish'](), primary: canPublish, disabled: !canPublish, run: () => void publish(), testid: 'experience-publish' }
	]);

	usePageNav({ sidebar: list, extra: () => (row ? [{ href: experienceHref(row.id), label: titleIn(row.title, locale, row.id) }] : []) });
</script>

{#snippet badge(state: ExperienceState)}
	{#if state === 'published'}<Badge tone="published">{m['creator.experiences_published']()}</Badge>{:else if state === 'draft'}<Badge tone="draft">{m['creator.experiences_draft']()}</Badge>{:else}<span title={m['creator.experiences_unpublished']()}><Badge tone="soon">{m['creator.experiences_draft']()}</Badge></span>{/if}
{/snippet}

{#snippet list()}
	<div class="xr-list" data-testid="experience-list">
		<ul use:region={'experiences'}>
			{#each rows as r (r.id)}
				<li>
					<a class="item" href={experienceHref(r.id)} class:sc-selected={r.id === id} aria-current={r.id === id ? 'page' : undefined} data-nav-child data-drawer-pick data-control-exempt="two-line list row" data-testid="experience-row-{r.id}">
						<span class="name">{titleIn(r.title, locale, r.id)}</span>
						<span class="marks">{@render badge(r.state)}</span>
						<span class="hint">{[typeName(r.type), toonNames(r.toons)].filter(Boolean).join(' · ')}</span>
					</a>
				</li>
			{/each}
		</ul>
		{#if loaded && !rows.length && !loadError}<p class="sc-empty pad">{m['creator.experiences_empty']()}</p>{/if}
	</div>
{/snippet}

<div class="creator-page">
	<main class="xr-page" data-testid="experience-page">
		{#if loadError}<p class="error" role="alert">{loadError}</p>{/if}
		{#if row && draft}
			<ExperienceDetail {row} content={draft.content} {versions} published={draft.published?.id ?? null} {outcome} typeName={typeName(row.type)} toons={toonNames(row.toons)}>
				{#snippet status()}{@render badge(row.state)}{/snippet}
			</ExperienceDetail>
		{:else if loaded && rows.length && !id}
			<p class="sc-empty">{m['creator.experiences_pick']()}</p>
		{:else if id && !loadError}
			<p class="sc-empty">{m['creator.loading']()}</p>
		{/if}
	</main>
	<ActionBar label={m['creator.page_actions']()} moreLabel={m['creator.more']()}>
		{#snippet groups()}
			<GroupBar {items} label={m['creator.bar_actions']()} backLabel={m['creator.bar_back']()} />
		{/snippet}
	</ActionBar>
</div>

<style>
	.xr-page {
		padding: 1.5rem 1rem;
		max-width: 60rem;
	}
	.xr-list {
		display: flex;
		flex-direction: column;
	}
	ul {
		list-style: none;
		margin: 0;
		padding: 0;
	}
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
	.name,
	.hint {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.hint {
		grid-column: 1 / -1;
		color: var(--text-muted);
		font-size: var(--type-caption-size);
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
