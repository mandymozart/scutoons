<script lang="ts">
	// Sound design's main view, the Rust tab (#282): the module being edited (a rig page picks one of the stored modules, its
	// policy by default, #412 – a module page IS one), Open in the deck, Derive (a factory module, #350: read-only), New
	// module; a rig's policy line; the code. On the stage it keeps clear of the bar and of the panels floating over it.
	import { RustTab, type RigDesigner } from '@scutoons/rig-designer';
	import { Button, IconButton, Select } from '@scutoons/ui/editor';
	import OpenInDeck from '$lib/components/shell/open-in-deck.svelte';
	import PolicyBar from '$lib/components/rigs/policy-bar.svelte';
	import type { RigSession } from '$lib/editor/rig-session.svelte.ts';
	import { shell } from '$lib/shell/shell.svelte.ts';
	import { roomAround } from '$lib/shell/stage.ts';
	import { m } from '$lib/paraglide/messages.js';

	let { designer, fixed = null, session = null, modules, factory = [], selected, notice = '', onselect, onnew, onderive }: {
		designer: RigDesigner;
		/** a module page: the module's id, shown as the title (no picker); a rig page: the rig open (the policy) */
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

	/** the whole stage: the room the shown panels take from it (rem), else none */
	const room = $derived.by(() => {
		const f = shell.floats;
		if (!f || f.stateOf('rust') !== 'stage' || f.stage.layout !== 'one') return null;
		const r = roomAround(
			Object.entries(f.views)
				.filter(([id]) => !f.closed(id) && f.shown.has(id))
				.map(([, v]) => v.rect),
			f.area
		);
		const rem = f.metrics.rem || 16;
		return { left: `${r.left / rem}rem`, right: `${r.right / rem}rem`, bottom: `${r.bottom / rem}rem` };
	});
</script>

<div class="module" class:staged={!!room} style:--room-left={room?.left} style:--room-right={room?.right} style:--room-bottom={room?.bottom} data-testid="sound-designer">
	<header class="head">
		{#if fixed}
			<span class="title" data-testid="sound-module-title">module:{fixed}</span>
		{:else}
			<span class="pick"><Select value={selected ?? ''} options={[...(session?.policy ? [] : [{ value: '', label: m['creator.sound_module_none']() }]), ...modules.map((id) => ({ value: id, label: `module:${id}${factory.includes(id) ? ` · ${m['creator.sound_factory']()}` : ''}` }))]} label={m['creator.sound_module']()} onchange={(v) => onselect(v || null)} data-testid="sound-module" /></span>
		{/if}
		{#if designer.moduleId || session}<OpenInDeck address={designer.moduleId ? `module:${designer.moduleId}` : `rig:${session?.subject}`} testid="sound-open-deck" />{/if}
		{#if designer.factory && designer.moduleId}
			<Button onclick={onderive} title={m['creator.sound_derive_title']({ id: designer.moduleId })} data-testid="sound-derive">{m['creator.sound_derive']()}</Button>
		{/if}
		<IconButton alone icon="plus" label={m['creator.sound_new_module']()} onclick={onnew} data-testid="sound-new-module" />
	</header>
	{#if notice}<p class="note" role="status">{notice}</p>{/if}
	<div class="body">
		{#if session}<PolicyBar {session} open={designer.moduleId} {onselect} />{/if}
		{#if designer.moduleId}
			<div class="code"><RustTab {designer} /></div>
		{:else}
			<p class="sc-empty pad">{m['creator.sound_pick_module']()}</p>
		{/if}
	</div>
</div>

<style>
	.module {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-width: 0;
		min-height: 0;
		height: 100%;
		box-sizing: border-box;
	}
	/* on the whole stage: under the bar and the cell's name, beside and above the panels over it */
	.module.staged {
		padding: calc(var(--stage-top, 0rem) + var(--control-height) + 0.75rem) calc(var(--room-right) + 0.5rem) calc(var(--room-bottom) + var(--status-room, 0rem) + 0.5rem) calc(var(--room-left) + 0.5rem);
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
	.code {
		flex: 1;
		min-height: 0;
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
