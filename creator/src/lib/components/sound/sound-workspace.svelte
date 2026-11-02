<script lang="ts">
	// The Sound design workspace (#282, research/EDITOR.md, owner 2026-11-02): a split screen – the designer on one side, the
	// deck (the engine host, #298) on the other, so every edit is heard and seen at once; below ~900 px the designer stacks
	// above the deck. The subject is a rig (`rig:character.<id>`: its character plays on the deck, the Rust tab picks a
	// module) or a module (`module:<id>`: the Rust tab IS it). The modules are `module:<id>` documents, drafts and history
	// like any catalog document; the compile service writes the descriptor into them (#281). Playing the compiled module
	// in the engine comes with #290.
	import { onMount, untrack } from 'svelte';
	import { MediaQuery } from 'svelte/reactivity';
	import { goto, beforeNavigate } from '$app/navigation';
	import { page } from '$app/state';
	import { Divider, askDialog, panelWidth } from '@scutoons/ui';
	import { createDesignerApi, isModuleId, moduleContent, RigDesigner, setupRigDesigner, STARTER_FILES } from '@scutoons/rig-designer';
	import { getSessionToken } from '@scutoons/api';
	import { API_URL } from '$lib/config.ts';
	import { api, putDraft } from '$lib/api.ts';
	import { leaveGuard } from '$lib/editor/ask.ts';
	import { editHref, selectedModule, type EditTarget } from '$lib/editor/edit-key.ts';
	import { createModuleStore, moduleKey } from '$lib/editor/module-store.ts';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { m } from '$lib/paraglide/messages.js';
	import { isolateThisPage } from '$lib/isolation.svelte.ts';
	import DeckHalf from '$lib/components/sound/deck-half.svelte';
	import DesignerHalf from '$lib/components/sound/designer-half.svelte';

	let { target }: { target: EditTarget } = $props();

	// live-core's shared ring needs cross-origin isolation: the editor's pages get the headers, the rest of the creator not
	isolateThisPage();
	setupRigDesigner({ locale: getLocale });

	const designer = new RigDesigner(
		createDesignerApi({ fetch: (...args) => fetch(...args), baseUrl: API_URL, getToken: getSessionToken }),
		createModuleStore()
	);
	beforeNavigate(leaveGuard(() => designer.dirty));

	let modules = $state<string[]>([]);
	/** a refused New module, one muted sentence */
	let notice = $state('');
	async function loadModules(): Promise<void> {
		try {
			// the catalog kind `modules`: every `module:<id>` document, a removed one is gone (server CatalogDocs)
			const r = await api.get<{ entries: { id: string; removed: boolean }[] }>('/catalog/docs/modules');
			modules = r.entries.filter((e) => !e.removed && isModuleId(e.id)).map((e) => e.id).sort();
		} catch {
			modules = [];
		}
	}
	onMount(() => void loadModules());

	// the address decides the module: a module page's own id, or a rig page's ?sel=module:<id>
	const selected = $derived(selectedModule(target, page.url.searchParams.get('sel')));
	$effect(() => {
		const id = selected;
		untrack(() => {
			if (id !== designer.moduleId) void designer.open(id);
		});
	});

	function select(id: string | null) {
		void goto(editHref(target.key, { w: 'sound', sel: id ? moduleKey(id) : null }), { replaceState: true, keepFocus: true, noScroll: true });
	}

	/** a new module: its id, the walk module as the template, a draft right away – then it opens */
	async function newModule(): Promise<void> {
		const typed = await askDialog({
			title: m['creator.sound_new_module'](),
			message: m['creator.sound_new_module_hint'](),
			label: 'id',
			placeholder: 'e.g. my_walk',
			confirmLabel: m['creator.sound_new_module_create']()
		});
		const id = typed?.trim();
		if (!id) return;
		notice = '';
		if (!isModuleId(id)) return void (notice = m['creator.sound_id_invalid']({ id }));
		await loadModules();
		if (modules.includes(id)) return void (notice = m['creator.sound_id_taken']({ id }));
		try {
			await putDraft(moduleKey(id), moduleContent(id, id, STARTER_FILES, null), null, 'new module');
		} catch (e) {
			return void (notice = e instanceof Error ? e.message : String(e));
		}
		await loadModules();
		if (target.kind === 'module') void goto(editHref(moduleKey(id), { w: 'sound' }));
		else select(id);
	}

	// ── the split: the deck on the right (remembered width, a Divider); below ~900 px they stack ──
	const stacked = new MediaQuery('max-width: 56rem');
	let splitEl = $state<HTMLElement>();
	const deckSize = panelWidth('sc-panel-width:sound-deck', 26, () => splitEl?.getBoundingClientRect().right ?? window.innerWidth);
</script>

<div class="split" class:stacked={stacked.current} bind:this={splitEl} style:--deck-width={deckSize.css} data-testid="sound-workspace">
	<section class="half designer-half" aria-label={m['creator.sound_designer']()}>
		<DesignerHalf
			{designer}
			fixed={target.kind === 'module' ? target.id : null}
			{modules}
			{selected}
			{notice}
			onselect={select}
			onnew={newModule}
		/>
	</section>
	<section class="half deck-half" id="sound-deck" aria-label={m['creator.sound_deck']()}>
		{#key target.key}
			<DeckHalf character={target.kind === 'rig' ? target.character : 'catpope'} />
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
