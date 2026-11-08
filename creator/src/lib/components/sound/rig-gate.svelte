<script lang="ts">
	// The rig page's gate (#425, owner 2026-11-08): `rig:character.<id>` of a character WITHOUT a rig, and no module opened
	// yet, shows the two ways in (RigEmptyState) instead of the designer; any other page – a module page, a rig page whose
	// character has a rig (or a draft of one) or has a module open – shows the designer as it was. Whether the character has
	// a rig is the core's word (space-web `rig_coverage`, the same rule the Assignment tab lists); what a module becomes as a
	// track is the core's too (`rig_module_instrument`). First step only: "start from existing modules" stores the instrument
	// that wraps ONE chosen module and a rig draft with that one track; wiring modules together is the patch editor's (#203).
	import { onMount, type Snippet } from 'svelte';
	import { STARTER_FILES, addTrack, emptyRig, type RigCoverage } from '@scutoons/rig-designer';
	import { loadRigRules } from '@scutoons/live-deck/assign';
	import { api, getDraft, library, publishVersion, putDraft } from '$lib/api.ts';
	import { firstDraft, idProblem } from '$lib/editor/module-catalog.ts';
	import { moduleKey } from '$lib/editor/module-store.ts';
	import { coverageRequest, createRigStore, instrumentKey, moduleInstrumentRequest, rigLists, type CatalogEntryLike } from '$lib/editor/rig-assign.ts';
	import { m } from '$lib/paraglide/messages.js';
	import RigEmptyState from '$lib/components/sound/rig-empty-state.svelte';

	let { subject = null, character = '', selected, modules, factory = [], onopen, reload, children }: {
		/** a rig page's subject (`character.<id>`); null = a module page: no gate */
		subject?: string | null;
		character?: string;
		/** the module open on the page (the Rust tab shows it): the page is no longer empty */
		selected: string | null;
		modules: string[];
		factory?: string[];
		onopen: (moduleId: string) => void;
		/** the workspace's module list anew */
		reload: () => Promise<void>;
		children: Snippet;
	} = $props();

	const rules = loadRigRules('/live/');
	const store = createRigStore({ getDraft, putDraft, publishVersion });
	let loaded = $state(false);
	let empty = $state(false);
	let busy = $state(false);
	let problem = $state('');
	const said = (e: unknown) => (e instanceof Error ? e.message : String(e));

	/** the core's coverage of the registry's characters over the catalog's rigs: does this character have a rig? */
	async function refresh(): Promise<void> {
		if (!subject) return void (loaded = true);
		try {
			const kind = (k: string) => api.get<{ entries: CatalogEntryLike[] }>(`/catalog/docs/${k}`).then((r) => r.entries).catch(() => []);
			const [characters, i, p, r] = await Promise.all([library.fetchRegistry({ draft: true }).then((x) => x.characters ?? []).catch(() => []), kind('instruments'), kind('presets'), kind('rigs')]);
			const coverage: RigCoverage[] = JSON.parse((await rules).coverage(coverageRequest(characters, rigLists(i, p, r))));
			empty = coverage.find((c) => c.subject === subject)?.default === 'missing';
		} catch (e) {
			problem = said(e);
		} finally {
			loaded = true;
		}
	}
	onMount(() => void refresh());

	/** way 1: ONE module as the rig's first track – the core makes the instrument that wraps it, both are stored as drafts */
	async function start(moduleId: string): Promise<void> {
		if (!subject || busy) return;
		busy = true;
		problem = '';
		try {
			const docs = await api.get<{ entries: { id: string; removed: boolean; content: unknown }[] }>('/catalog/docs/modules');
			const doc = docs.entries.find((e) => e.id === moduleId && !e.removed);
			if (!doc) throw new Error(`module:${moduleId}`);
			const instrument = JSON.parse((await rules).moduleInstrument(moduleInstrumentRequest(doc))) as { id: string };
			await putDraft(instrumentKey(instrument.id), instrument, null, `wraps module:${moduleId}`);
			const saved = await store.save(subject, addTrack(emptyRig(subject), instrument.id, []));
			if (!saved.ok) throw new Error(saved.message);
			await refresh();
		} catch (e) {
			problem = said(e);
		} finally {
			busy = false;
		}
	}

	/** way 2: a new module for this character – a `module:<character>` draft (the starter), opened on the Rust tab */
	async function create(): Promise<void> {
		if (busy) return;
		busy = true;
		problem = '';
		try {
			if (!modules.includes(character)) {
				if (idProblem(character, modules) === 'invalid') throw new Error(m['creator.sound_id_invalid']({ id: character }));
				await putDraft(moduleKey(character), firstDraft(character, null, STARTER_FILES), null, `new module for character.${character}`);
				await reload();
			}
			onopen(character);
		} catch (e) {
			problem = said(e);
		} finally {
			busy = false;
		}
	}
</script>

{#if !loaded}
	<!-- the core's word on the rig first: no flash of the designer -->
{:else if subject && empty && !selected}
	<RigEmptyState {character} {modules} {factory} {busy} {problem} onstart={(id) => void start(id)} oncreate={() => void create()} />
{:else}
	{@render children()}
{/if}
