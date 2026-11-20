<script lang="ts">
	import { folderTree, originCounts, scopeCounts, todoCounts, type FolderNode, type LibraryOrigin, type LibraryScope, type LibraryTodo } from '@scutoons/media';
	import { Section } from '@scutoons/ui/editor';
	import { library } from '$lib/editor/library.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The browser's places (research/redesign/library.html ?w=browser): the scope, the to-do lists (versions missing,
	 * unused), the folders of the toon (or of everything) as a tree, and where the files came from.
	 */
	const ctx = $derived(library.context);
	const counts = $derived(scopeCounts(library.families, ctx, library.toon));
	const todo = $derived(todoCounts(library.families));
	const origins = $derived(originCounts(library.families));
	const tree = $derived(folderTree(library.families, library.toon ?? ''));
	let open = $state<Record<string, boolean>>({});

	const scopes = $derived<{ id: LibraryScope; label: string; count: number }[]>([
		...(ctx && ctx.kind !== 'toon' ? [{ id: 'context' as const, label: m['library.place_this']({ what: m[`library.scope_${ctx.kind}`]() }), count: counts.context }] : []),
		...(library.toon ? [{ id: 'toon' as const, label: m['library.place_toon']({ toon: library.toon }), count: counts.toon }] : []),
		{ id: 'all', label: m['library.place_all'](), count: counts.all },
		{ id: 'unused', label: m['library.scope_unused'](), count: counts.unused }
	]);

	function scope(id: LibraryScope) {
		library.scope = id;
		library.todo = null;
		library.folder = null;
		library.origin = null;
	}
	function toggle<T>(current: T | null, value: T): T | null {
		return current === value ? null : value;
	}
	const setTodo = (t: LibraryTodo) => (library.todo = toggle(library.todo, t));
	const setOrigin = (o: LibraryOrigin) => (library.origin = toggle(library.origin, o));
	function setFolder(node: FolderNode) {
		library.folder = toggle(library.folder, node.path);
		if (library.folder) library.scope = 'all';
	}
</script>

{#snippet folders(nodes: FolderNode[], depth: number)}
	{#each nodes as node (node.path)}
		<li>
			<div class="place" class:on={library.folder === node.path} style:--depth={depth}>
				{#if node.children.length}
					<button type="button" class="fold" data-control-exempt="tree fold" aria-expanded={!!open[node.path]} aria-label={node.name} onclick={() => (open[node.path] = !open[node.path])}>{open[node.path] ? '▾' : '▸'}</button>
				{:else}<span class="fold"></span>{/if}
				<button type="button" class="label" data-control-exempt="tree row" onclick={() => setFolder(node)}>{node.name}</button>
				<span class="n">{node.count}</span>
			</div>
			{#if node.children.length && open[node.path]}<ul>{@render folders(node.children, depth + 1)}</ul>{/if}
		</li>
	{/each}
{/snippet}

<nav class="places" aria-label={m['library.places']()} data-testid="library-places">
	<h3>{m['library.scope']()}</h3>
	<ul>
		{#each scopes as s (s.id)}
			<li><button type="button" class="place row" data-control-exempt="tree row" class:on={library.scope === s.id && !library.folder} onclick={() => scope(s.id)}>{s.label}<span class="n">{s.count}</span></button></li>
		{/each}
	</ul>
	<h3>{m['library.todo']()}</h3>
	<ul>
		<li><button type="button" class="place row" data-control-exempt="tree row" class:on={library.todo === 'unused'} onclick={() => setTodo('unused')}>{m['library.todo_unused']()}<span class="n">{todo.unused}</span></button></li>
	</ul>
	<h3>{library.toon ? m['library.tree_toon']({ toon: library.toon }) : m['library.tree']()}</h3>
	<ul>{@render folders(tree, 0)}</ul>
	<h3>{m['library.from']()}</h3>
	<ul>
		{#each ['uploaded', 'generated', 'imported'] as const as o (o)}
			<li><button type="button" class="place row" data-control-exempt="tree row" class:on={library.origin === o} onclick={() => setOrigin(o)}>{m[`library.from_${o}`]()}<span class="n">{origins[o]}</span></button></li>
		{/each}
	</ul>
	<Section id="library-places-missing" label={m['library.todo_missing']()} open={false} summary={String(todo.missing)}>
		<ul>
			<li><button type="button" class="place row" data-control-exempt="tree row" class:on={library.todo === 'missing'} onclick={() => setTodo('missing')}>{m['library.show_missing']()}<span class="n">{todo.missing}</span></button></li>
		</ul>
	</Section>
</nav>

<style>
	.places {
		overflow: auto;
		padding: 0.5rem 0;
		border-right: var(--editor-edge) solid var(--editor-line);
		font-size: var(--editor-small);
	}
	h3 {
		margin: 0.7rem var(--editor-pad) 0.2rem;
		color: var(--editor-muted);
		font-size: var(--editor-label);
		font-weight: 400;
		letter-spacing: var(--editor-label-spacing, 0.08em);
		text-transform: uppercase;
	}
	ul {
		margin: 0;
		padding: 0;
		list-style: none;
	}
	.place {
		box-sizing: border-box;
		display: flex;
		align-items: center;
		gap: 0.3rem;
		width: 100%;
		min-height: var(--editor-tree-row);
		padding: 0 var(--editor-pad) 0 calc(var(--editor-pad) + var(--depth, 0) * 1rem);
		border: 0;
		background: none;
		color: var(--editor-text);
		font: inherit;
		text-align: left;
		justify-content: flex-start;
		cursor: pointer;
	}
	.place.on {
		background: var(--editor-selected);
	}
	.place:hover {
		background: var(--editor-hover);
	}
	.label {
		display: block;
		flex: 1 1 0;
		min-width: 0;
		padding: 0;
		border: 0;
		background: none;
		color: inherit;
		font: inherit;
		text-align: left;
		cursor: pointer;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.fold {
		flex: none;
		width: 0.9rem;
		padding: 0;
		border: 0;
		background: none;
		color: var(--editor-muted);
		font-size: var(--editor-label);
		cursor: pointer;
	}
	.n {
		margin-left: auto;
		padding-left: 0.4rem;
		color: var(--editor-muted);
		font-family: var(--editor-mono, ui-monospace, monospace);
		font-size: var(--editor-label);
	}
</style>
