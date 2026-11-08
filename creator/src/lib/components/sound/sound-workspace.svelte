<script lang="ts">
	// The Sound design workspace (#282, research/EDITOR.md): the designer beside the deck (the engine host, #298), so every edit is
	// heard and seen at once; below ~900 px they stack. The subject is a rig (`rig:character.<id>`: its character plays, the Rust tab
	// is its policy, #412, or a picked module) or a module (`module:<id>`: the Rust tab IS it). A module plays in the browser's engine
	// with #290; factory modules (#350) are read-only: the API refuses every change, the designer shows them locked with Derive.
	import { onMount, untrack } from 'svelte';
	import { MediaQuery } from 'svelte/reactivity';
	import { goto, beforeNavigate } from '$app/navigation';
	import { page } from '$app/state';
	import { Divider, Icon, askDialog, choiceDialog, confirmDialog, panelWidth } from '@scutoons/ui';
	import { createDesignerApi, RigDesigner, setupRigDesigner, STARTER_FILES, type GraphWiring } from '@scutoons/rig-designer';
	import { getSessionToken } from '@scutoons/api';
	import type { EngineHost } from '@scutoons/engine-host';
	import { API_URL } from '$lib/config.ts';
	import { api, putDraft } from '$lib/api.ts';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import { toast } from '$lib/toasts.ts';
	import { leaveGuard } from '$lib/editor/ask.ts';
	import { editHref, selectedModule, type EditTarget } from '$lib/editor/edit-key.ts';
	import { createModuleStore, moduleKey } from '$lib/editor/module-store.ts';
	import { factoryChoices, firstDraft, idProblem, NO_MODULES, readCatalog, templateOf, type ModuleCatalog } from '$lib/editor/module-catalog.ts';
	import { openRigSession } from '$lib/editor/rig-session.svelte.ts';
	import { loadRigRules } from '@scutoons/live-deck/assign';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { isolateThisPage } from '$lib/isolation.svelte.ts';
	import DeckHalf from '$lib/components/sound/deck-half.svelte';
	import DesignerHalf from '$lib/components/sound/designer-half.svelte';

	let { target }: { target: EditTarget } = $props();

	// live-core's shared ring needs cross-origin isolation: the editor's pages get the headers, the rest of the creator not
	isolateThisPage();
	setupRigDesigner({ locale: getLocale });

	const store = createModuleStore();
	const designer = new RigDesigner(createDesignerApi({ fetch: (...args) => fetch(...args), baseUrl: API_URL, getToken: getSessionToken }), store);
	beforeNavigate(leaveGuard(() => designer.dirty));

	// the catalog kind `modules` (server CatalogDocs) – the factory modules (#350) marked, New module's templates
	let catalog = $state<ModuleCatalog>(NO_MODULES);
	let notice = $state(''); // a refused New module, one muted sentence
	async function loadModules(): Promise<void> {
		const r = await api.get<{ entries: { id: string; removed: boolean; content: unknown }[] }>('/catalog/docs/modules').catch(() => null);
		catalog = r ? readCatalog(r.entries) : NO_MODULES;
	}
	onMount(() => void loadModules());

	// svelte-ignore state_referenced_locally
	const session = openRigSession(target, loadRigRules('/live/'), designer); // the rig open (a rig page): the Assignment tab's and the policy's (#412)

	// the address decides the module: a module page's own id, a rig page's ?sel=module:<id>, else the rig's policy
	const selected = $derived(selectedModule(target, page.url.searchParams.get('sel')) ?? session?.policy ?? null);
	$effect(() => {
		const id = selected;
		untrack(() => {
			if (id !== designer.moduleId) void designer.open(id);
		});
	});

	function select(id: string | null) {
		void loadModules(); // New policy (#412): the picker knows it
		void goto(editHref(target.key, { w: 'sound', sel: id ? moduleKey(id) : null }), { replaceState: true, keepFocus: true, noScroll: true });
	}

	/** New module (a copy of a picked factory module, else the starter) or Derive (of the open one): a draft, then it opens */
	async function create(derive: boolean): Promise<void> {
		const choices = factoryChoices(catalog);
		const pick = derive || choices.length === 0 ? null : await choiceDialog({ title: m['creator.sound_start_from'](), message: m['creator.sound_start_from_hint'](), choices });
		if (!derive && choices.length > 0 && !pick) return;
		const from = derive && designer.moduleId ? templateOf(designer.moduleId, designer.name, designer.files, designer.outcome) : pick ? { id: pick, module: catalog.factory[pick] } : null;
		const message = from ? m['creator.sound_derive_hint']({ origin: from.id }) : m['creator.sound_new_module_hint']();
		const typed = await askDialog({ title: from ? m['creator.sound_derive']() : m['creator.sound_new_module'](), message, label: 'id', placeholder: `e.g. my_${from?.id ?? 'walk'}`, confirmLabel: m['creator.sound_new_module_create']() });
		const id = typed?.trim() ?? '';
		if (!id) return;
		await loadModules();
		const problem = idProblem(id, catalog.ids);
		notice = problem === 'invalid' ? m['creator.sound_id_invalid']({ id }) : problem === 'taken' ? m['creator.sound_id_taken']({ id }) : '';
		if (problem) return;
		try {
			await putDraft(moduleKey(id), firstDraft(id, from, STARTER_FILES), null, from ? `derived from module:${from.id}` : 'new module');
		} catch (e) {
			return void (notice = e instanceof Error ? e.message : String(e));
		}
		await loadModules();
		if (target.kind === 'module') void goto(editHref(moduleKey(id), { w: 'sound' }));
		else select(id);
	}

	// ── the top bar (owner 2026-11-02: the editor's actions move there step by step, as the map's did): Publish of the open
	// module – needs a saved draft whose compile is current (a module without its descriptor cannot be played) ──
	const canPublish = $derived(!!designer.moduleId && !designer.factory && !designer.dirty && !!designer.outcome && !designer.stale && !designer.compiling && !designer.saving);
	async function publish(): Promise<void> {
		const id = designer.moduleId;
		if (!id || !canPublish) return;
		const ok = await confirmDialog({ title: m['creator.sound_publish'](), message: m['creator.sound_publish_confirm']({ id }), confirmLabel: m['creator.sound_publish']() });
		if (!ok) return;
		const r = await store.publish(id);
		if (r.ok) toast(m['creator.sound_published']({ id }));
		else toast(r.message, { type: 'error' });
	}
	usePageNav({
		fill: true,
		rail: true,
		extra: () => [{ href: editHref(target.key, { w: 'sound' }), label: `${m['creator.sound_title']()} · ${target.key}` }],
		actions: topBar
	});

	// ── the split: the deck on the right (remembered width, a Divider); below ~900 px they stack ──
	const stacked = new MediaQuery('max-width: 56rem');
	let splitEl = $state<HTMLElement>();
	const deckSize = panelWidth('sc-panel-width:sound-deck', 26, () => splitEl?.getBoundingClientRect().right ?? window.innerWidth);
	// the deck half's engine and the core's wiring of what it plays: the designer's Nodes tab draws them (#282)
	let host = $state<EngineHost | null>(null);
	let wiring = $state<GraphWiring>();
</script>

{#snippet topBar()}
	<button type="button" class="sc-round" disabled={!canPublish} onclick={publish} title={canPublish ? m['creator.sound_publish']() : m['creator.sound_publish_needs']()} aria-label={m['creator.sound_publish']()} data-testid="sound-publish">
		<Icon name="upload-cloud" size="1rem" />
	</button>
{/snippet}

<div class="split" class:stacked={stacked.current} bind:this={splitEl} style:--deck-width={deckSize.css} data-testid="sound-workspace">
	<section class="half designer-half" aria-label={m['creator.sound_designer']()}>
		<DesignerHalf
			{designer}
			fixed={target.kind === 'module' ? target.id : null}
			{session}
			modules={catalog.ids}
			factory={Object.keys(catalog.factory)}
			{selected}
			{notice}
			{host}
			{wiring}
			onselect={select}
			onnew={() => create(false)}
			onderive={() => create(true)}
		/>
	</section>
	<section class="half deck-half" id="sound-deck" aria-label={m['creator.sound_deck']()}>
		{#key target.key}
			<DeckHalf character={'character' in target ? target.character : 'catpope'} bind:host bind:wiring />
		{/key}
	</section>
	{#if !stacked.current}
		<Divider size={deckSize} edge="left" at="var(--deck-width, 26rem)" label={m['creator.resize_panel']()} controls="sound-deck" />
	{/if}
</div>

<style>
	.split {
		position: relative;
		display: grid;
		grid-template-columns: minmax(0, 1fr) var(--deck-width, 26rem);
		height: 100%;
		min-height: 0;
	}
	.half {
		min-width: 0;
		min-height: 0;
	}
	.designer-half {
		border-right: var(--border-width-regular) solid var(--ink);
	}
	.split.stacked {
		grid-template-columns: minmax(0, 1fr);
		grid-template-rows: minmax(0, 3fr) minmax(0, 2fr);
	}
	.stacked .deck-half {
		overflow: auto;
	}
	.stacked .designer-half {
		border-right: 0;
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
</style>
