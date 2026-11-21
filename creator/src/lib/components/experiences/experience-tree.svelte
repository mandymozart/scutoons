<script lang="ts">
	import { SvelteSet } from 'svelte/reactivity';
	import { Icon } from '@scutoons/ui';
	import { sameRef, sceneRows, type NodeRef, type TreeRow } from '$lib/editor/xr-nodes.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Tree tab: the open scene as an outliner (after OSCT's editor, panels/TreePanel.svelte) – its surfaces, each
	 * entry, the entry's target and the target's entity as its node tree. A tap on a target or a node row selects it (the
	 * stage outlines it, the Inspector shows it); the chevron folds a node; ↑ ↓ walk the selectable rows.
	 */
	let { configuration, scene, selected, onselect }: { configuration: unknown; scene: string; selected: NodeRef | null; onselect: (ref: NodeRef | null) => void } = $props();

	const folded = new SvelteSet<string>();
	const rows = $derived(sceneRows(configuration, scene, folded));
	const refs = $derived(rows.flatMap((r) => (r.kind === 'target' || r.kind === 'node' ? [r.ref] : [])));

	const ICONS: Record<string, string> = { group: 'layers', model: 'box', light: 'sun', text: 'type', image: 'image', video: 'film', audio: 'music' };
	const icon = (r: TreeRow) => (r.kind === 'surface' ? 'square' : r.kind === 'entry' ? 'file-text' : r.kind === 'target' ? 'crosshair' : (ICONS[r.type] ?? 'circle'));
	const foldKey = (ref: NodeRef) => `${ref.target}:${(ref.path ?? []).join('.')}`;

	function fold(ref: NodeRef) {
		const key = foldKey(ref);
		if (folded.has(key)) folded.delete(key);
		else folded.add(key);
	}

	function onkeydown(event: KeyboardEvent) {
		if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
		event.preventDefault();
		const at = refs.findIndex((r) => sameRef(r, selected));
		const next = refs[Math.min(refs.length - 1, Math.max(0, at + (event.key === 'ArrowDown' ? 1 : -1)))];
		if (next) onselect(next);
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
<div class="tree" role="tree" aria-label={m['shell.panel_tree']()} tabindex="0" {onkeydown} data-testid="experience-tree">
	{#if !rows.length}<p class="sc-empty">{m['creator.experiences_scenes_none']()}</p>{/if}
	{#each rows as r, i (i)}
		{@const ref = r.kind === 'target' || r.kind === 'node' ? r.ref : null}
		<div class="row" class:on={!!ref && sameRef(ref, selected)} class:plain={!ref} style:--depth={r.depth} role="treeitem" aria-selected={!!ref && sameRef(ref, selected)} data-testid="tree-row">
			<span class="chevron">
				{#if r.kind === 'node' && r.children}
					<button type="button" class="fold" onclick={() => fold(r.ref)} aria-label={m['creator.fold_panel']()}><Icon name={folded.has(foldKey(r.ref)) ? 'chevron-right' : 'chevron-down'} size="0.9em" /></button>
				{/if}
			</span>
			<button type="button" class="pick" disabled={!ref} onclick={() => ref && onselect(ref)}>
				<Icon name={icon(r)} size="0.95em" />
				<span class="name">{r.label}</span>
				{#if r.kind === 'target' && r.shared}<span class="hint">{m['creator.experiences_shows']({ entity: r.shared })}</span>{/if}
				{#if r.kind === 'node'}<span class="hint">{r.type}</span>{/if}
			</button>
		</div>
	{/each}
</div>

<style>
	.tree {
		display: flex;
		flex-direction: column;
		outline: none;
		min-width: 0;
	}
	.row {
		display: flex;
		align-items: center;
		min-height: 1.75rem;
		padding-left: calc(var(--depth) * 0.85rem);
		border-radius: var(--editor-radius, 0);
	}
	.row:nth-child(even) {
		background: color-mix(in srgb, var(--editor-text, currentColor) 3%, transparent);
	}
	.row.on {
		background: color-mix(in srgb, var(--editor-accent, #ffa028) 28%, transparent);
	}
	.row.plain {
		color: var(--editor-muted, var(--text-muted));
	}
	.chevron {
		flex: none;
		width: 1.25rem;
		display: flex;
		justify-content: center;
	}
	.fold,
	.pick {
		margin: 0;
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		min-height: 0;
	}
	.pick {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 0.4rem;
		text-align: left;
		cursor: pointer;
	}
	.pick:disabled {
		cursor: default;
		opacity: 1;
	}
	.name {
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.hint {
		margin-left: auto;
		padding-right: 0.4rem;
		color: var(--editor-muted, var(--text-muted));
		font-size: var(--editor-label, 0.85em);
		white-space: nowrap;
	}
</style>
