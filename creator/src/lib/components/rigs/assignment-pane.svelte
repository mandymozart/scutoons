<script lang="ts">
	// The Assignment tab of the Sound page (#283): the open character's rig `rig:<subject>` as the designer's form, saved as
	// a draft and published like any catalog document (rig-assign.ts), heard on the deck half at every change (the core
	// builds the bundle from the catalog, `rules.bundle`, the engine host plays it, `play`), and the characters without a
	// default rig (the core's `rules.coverage` over the registry and the catalog's rig list). The rules come in from the
	// designer half (space-web, loaded once per page); this pane holds only the draft and the lists.
	import { onMount } from 'svelte';
	import { AssignmentTab, RigCoverageList, designerMessages as m, emptyRig, type RigCoverage, type RigDoc } from '@scutoons/rig-designer';
	import { confirmDialog } from '@scutoons/ui';
	import { api, getDraft, library, publishVersion, putDraft } from '$lib/api.ts';
	import { editHref } from '$lib/editor/edit-key.ts';
	import { NO_RIG_LISTS, bundleRequest, coverageRequest, createRigStore, rigKey, rigLists, type CatalogEntryLike, type RigLists } from '$lib/editor/rig-assign.ts';
	import { toast } from '$lib/toasts.ts';

	let { subject, rules, play, moods = [] }: {
		/** the open rig's subject, `character.<id>[.<persona>]` */
		subject: string;
		/** the core's assignment rules (space-web `rig_coverage`, `rig_catalog_bundle`) */
		rules: Promise<{ coverage(request: string): string; bundle(request: string): string }>;
		/** plays a bundle on the deck half (throws the core's words) */
		play: (bundle: string) => void;
		/** the character's moods (the deck's) */
		moods?: string[];
	} = $props();

	const store = createRigStore({ getDraft, putDraft, publishVersion });
	let lists = $state<RigLists>(NO_RIG_LISTS);
	let characters: { id: string; personas?: { id: string }[] }[] = [];
	let coverage = $state<RigCoverage[]>([]);
	let rig = $state<RigDoc | null>(null);
	let saved = $state('null');
	let problem = $state('');
	let busy = $state(false);
	const dirty = $derived(JSON.stringify(rig) !== saved);
	const mine = $derived(coverage.find((c) => c.subject === subject) ?? null);
	const said = (e: unknown) => (e instanceof Error ? e.message : String(e));

	async function loadLists(): Promise<void> {
		const kind = (k: string) => api.get<{ entries: CatalogEntryLike[] }>(`/catalog/docs/${k}`).then((r) => r.entries).catch(() => []);
		const [i, p, r] = await Promise.all([kind('instruments'), kind('presets'), kind('rigs')]);
		lists = rigLists(i, p, r);
		try {
			coverage = JSON.parse((await rules).coverage(coverageRequest(characters, lists)));
		} catch (e) {
			problem = said(e);
		}
	}

	onMount(async () => {
		characters = (await library.fetchRegistry({ draft: true }).catch(() => null))?.characters ?? [];
		const loaded = await store.load(subject);
		if (loaded.ok) [rig, saved] = [loaded.rig, JSON.stringify(loaded.rig)];
		else problem = loaded.message;
		await loadLists();
	});

	/** the draft on the deck: the core's bundle from the catalog, played; what the core says is shown */
	async function hear(): Promise<void> {
		if (!rig) return;
		try {
			play((await rules).bundle(bundleRequest($state.snapshot(rig), lists)));
			problem = '';
		} catch (e) {
			problem = said(e);
		}
	}

	function change(next: RigDoc): void {
		rig = next;
		void hear();
	}

	async function save(): Promise<void> {
		if (!rig || busy) return;
		busy = true;
		const r = await store.save(subject, $state.snapshot(rig));
		busy = false;
		if (!r.ok) return void toast(r.message, { type: 'error' });
		saved = JSON.stringify(rig);
		await loadLists();
	}

	async function publish(): Promise<void> {
		if (!rig || dirty || busy) return;
		if (!(await confirmDialog({ title: m['designer.assign_publish'](), message: m['designer.assign_publish_confirm']({ subject }), confirmLabel: m['designer.assign_publish']() }))) return;
		busy = true;
		const r = await store.publish(subject);
		busy = false;
		if (!r.ok) return void toast(r.message, { type: 'error' });
		toast(m['designer.assign_published']({ subject }));
		await loadLists();
	}
</script>

<div class="pane" data-testid="assignment-pane">
	<div class="form">
		<AssignmentTab {rig} instruments={lists.instruments} presets={lists.presets} {moods} coverage={mine} {problem} onchange={change} oncreate={() => change(emptyRig(subject))} />
	</div>
	<RigCoverageList list={coverage} current={subject} href={(c) => editHref(rigKey(c.subject), { w: 'sound' })} />
	{#if rig}
		<footer class="actions">
			<span class="state">{dirty ? m['designer.unsaved']() : ''}</span>
			<button type="button" onclick={hear} title={m['designer.assign_hear_title']()} data-testid="assign-hear">{m['designer.assign_hear']()}</button>
			<button type="button" onclick={publish} disabled={dirty || busy || !lists.rigs.find((r) => r.id === subject)?.draft} title={m['designer.assign_publish_title']()} data-testid="assign-publish">{m['designer.assign_publish']()}</button>
			<button type="button" onclick={save} disabled={!dirty || busy} title={m['designer.assign_save_title']()} data-testid="assign-save">{m['designer.save']()}</button>
		</footer>
	{/if}
</div>

<style>
	.pane {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}
	.form {
		flex: 1;
		min-height: 0;
	}
	.actions {
		flex: none;
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-05) var(--space-1);
		border-top: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
	}
	.state {
		flex: 1;
		color: var(--text-muted);
		font-family: var(--font-mono);
		font-size: 0.75rem;
	}
</style>
