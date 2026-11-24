<script lang="ts">
	// The designer half of the Sound design workspace (#282): the module being edited (a rig page picks one of the stored
	// modules, its policy by default, #412 – a module page IS one), Derive (a factory module, #350: read-only), New module, and
	// the tab row – Parameters · Nodes · Rust · Assignment (a rig page: the character's rig as a form, #283), all mounted while hidden.
	import { DesignerTabs, NodesTab, ParametersTab, RustTab, type DesignerTabId, type GraphWiring, type RigDesigner } from '@scutoons/rig-designer';
	import type { EngineHost } from '@scutoons/engine-host';
	import { Button, IconButton, Select } from '@scutoons/ui/editor';
	import { VIEWER_URL } from '$lib/config.ts';
	import AssignmentPane from '$lib/components/rigs/assignment-pane.svelte';
	import PolicyBar from '$lib/components/rigs/policy-bar.svelte';
	import type { RigSession } from '$lib/editor/rig-session.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	let { designer, fixed = null, session = null, modules, factory = [], selected, notice = '', host, wiring, onselect, onnew, onderive }: {
		/** the deck half's engine and the core's wiring of what it plays (the Nodes tab) */
		host: EngineHost | null; wiring?: GraphWiring;
		designer: RigDesigner;
		/** a module page: the module's id, shown as the title (no picker); a rig page: the rig open (Assignment, the policy) */
		fixed?: string | null; session?: RigSession | null;
		/** the stored modules a rig page can pick; `factory`: which of them are factory modules (#350, marked) */
		modules: string[];
		factory?: string[];
		selected: string | null;
		notice?: string; // a refused New module
		onselect: (id: string | null) => void;
		onnew: () => void;
		/** Derive: a module of one's own from the open factory module */
		onderive: () => void;
	} = $props();

	let tab = $state<DesignerTabId>('rust');
</script>

<div class="designer" data-testid="sound-designer">
	<header class="head">
		{#if fixed}
			<span class="title" data-testid="sound-module-title">module:{fixed}</span>
		{:else}
			<span class="pick"><Select value={selected ?? ''} options={[...(session?.policy ? [] : [{ value: '', label: m['creator.sound_module_none']() }]), ...modules.map((id) => ({ value: id, label: `module:${id}${factory.includes(id) ? ` · ${m['creator.sound_factory']()}` : ''}` }))]} label={m['creator.sound_module']()} onchange={(v) => onselect(v || null)} data-testid="sound-module" /></span>
		{/if}
		<!-- TODO(#859): the deck opens on this module; until then a plain link to the deck -->
		<Button variant="quiet" icon="external-link" href="{VIEWER_URL}/live/deck" target="_blank" rel="noopener" title={m['creator.sound_open_in_deck_title']()} data-testid="sound-open-deck">{m['creator.sound_open_in_deck']()}</Button>
		{#if designer.factory && designer.moduleId}
			<Button onclick={onderive} title={m['creator.sound_derive_title']({ id: designer.moduleId })} data-testid="sound-derive">{m['creator.sound_derive']()}</Button>
		{/if}
		<IconButton alone icon="plus" label={m['creator.sound_new_module']()} onclick={onnew} data-testid="sound-new-module" />
	</header>
	{#if notice}<p class="note" role="status">{notice}</p>{/if}
	<DesignerTabs value={tab} onchange={(id) => (tab = id)} />
	<div class="body">
		{#if session && tab === 'rust'}<PolicyBar {session} open={designer.moduleId} {onselect} />{/if}
		{#if session}<div class="pane" hidden={tab !== 'assignment'}><AssignmentPane {session} {host} moods={host?.moods ?? []} /></div>{/if}
		<div class="pane" hidden={tab !== 'nodes'}><NodesTab {wiring} read={(p) => host?.deck?.read(p) ?? NaN} active={tab === 'nodes'} /></div>
		{#if !designer.moduleId}
			{#if tab !== 'nodes' && tab !== 'assignment'}<p class="sc-empty pad">{m['creator.sound_pick_module']()}</p>{/if}
		{:else}
			<div class="pane" hidden={tab !== 'rust'}><RustTab {designer} /></div>
			<div class="pane" hidden={tab !== 'parameters'}><ParametersTab {designer} /></div>
		{/if}
	</div>
</div>

<style>
	.designer {
		display: flex;
		flex-direction: column;
		min-width: 0;
		min-height: 0;
		height: 100%;
	}
	.head {
		flex: none;
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-05) var(--space-1);
	}
	.pick,
	.title {
		flex: 1;
		min-width: 0;
	}
	.title {
		overflow: hidden;
		white-space: nowrap;
		text-overflow: ellipsis;
		font-size: var(--type-title-size);
	}
	.body {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		border-top: 1px solid var(--tool-line);
	}
	.pane {
		flex: 1;
		min-height: 0;
	}
	.pane[hidden] {
		display: none;
	}
	.note {
		flex: none;
		margin: 0;
		padding: var(--space-025) var(--space-1);
		color: var(--tool-muted);
	}
	.pad {
		padding: var(--space-1);
	}
</style>
