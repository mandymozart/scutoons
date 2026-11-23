<script lang="ts">
	import { untrack } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { NEUTRAL_LOCALE } from '@scutoons/content';
	import { publishPartial } from '$lib/api.ts';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { DocSession, useAutosave } from '$lib/editor/doc-session.svelte.ts';
	import { experienceFacts, experienceHref, experienceKey, experienceOutline, pluginRows, titleIn } from '$lib/editor/experiences.ts';
	import { entityOf, entitySlot, experienceFiles, nodeDocPath, targetDocPath, targetPlace, targetsSharing, type NodeRef, type PartAction, type XrFile } from '$lib/editor/xr-nodes.ts';
	import { projectXr } from '@scutoons/xr-engine/network';
	import { sporeScope } from '$lib/spores/scope.svelte.ts';
	import {
		xrAddNode,
		xrAddSurface,
		xrDefaultAddPlace,
		xrDuplicateNode,
		xrGroupNode,
		xrIndentNode,
		xrNewNode,
		xrNodeAt,
		xrOutdentNode,
		xrRemoveNode,
		xrShiftNode,
		type XrNode,
		type XrTreeAdd
	} from '@scutoons/xr-engine/tree';
	import { neutralDoc } from '$lib/editor/languages.ts';
	import { publishOutcome } from '$lib/editor/scenes.ts';
	import { STAGE_LOADING, type StagePlacement, type StageTool, type StageView } from '$lib/editor/xr-stage.ts';
	import { getCreatorNav, usePageNav } from '$lib/nav.svelte.ts';
	import { shell, usePageDoc, useTabs } from '$lib/shell/shell.svelte.ts';
	import { toast } from '$lib/toasts.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import ExperienceMeta from '$lib/components/experiences/experience-meta.svelte';
	import ExperienceInspector from '$lib/components/experiences/experience-inspector.svelte';
	import ExperienceTree from '$lib/components/experiences/experience-tree.svelte';
	import ExperienceScenes from '$lib/components/experiences/experience-scenes.svelte';
	import ExperienceStage from '$lib/components/experiences/experience-stage.svelte';

	/**
	 * The XR editor's workspace (#482): the stage is the experience and fills the whole workspace; the panels lie over it –
	 * the outline in the Tree, the meta in the Experience tab, the scenes strip with the stage's view in the Scenes tab;
	 * Publish and History are the bar's, as everywhere. Experiences are opened from the Library. Parts and surfaces are added
	 * with the Tree's footer, moved, duplicated, grouped and removed in the Tree, placed with the gizmo.
	 */
	let { id }: { id: string | null } = $props();

	const nav = getCreatorNav();
	const locale = getLocale();
	let doc = $state.raw<ContentDoc | null>(null);
	let picked = $state('');
	let view = $state<StageView>('reader');
	let tool = $state<StageTool>('select');
	let run = $state(0);
	let shown = $state(STAGE_LOADING);
	let problems = $state<string[]>([]);
	let busy = $state(false);
	/** the one selection: the Tree, the stage and the Inspector share it */
	let selected = $state.raw<NodeRef | null>(null);
	/** the last edit's trees for the stage (each target whose entity changed) */
	let edited = $state.raw<{ at: number; trees: Record<string, object | null> } | null>(null);
	/** the last moved target's place for the stage */
	let moved = $state.raw<{ at: number; target: string; place: NonNullable<ReturnType<typeof targetPlace>> } | null>(null);

	$effect(() => {
		const current = id;
		untrack(() => {
			problems = [];
			picked = '';
			selected = null;
			edited = null;
			moved = null;
			doc = current ? neutralDoc(experienceKey(current)) : null;
			void doc?.load();
		});
	});

	const content = $derived(doc && !doc.loading ? (doc.data[NEUTRAL_LOCALE] ?? null) : null);
	const facts = $derived(content && id ? experienceFacts(content, id) : null);
	const scenes = $derived(experienceOutline((content as { configuration?: unknown } | null)?.configuration).scenes);
	const scene = $derived(scenes.find((s) => s.id === picked)?.id ?? scenes[0]?.id ?? '');
	const configuration = $derived((content as { configuration?: unknown } | null)?.configuration);
	const plugins = $derived(pluginRows((content as { plugins?: unknown } | null)?.plugins));
	const base = $derived(doc?.base[NEUTRAL_LOCALE] ?? null);
	const unpublished = $derived(!!doc && base !== null && doc.published[NEUTRAL_LOCALE] !== base);

	const TYPES: Record<string, () => string> = {
		book: () => m['creator.experiences_type_book'](),
		record: () => m['creator.experiences_type_record'](),
		poster: () => m['creator.experiences_type_poster'](),
		exhibition: () => m['creator.experiences_type_exhibition'](),
		'concert-reading': () => m['creator.experiences_type_concert_reading']()
	};

	/** the creator's one autosave (#760): the document is saved 1.2 s after the last edit, one save at a time, a Log line each */
	const session = new DocSession({ docs: () => (doc ? [doc] : []) });
	useAutosave(session);

	/** the creator's one way to publish: the server's partial publish (its refusals named with their reason); a pending save first */
	async function publish() {
		if (!doc || busy) return;
		const open = doc;
		busy = true;
		try {
			if (!(await session.flush())) return;
			const version = open.base[NEUTRAL_LOCALE];
			if (version === null || version === undefined) return;
			const key = open.key(NEUTRAL_LOCALE);
			const result = await publishPartial([{ key, version }]);
			const outcome = publishOutcome(key, { published: result.published.map((p) => p.key), refused: result.refused });
			problems =
				outcome?.kind === 'problems' ? outcome.problems
				: outcome?.kind === 'error' ? [outcome.message]
				: outcome?.kind === 'unreachable' ? [m['creator.experiences_unreachable']()]
				: [];
			if (outcome?.kind === 'published') toast(m['creator.experiences_publish_ok'](), { type: 'success' });
		} catch (e) {
			problems = [e instanceof Error && e.message ? e.message : m['creator.experiences_unreachable']()];
		} finally {
			busy = false;
		}
		await open.load();
	}

	/**
	 * An Inspector field changed: the document takes it (one undo step per field, ContentDoc), the stage shows every
	 * target of the edited entity at once, and the shared autosave (#760) saves the draft a moment after the last change.
	 */
	function onedit(at: (string | number)[], value: string | number | boolean) {
		const ref = selected;
		if (doc && ref && ref.path === null) {
			// a target: its place on the surface (mm); the stage lays it out there at once
			const target = targetDocPath(configuration, ref.target);
			if (!target || at[0] !== 'place') return;
			doc.set([...target, ...at], value);
			showPlace(ref.target);
			return;
		}
		const node = ref ? nodeDocPath(configuration, ref) : null;
		if (!doc || !ref || !node) return;
		doc.set([...node, ...at], value);
		showEdits(ref.target);
	}

	/**
	 * A gizmo drag on the stage: the node's placement goes into the document (one undo step per drag: the edits of one
	 * drag merge), saved like an Inspector edit; the stage already shows it, so nothing is sent back while it lasts.
	 */
	function onplace(ref: NodeRef, placement: StagePlacement) {
		const node = nodeDocPath(configuration, ref);
		if (!doc || !node) return;
		const before = doc.get(NEUTRAL_LOCALE, [...node, 'params', 'scale']);
		const scale = typeof placement.scale === 'number' && Array.isArray(before) ? [placement.scale, placement.scale, placement.scale] : placement.scale;
		doc.set([...node, 'params'], { ...((doc.get(NEUTRAL_LOCALE, [...node, 'params']) as object | undefined) ?? {}), position: placement.position, rotation: placement.rotation, scale }, `gizmo:${node.join('.')}`);
	}

	function showEdits(target: string) {
		const now = (content as { configuration?: unknown } | null)?.configuration;
		const entries = ((now as { entries?: { target?: { id?: string } }[] } | undefined)?.entries ?? []).map((e) => e.target);
		const trees: Record<string, object | null> = {};
		for (const id of new Set([target, ...targetsSharing(now, target)])) {
			const tree = entityOf(now, entries.find((t) => t?.id === id));
			trees[id] = tree ? ($state.snapshot(tree) as object) : null;
		}
		edited = { at: Date.now(), trees };
	}

	/**
	 * A target's tree changed (a part added, removed, moved or duplicated): the document takes the whole tree as one undo
	 * step – the shared entity when the target shows one, so every target that shows it changes – the stage shows it, the
	 * selection follows the part. No tree: the target's content is removed (a shared entity stays for the others).
	 */
	function setTree(target: string, tree: XrNode | null, path: number[] | null) {
		const slot = entitySlot(configuration, target);
		if (!doc || !slot) return;
		if (tree) doc.set(slot.at, tree);
		else doc.set(slot.own, undefined);
		selected = { target, path: tree ? path : null };
		showEdits(target);
	}

	/** the tree of a target as it is now, a plain copy (the document's own objects stay untouched) */
	const treeOf = (target: string) => {
		const slot = entitySlot(configuration, target);
		return slot?.tree ? ($state.snapshot(slot.tree) as unknown as XrNode) : null;
	};

	/**
	 * The Tree's row menu, keys and footer on a part: moves keep it and its children; duplicate copies it right after it
	 * (its names free ones, its ids gone), group puts it into a new group in its place; the root's removal clears the
	 * target's content.
	 */
	function onpart(ref: NodeRef, action: PartAction) {
		const root = treeOf(ref.target);
		if (!root) return;
		const path = ref.path ?? [];
		if (action === 'remove') {
			if (!path.length) setTree(ref.target, null, null);
			else setTree(ref.target, xrRemoveNode(root, path), path.slice(0, -1));
			return;
		}
		const edit =
			action === 'up' ? xrShiftNode(root, path, -1)
			: action === 'down' ? xrShiftNode(root, path, 1)
			: action === 'indent' ? xrIndentNode(root, path)
			: action === 'outdent' ? xrOutdentNode(root, path)
			: action === 'duplicate' ? xrDuplicateNode(root, path)
			: action === 'group' ? xrGroupNode(root, path)
			: null;
		if (edit) setTree(ref.target, edit.tree, edit.path);
	}

	/**
	 * The footer's +: a part of the type (with its file) where OSCT's editor puts it – the target's root when it shows
	 * nothing yet, else next to a part that holds a file or inside the selected part; a surface of a preset beside the
	 * scene's row (the stage opens the scene anew to lay it out).
	 */
	function onadd(add: XrTreeAdd, file: XrFile | null) {
		if (add.add === 'surface') {
			const scenes = (configuration as { scenes?: { id?: string; surfaces?: unknown[] }[] } | undefined)?.scenes ?? [];
			const index = scenes.findIndex((s) => s.id === scene);
			if (!doc || index < 0) return;
			const { surfaces } = xrAddSurface(add.preset, $state.snapshot(scenes[index].surfaces ?? []) as unknown[]);
			doc.set(['configuration', 'scenes', index, 'surfaces'], surfaces);
			run += 1;
			return;
		}
		const ref = selected;
		if (add.add !== 'node' || !ref) return;
		const root = treeOf(ref.target);
		const node = xrNewNode(add.type, file ? { id: `${ref.target}-${Date.now().toString(36)}`, assetType: file.assetType, src: file.src } : undefined);
		const path = ref.path ?? [];
		const edit = xrAddNode(root, path, node, xrDefaultAddPlace(root ? xrNodeAt(root, path) : undefined));
		setTree(ref.target, edit.tree, edit.path);
	}

	function showPlace(target: string) {
		const place = targetPlace((content as { configuration?: unknown } | null)?.configuration, target);
		if (place) moved = { at: Date.now(), target, place };
	}

	/** the open scene's surfaces as a key: an undo or redo that changes them opens the stage anew to lay them out */
	const surfacesKey = () => JSON.stringify(((content as { configuration?: { scenes?: { id?: string; surfaces?: unknown }[] } } | null)?.configuration?.scenes ?? []).find((s) => s.id === scene)?.surfaces ?? null);

	function step(back: boolean) {
		if (!doc) return;
		const before = surfacesKey();
		if (back) doc.undo();
		else doc.redo();
		if (surfacesKey() !== before) run += 1;
		if (selected) {
			showEdits(selected.target);
			showPlace(selected.target);
		}
	}

	// the Spores tab (#763): the experience's logic as a weave from its top level (its scenes, groups, plugins and what
	// joins them – OSCT's projection), offered while the experience is open; read only
	// until the scope's ops exist. A scene's own level is entered from there (its weave is empty without rules of its own).
	const sporesAddress = $derived(id ? `experience:${id}` : '');
	$effect(() => {
		const address = sporesAddress;
		if (!address) return;
		sporeScope.set({
			address,
			weave: () => (content && id ? projectXr({ ...($state.snapshot(content) as object), id }, address) : null)
		});
		return () => sporeScope.drop(address);
	});

	usePageNav({ fill: true, extra: () => (facts ? [{ href: experienceHref(facts.id), label: titleIn(facts.title, locale, facts.id) }] : []) });
	usePageDoc(() =>
		doc ? { publish: publishMenu, docs: [doc], unpublished, errors: problems.length, saving: session.saving, undo: () => step(true), redo: () => step(false) } : null
	);
	useTabs(() => ({ tree: facts ? tree : null, inspector: facts ? inspector : null, experience: facts ? meta : null, scenes: facts && scenes.length ? strip : null }));
</script>

{#snippet publishMenu()}
	<div class="publish-menu">
		<button type="button" class:primary={unpublished} disabled={!unpublished || busy} onclick={() => void publish()} data-testid="experience-publish"><Icon name="upload-cloud" />{m['creator.publish']()}</button>
		{#if !unpublished}<p class="muted">{m['creator.experiences_nothing_new']()}</p>{/if}
	</div>
{/snippet}

{#snippet tree()}
	{#if facts}<ExperienceTree {configuration} {scene} {selected} onselect={(ref) => (selected = ref)} {onpart} {onadd} files={experienceFiles(configuration)} />{/if}
{/snippet}

{#snippet inspector()}
	<ExperienceInspector {configuration} {selected} {onedit} />
{/snippet}

{#snippet meta()}
	{#if facts}<ExperienceMeta {facts} typeName={TYPES[facts.type]?.() ?? facts.type} toons={facts.toons.map((t) => nav.toonName(t)).join(' · ')} {problems} {plugins} />{/if}
{/snippet}

{#snippet strip()}
	{#if facts}<ExperienceScenes {scenes} current={scene} assets={facts.assets} bind:view bind:tool playing={shown.phase === 'playing'} onpick={(s) => ((picked = s), (view = 'reader'), (selected = null))} onreplay={() => ((run += 1), (view = 'reader'))} />{/if}
{/snippet}

<div class="xr-workspace" data-testid="experience-workspace">
	{#if facts && scene}
		<ExperienceStage {content} assets={facts.assets} {scene} {view} {run} {selected} onpick={(ref) => (selected = ref)} {edited} {moved} {tool} handles={shell.settings.gizmoAlways} {onplace} bind:shown />
	{:else}
		<div class="empty">
			{#if !id}
				<p>{m['creator.experiences_from_library']({ chip: m['library.entities']() })}</p>
				<button type="button" onclick={() => shell.open('library')} data-testid="experience-open-library"><Icon name="grid" />{m['shell.library']()}</button>
			{:else if doc?.error}
				<p role="alert">{doc.error}</p>
			{:else if doc && !doc.loading}
				<p>{m['creator.experiences_stage_none']()}</p>
			{:else}
				<p role="status">{m['creator.experiences_stage_loading']()}</p>
			{/if}
		</div>
	{/if}
</div>

<style>
	/* the host's stage holds the page full-bleed (usePageNav fill, preset clear off): the workspace fills it */
	.xr-workspace {
		position: relative;
		flex: 1;
		min-height: 0;
		background: var(--tool-ground);
	}
	.empty {
		position: absolute;
		inset: 0;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		gap: 0.75rem;
		color: var(--text-muted);
	}
	.empty p {
		margin: 0;
	}
	.publish-menu {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	.publish-menu p {
		margin: 0;
	}
	.muted {
		color: var(--text-muted);
	}
</style>
