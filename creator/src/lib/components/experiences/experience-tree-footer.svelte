<script lang="ts">
	import { clickOutside } from '@scutoons/ui';
	import { Button, IconButton, MenuHead, MenuItem, MenuPanel, Pill } from '@scutoons/ui/editor';
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
			<MenuPanel width="min(18rem, 100%)" role="group">
				{#if picking}
					<MenuItem icon="chevron-left" onclick={() => (picking = null)}>{m['creator.experiences_add_file_pick']({ type: picking })}</MenuItem>
					{#each choices as f (f.src)}
						<MenuItem title={f.src} onclick={() => pick(f)} data-testid="tree-add-file">{fileName(f.src)}<span class="detail">{f.assetType}</span></MenuItem>
					{:else}
						<p class="kit-menu-note">{m['creator.experiences_add_file_none']()}</p>
					{/each}
				{:else}
					<!-- svelte-ignore a11y_autofocus -->
					<input class="filter" type="search" bind:value={query} placeholder={m['creator.experiences_add_filter']()} aria-label={m['creator.experiences_add_filter']()} autofocus data-testid="tree-add-filter" />
					{#each groups as g (g.group)}
						<MenuHead label={GROUPS[g.group]?.() ?? g.group} />
						{#each g.items as i (i.key)}
							<MenuItem disabled={!!i.disabled} title={why(i) || detail(i)} onclick={() => choose(i)} data-testid="tree-add-{i.key}">{label(i)}<span class="detail">{why(i) || detail(i)}</span></MenuItem>
						{/each}
					{/each}
				{/if}
			</MenuPanel>
		</div>
	{/if}
	<Pill>
		<Button variant="quiet" icon="plus" aria-haspopup="menu" aria-expanded={open} onclick={toggle} title={m['creator.experiences_add_about']()} data-testid="tree-add">{m['creator.experiences_add']()}</Button>
	</Pill>
	<span class="spacer"></span>
	<Pill label={m['creator.experiences_tree_actions']()}>
		<IconButton icon="copy" disabled={!part} onclick={() => part && onpart(part, 'duplicate')} label={m['creator.experiences_part_duplicate']()} data-testid="tree-duplicate" />
		<IconButton icon="layers" disabled={!part} onclick={() => part && onpart(part, 'group')} label={m['creator.experiences_part_group']()} data-testid="tree-group" />
		<IconButton
			icon="trash-2"
			disabled={!removable}
			onclick={() => selected && onpart(selected, 'remove')}
			label={part ? m['creator.experiences_part_remove']() : m['creator.experiences_content_remove']()}
			title={`${part ? m['creator.experiences_part_remove']() : m['creator.experiences_content_remove']()} (Delete)`}
			data-testid="tree-remove"
		/>
	</Pill>
</div>

<style>
	/* at the panel's foot however far the tree scrolls (the panel's body is the scroller); on the stage, pills only */
	.footer {
		position: sticky;
		bottom: 0;
		z-index: 1;
		margin-top: auto;
		display: flex;
		align-items: center;
		gap: var(--kit-gap);
		padding: 0.25rem 0;
	}
	.spacer {
		flex: 1;
	}
	.add-menu {
		position: absolute;
		left: 0;
		bottom: calc(100% + 0.25rem);
		width: min(18rem, 100%);
		max-height: 24rem;
		display: flex;
	}
	.add-menu :global(.kit-menu-panel) {
		max-height: 24rem;
	}
	.filter {
		margin: 0 0 0.25rem;
	}
	.detail {
		margin-left: auto;
		padding-left: 0.5rem;
		color: var(--tool-muted);
		font-size: var(--editor-label);
	}
</style>
