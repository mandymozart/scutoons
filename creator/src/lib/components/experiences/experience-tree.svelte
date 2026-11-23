<script lang="ts">
	import { SvelteSet } from 'svelte/reactivity';
	import { Icon, Menu } from '@scutoons/ui';
	import type { XrTreeAdd } from '@scutoons/xr-engine/tree';
	import { sameRef, sceneRows, type NodeRef, type PartAction, type TreeRow, type XrFile } from '$lib/editor/xr-nodes.ts';
	import ExperienceTreeFooter from '$lib/components/experiences/experience-tree-footer.svelte';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Tree tab: the open scene as an outliner (after OSCT's editor, panels/TreePanel.svelte) – its surfaces, each
	 * entry, the entry's target and the target's entity as its node tree. A tap on a target or a node row selects it (the
	 * stage outlines it, the Inspector shows it); the chevron folds a node; ↑ ↓ walk the selectable rows. A part's ⋯ menu
	 * and the keys move it (Alt ↑ ↓ among its siblings, Tab into the part above, Shift Tab out of its parent); Delete
	 * removes it. The footer (OSCT's, as in Photoshop's layers panel) adds, duplicates, groups and removes.
	 */
	let {
		configuration,
		scene,
		selected,
		onselect,
		onpart,
		onadd,
		files
	}: {
		configuration: unknown;
		scene: string;
		selected: NodeRef | null;
		onselect: (ref: NodeRef | null) => void;
		onpart: (ref: NodeRef, action: PartAction) => void;
		onadd: (add: XrTreeAdd, file: XrFile | null) => void;
		files: XrFile[];
	} = $props();

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

	/** the part's moves as the menu offers them: what cannot be done now is left out */
	function moves(r: Extract<TreeRow, { kind: 'node' }>): { action: PartAction; label: string; keys: string }[] {
		const path = r.ref.path ?? [];
		if (!path.length) return [];
		const last = path[path.length - 1];
		const siblings = rows.filter((x) => x.kind === 'node' && x.ref.target === r.ref.target && x.ref.path?.length === path.length && x.ref.path.slice(0, -1).join('.') === path.slice(0, -1).join('.')).length;
		return [
			...(last > 0 ? [{ action: 'up' as const, label: m['creator.experiences_part_up'](), keys: 'Alt ↑' }] : []),
			...(last < siblings - 1 ? [{ action: 'down' as const, label: m['creator.experiences_part_down'](), keys: 'Alt ↓' }] : []),
			...(last > 0 ? [{ action: 'indent' as const, label: m['creator.experiences_part_indent'](), keys: 'Tab' }] : []),
			{ action: 'outdent', label: m['creator.experiences_part_outdent'](), keys: '⇧ Tab' }
		];
	}

	const KEYS: Record<string, PartAction> = { Delete: 'remove', Backspace: 'remove' };

	function onkeydown(event: KeyboardEvent) {
		const part = selected && selected.path !== null ? selected : null;
		const action: PartAction | undefined =
			part && event.altKey && event.key === 'ArrowUp' ? 'up'
			: part && event.altKey && event.key === 'ArrowDown' ? 'down'
			: part && event.key === 'Tab' && part.path!.length ? (event.shiftKey ? 'outdent' : 'indent')
			: part ? KEYS[event.key]
			: undefined;
		if (part && action) {
			event.preventDefault();
			onpart(part, action);
			return;
		}
		if (event.key !== 'ArrowDown' && event.key !== 'ArrowUp') return;
		event.preventDefault();
		const at = refs.findIndex((r) => sameRef(r, selected));
		const next = refs[Math.min(refs.length - 1, Math.max(0, at + (event.key === 'ArrowDown' ? 1 : -1)))];
		if (next) onselect(next);
	}
</script>

<div class="tree-tab">
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
			{#if r.kind === 'node' && moves(r).length}
				<span class="more">
					<Menu label={m['creator.experiences_part_menu']({ name: r.label })} align="end">
						{#snippet trigger()}<Icon name="more-horizontal" size="0.95em" />{/snippet}
						{#each moves(r) as move (move.action)}
							<button type="button" onclick={() => onpart(r.ref, move.action)} data-testid="part-{move.action}">{move.label}<span class="keys">{move.keys}</span></button>
						{/each}
					</Menu>
				</span>
			{/if}
		</div>
	{/each}
</div>
<ExperienceTreeFooter {configuration} {selected} {files} {onadd} {onpart} />
</div>

<style>
	.tree-tab {
		display: flex;
		flex-direction: column;
		min-height: 100%;
		min-width: 0;
	}
	.tree {
		display: flex;
		flex-direction: column;
		outline: none;
		min-width: 0;
	}
	/* the kit's tree row (ListRow dense): the hover ground under the pointer, selected = the pressed fill (no hue) */
	.row {
		display: flex;
		align-items: center;
		min-height: var(--editor-tree-row);
		padding-left: calc(var(--depth) * 0.85rem);
		border-radius: var(--editor-radius);
		background: var(--kit-list-row-rest-ground);
		color: var(--kit-list-row-rest-text);
	}
	@media (hover: hover) {
		.row:hover:not(.on) {
			background: var(--kit-list-row-hover-ground);
		}
	}
	.row.on {
		background: var(--kit-list-row-selected-ground);
		color: var(--kit-list-row-selected-text);
		text-shadow: none;
	}
	.row.on .hint {
		color: inherit;
	}
	.row.plain {
		color: var(--tool-muted);
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
	.more {
		flex: none;
		opacity: 0;
	}
	.row:hover .more,
	.row.on .more,
	.more:focus-within {
		opacity: 1;
	}
	.more :global(.keys) {
		margin-left: auto;
		padding-left: 1rem;
		color: var(--tool-muted);
		font-size: var(--editor-label);
	}
	.hint {
		margin-left: auto;
		padding-right: 0.4rem;
		color: var(--tool-muted);
		font-size: var(--editor-label);
		white-space: nowrap;
	}
</style>
