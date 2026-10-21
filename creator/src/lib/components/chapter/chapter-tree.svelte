<script lang="ts">
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { CloseIcon, Icon, SortGrip, region, sortable } from '@scutoons/ui';
	import { untrack } from 'svelte';
	import { linksToNext, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { ChapterView } from '$lib/editor/chapter-state.svelte.ts';
	import { clickSelect, moveMany } from '$lib/editor/multi-select.ts';
	import { useClipHandler } from '$lib/editor/clipboard.svelte.ts';
	import { groupsClipHandler } from '$lib/editor/groups-clip.svelte.ts';
	import { newGroup } from '$lib/editor/defaults.ts';
	import SliceList from '$lib/components/chapter/slice-list.svelte';
	import ReachChip, { type ReachMark } from '$lib/components/chapter/reach-chip.svelte';
	import Marks from '$lib/components/chapter/marks.svelte';
	import { groupMarks } from '$lib/editor/slice-marks.ts';

	/**
	 * The chapter's children in the sidebar (COMPONENTS.md step 12): meta, audio, groups → the open group's slices
	 * (Alt+ArrowDown opens the first child). Groups: click opens / folds one, Shift+click picks the range from the anchor,
	 * Ctrl / Cmd+click adds / removes one; dragging a picked group by its grip (the shared Sortable) moves all picked
	 * ones (in their order) – owner 2026-10-03. One "Add group" (new groups live in the chapter).
	 * Selection (DESIGN.md "Selection"): the tree is a region ('tree'); the open group and every picked one are
	 * `sc-selected` – inverted, the live look while the tree is the live region. The open group's slice list is a region
	 * of its own (nested). With a slice open the slice is the selection: the tree's picks are let go.
	 */
	let {
		view,
		chapter,
		groups,
		groupName,
		activeDoc,
		groupPath,
		chapterNumber,
		markAt = () => null
	}: {
		view: ChapterView;
		chapter: ContentDoc;
		groups: any[];
		groupName: (g: any, i: number) => string;
		/** the open group's file (the chapter) */
		activeDoc: ContentDoc | null;
		groupPath: Path;
		chapterNumber: number;
		/** the reachability mark at a chapter path (groups.g, groups.g.slices.s – @scutoons/content reach.ts) */
		markAt?: (...path: (string | number)[]) => ReachMark;
	} = $props();
	/** a group's slice count */
	const sliceCount = (g: any) => (g ? String(g.slices?.length ?? 0) : '');

	let picked = $state<number[]>([]);
	let anchor = $state<number | null>(null);
	// the open group is selected (a jump from the Flow / Problems too); meta, audio or a slice open: no group is
	$effect(() => {
		const section = view.section;
		const index = view.groupIndex;
		untrack(() => {
			if (section !== 'group') picked = [];
			else if (!picked.includes(index)) {
				picked = [index];
				anchor = index;
			}
		});
	});

	function clickGroup(index: number, event: MouseEvent) {
		const next = clickSelect(picked, anchor, index, event);
		picked = next.selected;
		anchor = next.anchor;
		// a plain click opens / folds it; Shift / Ctrl only pick
		if (!event.shiftKey && !event.ctrlKey && !event.metaKey) view.toggleGroup(index);
	}
	/** the shared Sortable's move: the row at `from` (and every other picked one with it) goes to `to` */
	function moveGroups(from: number, to: number) {
		const moving = picked.length > 1 && picked.includes(from) ? [...picked] : [from];
		let map = new Map<number, number>();
		chapter.edit((file) => {
			if (!Array.isArray(file.groups)) return;
			const result = moveMany(file.groups, moving, from, to);
			map = result.map;
			file.groups.splice(0, file.groups.length, ...result.list);
		});
		picked = moving.map((i) => map.get(i) ?? i).sort((a, b) => a - b);
		anchor = map.get(anchor ?? -1) ?? picked[0];
		view.groupIndex = map.get(view.groupIndex) ?? view.groupIndex;
	}
	async function removeGroup(index: number) {
		const g = groups[index];
		if (!(await confirmRemove(m['creator.remove_confirm']({ what: `group ${g?.label ?? index + 1}` })))) return;
		chapter.edit((file) => file.groups.splice(index, 1));
		view.groupIndex = Math.max(0, index - 1);
		view.section = 'meta';
	}
	// ── copy / cut / paste (BACKLOG 10.39 step 5): the picked groups (else the open one); a paste goes after the last picked / the open group (else at the
	// end), fresh ids, unique labels, media of another toon referenced; Shift+V puts the copied group's renderer, end and
	// `continues` onto every picked group of the chapter ──
	useClipHandler(
		() => 'tree',
		groupsClipHandler({
			chapter: () => chapter,
			groups: () => groups,
			chapterNumber: () => chapterNumber,
			picked: () => (picked.length ? picked : view.inGroup ? [view.groupIndex] : []),
			pick: (next) => {
				picked = next;
				anchor = next[0] ?? null;
			},
			onremove: (gone) => {
				view.groupIndex = Math.max(0, gone[0] - 1);
				view.section = 'meta';
			},
			onpaste: (at) => view.selectGroup(at)
		})
	);

	/** one "Add group" (owner 2026-10-03: "Add separate scene" was a storage detail); new groups live in the chapter */
	function addGroup() {
		const label = `scene-${groups.length + 1}`;
		chapter.edit((file) => file.groups.push(newGroup(label)));
		view.selectGroup(groups.length - 1);
	}
</script>

<div class="tree" data-testid="chapter-tree" use:region={'tree'}>
	<button class="node sc-nav-item" class:is-current={view.section === 'meta'} onclick={() => (view.section = 'meta')} data-drawer-pick data-testid="tree-meta">{m['creator.meta']()}</button>
	<button class="node sc-nav-item" class:is-current={view.section === 'audio'} onclick={() => (view.section = 'audio')} data-drawer-pick data-testid="tree-audio">{m['creator.audio']()}</button>
	<!-- the chapter's storyboard (#123): its own page, the shots as that page's list -->
	<a class="node sc-nav-item" href="/{chapter.toon}/chapter/{chapterNumber}/storyboard" data-drawer-pick data-testid="tree-storyboard">{m['storyboard.title']()}</a>
	<!-- the chapter's script (research/KANON.md §3, #81): its own page -->
	<a class="node sc-nav-item" href="/{chapter.toon}/chapter/{chapterNumber}/script" data-drawer-pick data-testid="tree-script">{m['creator.script']()}</a>
	<div class="sc-nav-head">{m['creator.groups']()}</div>
	<div class="groups" use:sortable={{ onmove: moveGroups }}>
	{#each groups as g, gi (gi)}
		{@const open = gi === view.groupIndex && view.inGroup && !view.collapsed}
		<div class="group" class:open data-sort-row>
			<div class="group-row" class:sc-selected={picked.includes(gi)} data-testid="group-row-{gi}">
				<SortGrip label={m['creator.sort_move']({ name: groupName(g, gi) })} title={m['creator.sort_hint']()} testid="group-grip-{gi}" />
				<button
					class="node sc-nav-item"
					onclick={(e) => clickGroup(gi, e)}
					aria-expanded={open}
					data-drawer-pick
					data-control-exempt="a long group name wraps its count below"
					data-nav-child={view.section === 'meta' || view.section === 'audio' ? '' : undefined}
					data-testid="group-{gi}"
				>
					<span class="name">{groupName(g, gi)}</span>
					<ReachChip mark={markAt('groups', gi)} />
					<span class="hint">
						{#if linksToNext(groups, gi)}
							<span class="link" title={m['creator.group_linked']()} data-testid="group-linked-{gi}"><Icon name="link" size="0.875rem" label={m['creator.group_linked']()} /></span>
						{/if}
						{sliceCount(g)}
					</span>
					<!-- what happens in the group (its slices added up + its own script): items, audio, video, places, events -->
					<span class="group-marks"><Marks marks={groupMarks(g)} testid="group-marks-{gi}" /></span>
				</button>
				<span class="tools">
					<button class="ghost" onclick={() => removeGroup(gi)} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
				</span>
			</div>
			{#if open}
				{#if activeDoc && !activeDoc.loading}
					<div class="slices" data-child-slices={view.section === 'group' ? '' : undefined}>
						<SliceList doc={activeDoc} {groupPath} {chapterNumber} markOf={(si) => markAt('groups', gi, 'slices', si)} selected={view.section === 'slice' ? view.sliceIndex : -1} onselect={(i) => view.selectSlice(i)} />
					</div>
				{:else}
					<p class="hint pad">{m['creator.loading']()}</p>
				{/if}
			{/if}
		</div>
	{/each}
	</div>
	<div class="tree-add">
		<button class="secondary" title={m['creator.add_group']()} onclick={addGroup}><Icon name="plus" /> <span class="one-line">{m['creator.add_group']()}</span></button>
	</div>
</div>

<style>
	/* ── the chapter tree in the sidebar (entries: tools.css sc-nav-item / sc-nav-head) ── */
	.tree {
		padding: 0 0 var(--space-05);
	}
	.group-row {
		display: flex;
		align-items: center;
	}
	/* the name comes first (BACKLOG 10.50 g: grip + remove left it a letter or two in a narrow sidebar): it is never cut –
	   a long name wraps onto more lines beside the count (at least 6rem wide; narrower, the count wraps below it) */
	.group-row .node {
		flex: 1;
		min-width: 0;
		flex-wrap: wrap;
		justify-content: space-between;
		column-gap: var(--space-05);
		row-gap: 0;
		padding-block: var(--space-025);
		overflow: hidden;
	}
	.node .hint {
		margin-left: auto;
		color: inherit;
		white-space: nowrap;
	}
	.node .name {
		flex: 1 1 0;
		min-width: min(6rem, 100%);
		overflow-wrap: anywhere;
		text-align: left;
	}
	/* the mark before the slice count (owner 2026-10-07): continues into the next group (link) */
	.node .hint .link {
		display: inline-flex;
		vertical-align: -0.125rem;
		margin-right: 0.25rem;
	}
	/* the group's marks on their own line under the name (owner 2026-10-16) */
	.group-marks {
		flex-basis: 100%;
		display: flex;
	}
	.group-marks:empty {
		display: none;
	}
	.group-row .tools {
		display: none;
	}
	.group-row:hover .tools,
	.group.open .tools {
		display: flex;
	}
	.tools button {
		min-width: var(--control-height);
		padding: 0;
	}
	.group.open {
		border-top: var(--border-width-regular) solid var(--ink);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.tree-add {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
		padding: var(--space-1);
	}
	/* one control height (owner 2026-10-02): the narrow sidebar keeps the labels on one line, the full text in the tooltip */
	.tree-add button {
		justify-content: flex-start;
		min-width: 0;
		padding-inline: var(--space-05);
	}
	.one-line {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.pad {
		padding: 1rem;
	}
</style>
