<script lang="ts">
	// The Sound design workspace (#282, EDITOR.md) on the workspace host (RULES H5, lap 2 T10): every part of the module editor
	// is a view – the code (the main view, on the stage), Parameters, Nodes, the rig's Assignment, the Monitor, Out, Knobs,
	// Problems, Console – a panel's tab like any other workspace's (lib/shell/presets.ts). A rig's Rust view is its policy
	// (#412), no rig = RigGate (#425). The views other than the code: sound-views.svelte.
	import { onMount, untrack } from 'svelte';
	import { goto, beforeNavigate } from '$app/navigation';
	import { page } from '$app/state';
	import { askDialog, choiceDialog } from '@scutoons/ui';
	import { createDesignerApi, RigDesigner, setupRigDesigner, STARTER_FILES } from '@scutoons/rig-designer';
	import { getSessionToken } from '@scutoons/api';
	import { API_URL } from '$lib/config.ts';
	import { api, putDraft } from '$lib/api.ts';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import { usePageDoc } from '$lib/shell/shell.svelte.ts';
	import { leaveGuard } from '$lib/editor/ask.ts';
	import { editHref, selectedModule, type EditTarget } from '$lib/editor/edit-key.ts';
	import { createModuleStore, moduleKey } from '$lib/editor/module-store.ts';
	import { factoryChoices, firstDraft, idProblem, NO_MODULES, readCatalog, templateOf, type ModuleCatalog } from '$lib/editor/module-catalog.ts';
	import { openRigSession } from '$lib/editor/rig-session.svelte.ts';
	import { loadRigRules } from '@scutoons/live-deck/assign';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { isolateThisPage } from '$lib/isolation.svelte.ts';
	import ModuleView from '$lib/components/sound/module-view.svelte';
	import RigGate from '$lib/components/sound/rig-gate.svelte';
	import SoundViews from '$lib/components/sound/sound-views.svelte';

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
		const pick = derive || choices.length === 0 ? null : await choiceDialog({ title: m['creator.sound_start_from'](), hover: m['creator.sound_start_from_hint'](), choices });
		if (!derive && choices.length > 0 && !pick) return;
		const from = derive && designer.moduleId ? templateOf(designer.moduleId, designer.name, designer.files, designer.outcome) : pick ? { id: pick, module: catalog.factory[pick] } : null;
		const hover = from ? m['creator.sound_derive_hint']({ origin: from.id }) : m['creator.sound_new_module_hint']();
		const typed = await askDialog({ title: from ? m['creator.sound_derive']() : m['creator.sound_new_module'](), hover, label: 'id', placeholder: `e.g. my_${from?.id ?? 'walk'}`, confirmLabel: m['creator.sound_new_module_create']() });
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

	// ── the bar's one Publish (RULES H18): the open module goes only as a saved draft whose compile is current (a module
	// without its descriptor cannot be played) – else the dialog holds it back with the reason; the open rig is saved first ──
	const canPublish = $derived(!!designer.moduleId && !designer.factory && !designer.dirty && !!designer.outcome && !designer.stale && !designer.compiling && !designer.saving);
	const held = $derived(designer.moduleId && !designer.factory && !canPublish ? [{ key: moduleKey(designer.moduleId), reason: m['creator.sound_publish_needs']() }] : []);
	usePageDoc(() => ({ held, ...session?.publishHooks }));
	usePageNav({
		fill: true,
		extra: () => [{ href: editHref(target.key, { w: 'sound' }), label: `${m['creator.sound_title']()} · ${target.key}` }]
	});
</script>

{#snippet rustTab()}
	<RigGate subject={'subject' in target ? target.subject : null} character={'character' in target ? target.character : ''} {selected} modules={catalog.ids} factory={Object.keys(catalog.factory)} onopen={select} reload={loadModules}>
		<ModuleView
			{designer}
			fixed={target.kind === 'module' ? target.id : null}
			{session}
			modules={catalog.ids}
			factory={Object.keys(catalog.factory)}
			{selected}
			{notice}
			onselect={select}
			onnew={() => create(false)}
			onderive={() => create(true)}
		/>
	</RigGate>
{/snippet}

<SoundViews {designer} {session} character={('character' in target && target.character) || 'catpope'} rust={rustTab} />
