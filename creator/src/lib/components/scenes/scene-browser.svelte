<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import { Badge, confirmDialog } from '@scutoons/ui';
	import type { ContentVersion, SceneDocument, WrappedContent } from '@scutoons/api';
	import { scenesApi } from '$lib/api.ts';
	import type { PublishRefusal } from '$lib/editor/publish-report.ts';
	import { publishOutcome, sceneHref, sceneRows, sceneTitle, type PublishOutcome, type SceneRow } from '$lib/editor/scenes.ts';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import { usePageDoc } from '$lib/shell/shell.svelte.ts';
	import { toast } from '$lib/toasts.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SceneDetail from '$lib/components/scenes/scene-detail.svelte';

	/**
	 * The creator's Scenes area (#490 – the first slice of draft 3, research/SCENES.md §9; owner 2026-11-12 [scenes]):
	 * the open scene's facts and its versions – opened from the Library, no list of its own (not twice) –, and the one
	 * Publish (publish-tree.ts: the scene with the drafts it plays, the core's check last; its problems listed here). No editing
	 * here: the member's AI saves drafts through the MCP, the scene editor comes later.
	 */
	let { id }: { id: string | null } = $props();

	let rows = $state<SceneRow[]>([]);
	let loaded = $state(false);
	let loadError = $state<string | null>(null);
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
		const first = rows[0]?.id;
		if (!id && first) void goto(sceneHref(first), { replaceState: true, keepFocus: true, noScroll: true });
	});

	/** after the bar's one Publish (RULES H18): the open scene's outcome (the core's check) at its page, the lists read again */
	async function published(result: { published: string[]; refused: PublishRefusal[] }) {
		if (!id) return;
		const sceneId = id;
		const next = publishOutcome(`scene:${sceneId}`, result);
		if (next) outcome = next;
		if (next || result.published.some((k) => k.startsWith('scene:'))) await Promise.all([reload(), load(sceneId)]);
	}

	async function restore(v: ContentVersion) {
		if (!id) return;
		const sceneId = id;
		const ok = await confirmDialog({ title: m['creator.scenes_restore_title'](), message: m['creator.scenes_restore_message'](), confirmLabel: m['creator.restore']() });
		if (!ok) return;
		try {
			await scenesApi.restore(sceneId, v.id);
			outcome = null;
			toast(m['creator.scenes_restored'](), { type: 'success', kind: 'history' });
		} catch (e) {
			toast(message(e), { type: 'error' });
		}
		await Promise.all([reload(), load(sceneId)]);
	}

	usePageDoc(() => ({ published, unpublished: !!row && row.state !== 'published' }));
	usePageNav({ extra: () => (row ? [{ href: sceneHref(row.id), label: sceneTitle(row.facts) }] : []) });
</script>

{#snippet badge(state: SceneRow['state'])}
	{#if state === 'published'}<Badge tone="published">{m['creator.scenes_published']()}</Badge>{:else if state === 'draft'}<Badge tone="draft">{m['creator.scenes_draft']()}</Badge>{:else}<span title={m['creator.scenes_unpublished']()}><Badge tone="soon">{m['creator.scenes_draft']()}</Badge></span>{/if}
{/snippet}

<div class="creator-page">
	<main class="scene-page" data-testid="scene-page">
		{#if loadError}<p class="error" role="alert">{loadError}</p>{/if}
		{#if row && draft}
			<SceneDetail {row} {versions} published={draft.published?.id ?? null} {outcome} {restore}>
				{#snippet status()}{@render badge(row.state)}{/snippet}
			</SceneDetail>
		{:else if loaded && !rows.length}
			<p class="sc-empty">{m['creator.scenes_empty']()}</p>
		{:else if id}
			<p class="sc-empty">{m['creator.loading']()}</p>
		{/if}
	</main>
</div>

<style>
	.scene-page {
		padding: 1.5rem 1rem;
		max-width: 60rem;
	}
	.error {
		color: var(--tool-stop);
	}
</style>
