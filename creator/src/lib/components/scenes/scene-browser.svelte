<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { goto } from '$app/navigation';
	import type { ContentVersion, SceneDocument, WrappedContent } from '@scutoons/api';
	import { Thumb } from '@scutoons/ui/editor';
	import { scenesApi } from '$lib/api.ts';
	import type { PublishRefusal } from '$lib/editor/publish-report.ts';
	import { publishOutcome, sceneHref, sceneRows, sceneTitle, type PublishOutcome, type SceneRow } from '$lib/editor/scenes.ts';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import { iconOf } from '$lib/shell/address.ts';
	import { usePageDoc, useTabs } from '$lib/shell/shell.svelte.ts';
	import { thumbs } from '$lib/shell/thumbs.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import SceneDetail from '$lib/components/scenes/scene-detail.svelte';
	import OpenInDeck from '$lib/components/shell/open-in-deck.svelte';

	/**
	 * The creator's Scenes area (lap 2 C5): the open scene previewed – its still (`scene:<id>`, #842) the whole back, or
	 * the kind's icon with "No still yet" as hover text –, "Edit in deck" on it, and its facts and versions read only in
	 * the Scene panel over it. Opened from the Library, no list of its own (not twice). The scene is edited in the deck;
	 * here only the bar's one Publish (publish-tree.ts: the scene with the drafts it plays, the core's check last; its
	 * problems shown in the panel).
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

	const key = $derived(id ? `scene:${id}` : null);
	$effect(() => {
		if (key) thumbs.want([key]);
	});
	const still = $derived(key ? (thumbs.get(key)?.url ?? null) : null);

	/** after the bar's one Publish (RULES H18): the open scene's outcome (the core's check) in its panel, the lists read again */
	async function published(result: { published: string[]; refused: PublishRefusal[] }) {
		if (!id) return;
		const sceneId = id;
		const next = publishOutcome(`scene:${sceneId}`, result);
		if (next) outcome = next;
		if (next || result.published.some((k) => k.startsWith('scene:'))) await Promise.all([reload(), load(sceneId)]);
	}

	usePageDoc(() => ({ published, unpublished: !!row && row.state !== 'published' }));
	usePageNav({ fill: true, extra: () => (row ? [{ href: sceneHref(row.id), label: sceneTitle(row.facts) }] : []) });
	useTabs(() => ({ facts: row && draft ? factsTab : null }));
</script>

{#snippet factsTab()}
	{#if row && draft}<SceneDetail {row} {versions} published={draft.published?.id ?? null} {outcome} />{/if}
{/snippet}

<main class="scene-page" data-testid="scene-page">
	{#if key}
		<div class="back" title={still ? undefined : m['library.no_still']()} data-testid="scene-preview" data-still={still ? '' : undefined}>
			<Thumb src={still} icon={iconOf(key)} size="3rem" fill empty={m['library.no_still']()} />
		</div>
		<div class="on-back"><OpenInDeck address={key} edit variant="framed" testid="scene-open-deck" /></div>
	{/if}
	{#if loadError}<p class="note error" role="alert">{loadError}</p>
	{:else if loaded && !rows.length}<p class="note sc-empty">{m['creator.scenes_empty']()}</p>{/if}
</main>

<style>
	/* the still is the whole back: under the bar, the panels and the status bar */
	.scene-page {
		position: relative;
		flex: 1;
		min-height: 0;
	}
	/* the stage keeps the status bar's room as padding: the back runs into it */
	.back {
		position: absolute;
		inset: 0 0 calc(-1 * var(--status-room));
	}
	.on-back {
		position: absolute;
		z-index: 1;
		top: calc(var(--shell-top) + var(--editor-gap));
		left: 50%;
		transform: translateX(-50%);
	}
	.note {
		position: absolute;
		z-index: 1;
		top: 50%;
		left: 50%;
		margin: 0;
		transform: translate(-50%, -50%);
	}
	.error {
		color: var(--tool-stop);
	}
</style>
