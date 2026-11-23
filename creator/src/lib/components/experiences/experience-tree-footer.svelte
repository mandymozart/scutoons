<script lang="ts">
	import { Icon, clickOutside } from '@scutoons/ui';
	import { xrFilterTreeAdd, xrNodeFileTypes, xrNodeTypes, xrPluginNodeTypes, xrTreeAddItems, type XrTreeAdd, type XrTreeAddItem } from '@scutoons/xr-engine/tree';
	import { entitySlot, fileName, nodeAtPath, nodeName, type NodeRef, type PartAction, type XrFile } from '$lib/editor/xr-nodes.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The Tree's footer (OSCT 4.10.1, #339 – as in Photoshop's layers panel): it stays at the panel's foot however far the
	 * tree scrolls. **+** opens one menu of everything the scene can get (OSCT's tree-add.ts): a surface, a target or an
	 * entry, a part of every type (the built-in ones, then the plugins'); a part needs a target to hang on, a type that
	 * takes a file asks for one of the experience's files next. For the selected row: duplicate, group, remove.
	 */
	let {
		configuration,
		selected,
		files,
		onadd,
		onpart
	}: {
		configuration: unknown;
		selected: NodeRef | null;
		files: XrFile[];
		onadd: (add: XrTreeAdd, file: XrFile | null) => void;
		onpart: (ref: NodeRef, action: PartAction) => void;
	} = $props();

	let open = $state(false);
	let query = $state('');
	/** a part's type that takes a file: the menu's second step lists the files */
	let picking = $state<string | null>(null);

	const slot = $derived(selected ? entitySlot(configuration, selected.target) : null);
	const node = $derived(slot?.tree && selected?.path ? nodeAtPath(slot.tree, selected.path) : null);
	const part = $derived(selected && selected.path !== null && node ? selected : null);
	const removable = $derived(!!part || (!!selected && !!slot?.tree));

	/** the menu's items for what is selected now (the plugins' types are known once the stage has loaded them) */
	function items(): XrTreeAddItem[] {
		const plugins = xrPluginNodeTypes();
		return xrTreeAddItems({
			target: selected?.target ?? null,
			hasEntity: !!slot?.tree,
			node: node ? nodeName(node) : null,
			builtIn: xrNodeTypes().filter((t) => !plugins.includes(t)),
			plugins,
			takesFile: (t) => !!xrNodeFileTypes(t),
			canAddTarget: false,
			canAddEntry: false
		});
	}
	let all = $state<XrTreeAddItem[]>([]);
	const shown = $derived(xrFilterTreeAdd(all, query));
	const groups = $derived([...new Set(shown.map((i) => i.group))].map((g) => ({ group: g, items: shown.filter((i) => i.group === g) })));
	const choices = $derived(picking ? files.filter((f) => (xrNodeFileTypes(picking!) ?? []).includes(f.assetType)) : []);

	const GROUPS: Record<string, () => string> = {
		scene: () => m['creator.experiences_add_group_scene'](),
		nodes: () => m['creator.experiences_add_group_parts'](),
		'plugin nodes': () => m['creator.experiences_add_group_plugins']()
	};
	const LABELS: Record<string, () => string> = {
		'surface:double': () => m['creator.experiences_add_surface_double'](),
		'surface:single': () => m['creator.experiences_add_surface_single'](),
		'surface:wall': () => m['creator.experiences_add_surface_wall'](),
		'surface:custom': () => m['creator.experiences_add_surface_custom'](),
		target: () => m['creator.experiences_add_target'](),
		entry: () => m['creator.experiences_add_entry']()
	};
	const label = (i: XrTreeAddItem) => LABELS[i.key]?.() ?? i.label;
	/** a surface's size from OSCT's words (numbers read the same in every language); a part: where it goes */
	const detail = (i: XrTreeAddItem) =>
		i.add.add === 'surface'
			? (/\d+ × \d+ mm/.exec(i.detail)?.[0] ?? '')
			: i.add.add === 'node'
				? xrNodeFileTypes(i.add.type)
					? m['creator.experiences_add_takes_file']()
					: ''
				: '';
	const why = (i: XrTreeAddItem) => (!i.disabled ? '' : i.add.add === 'node' ? m['creator.experiences_add_needs_target']() : m['creator.experiences_add_not_yet']());

	function toggle() {
		open = !open;
		query = '';
		picking = null;
		if (open) all = items();
	}
	function close() {
		open = false;
		picking = null;
	}
	function choose(i: XrTreeAddItem) {
		if (i.disabled) return;
		if (i.add.add === 'node' && xrNodeFileTypes(i.add.type)) {
			picking = i.add.type;
			return;
		}
		onadd(i.add, null);
		close();
	}
	function pick(file: XrFile) {
		if (!picking) return;
		onadd({ add: 'node', type: picking }, file);
		close();
	}
	function onkeydown(event: KeyboardEvent) {
		if (event.key === 'Escape') {
			event.stopPropagation();
			close();
		} else if (event.key === 'Enter' && !picking) {
			const first = shown.find((i) => !i.disabled);
			if (first) {
				event.preventDefault();
				choose(first);
			}
		}
	}
</script>

<div class="footer" role="toolbar" aria-label={m['creator.experiences_tree_actions']()} data-testid="tree-footer" {@attach clickOutside(close)}>
	{#if open}
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div class="add-menu" role="menu" tabindex="-1" {onkeydown} data-testid="tree-add-menu">
			{#if picking}
				<button type="button" class="back" onclick={() => (picking = null)}><Icon name="chevron-left" size="0.9em" />{m['creator.experiences_add_file_pick']({ type: picking })}</button>
				{#each choices as f (f.src)}
					<button type="button" class="item" role="menuitem" title={f.src} onclick={() => pick(f)} data-testid="tree-add-file">{fileName(f.src)}<span class="detail">{f.assetType}</span></button>
				{:else}
					<p class="none">{m['creator.experiences_add_file_none']()}</p>
				{/each}
			{:else}
				<!-- svelte-ignore a11y_autofocus -->
				<input class="filter" type="search" bind:value={query} placeholder={m['creator.experiences_add_filter']()} aria-label={m['creator.experiences_add_filter']()} autofocus data-testid="tree-add-filter" />
				{#each groups as g (g.group)}
					<span class="head">{GROUPS[g.group]?.() ?? g.group}</span>
					{#each g.items as i (i.key)}
						<button type="button" class="item" role="menuitem" disabled={!!i.disabled} title={why(i) || detail(i)} onclick={() => choose(i)} data-testid="tree-add-{i.key}"
							>{label(i)}<span class="detail">{why(i) || detail(i)}</span></button
						>
					{/each}
				{/each}
			{/if}
		</div>
	{/if}
	<button type="button" class="add" aria-haspopup="menu" aria-expanded={open} onclick={toggle} title={m['creator.experiences_add_about']()} data-testid="tree-add"><Icon name="plus" size="1em" />{m['creator.experiences_add']()}</button>
	<span class="actions">
		<button type="button" class="icon" disabled={!part} onclick={() => part && onpart(part, 'duplicate')} aria-label={m['creator.experiences_part_duplicate']()} title={m['creator.experiences_part_duplicate']()} data-testid="tree-duplicate"><Icon name="copy" size="1em" /></button>
		<button type="button" class="icon" disabled={!part} onclick={() => part && onpart(part, 'group')} aria-label={m['creator.experiences_part_group']()} title={m['creator.experiences_part_group']()} data-testid="tree-group"><Icon name="layers" size="1em" /></button>
		<button
			type="button"
			class="icon"
			disabled={!removable}
			onclick={() => selected && onpart(selected, 'remove')}
			aria-label={part ? m['creator.experiences_part_remove']() : m['creator.experiences_content_remove']()}
			title={`${part ? m['creator.experiences_part_remove']() : m['creator.experiences_content_remove']()} (Delete)`}
			data-testid="tree-remove"><Icon name="trash-2" size="1em" /></button
		>
	</span>
</div>

<style>
	/* at the panel's foot however far the tree scrolls (the panel's body is the scroller) */
	.footer {
		position: sticky;
		bottom: 0;
		z-index: 1;
		margin-top: auto;
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.25rem 0;
		border-top: var(--editor-edge, 1px) solid var(--editor-line);
		background: var(--editor-surface);
	}
	.add,
	.icon {
		margin: 0;
		min-height: 0;
		height: var(--control-height, 1.75rem);
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
	}
	.icon {
		width: var(--control-height, 1.75rem);
		justify-content: center;
		padding: 0;
	}
	.actions {
		margin-left: auto;
		display: flex;
		gap: 0.125rem;
	}
	.add-menu {
		position: absolute;
		left: 0;
		bottom: calc(100% + 0.25rem);
		width: min(18rem, 100%);
		max-height: 24rem;
		overflow-y: auto;
		display: flex;
		flex-direction: column;
		padding: 0.25rem;
		background: var(--editor-surface);
		color: var(--editor-text);
		border: var(--editor-edge, 1px) solid var(--editor-line);
		border-radius: var(--editor-radius, 0);
		text-shadow: none;
	}
	.filter {
		margin: 0 0 0.25rem;
	}
	.head {
		padding: 0.4rem 0.4rem 0.15rem;
		color: var(--editor-muted, var(--text-muted));
		font-size: var(--editor-label, 0.85em);
		text-transform: uppercase;
		letter-spacing: 0.04em;
	}
	.item,
	.back {
		display: flex;
		align-items: baseline;
		gap: 0.5rem;
		margin: 0;
		min-height: 0;
		padding: 0.3rem 0.4rem;
		border: 0;
		background: none;
		color: inherit;
		text-align: left;
		cursor: pointer;
	}
	.item:hover:not(:disabled),
	.item:focus-visible {
		background: color-mix(in srgb, var(--editor-accent, #ffa028) 22%, transparent);
	}
	.item:disabled {
		opacity: 0.45;
		cursor: default;
	}
	.detail {
		margin-left: auto;
		color: var(--editor-muted, var(--text-muted));
		font-size: var(--editor-label, 0.85em);
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.none {
		margin: 0.4rem;
		color: var(--editor-muted, var(--text-muted));
	}
</style>
