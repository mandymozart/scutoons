<script lang="ts">
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { CloseIcon, Icon } from '@scutoons/ui';
	import { linksToNext, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { ChapterView } from '$lib/editor/chapter-state.svelte.ts';
	import { clickSelect, moveMany } from '$lib/editor/multi-select.ts';
	import { newGroup } from '$lib/editor/defaults.ts';
	import SliceList from '$lib/components/chapter/slice-list.svelte';

	/**
	 * The chapter's children in the sidebar (COMPONENTS.md step 12): meta, audio, groups → the open group's slices
	 * (Alt+ArrowDown opens the first child). Groups: click opens / folds one, Shift+click picks the range from the anchor,
	 * Ctrl / Cmd+click adds / removes one; dragging a picked group moves all picked ones (in their order) – owner
	 * 2026-10-03. One "Add group" (new groups live in the chapter).
	 */
	let {
		view,
		chapter,
		groups,
		groupName,
		activeDoc,
		groupPath,
		chapterNumber,
		resolveGroup = (g) => g
	}: {
		view: ChapterView;
		chapter: ContentDoc;
		groups: any[];
		groupName: (g: any, i: number) => string;
		/** the open group's file (the chapter, or its separate scene) */
		activeDoc: ContentDoc | null;
		groupPath: Path;
		chapterNumber: number;
		/** a group as stored: a `ref` group's separate scene document (undefined while it loads), else the group itself */
		resolveGroup?: (g: any) => any;
	} = $props();
	/** slices · VN of a group – a `ref` group's from its scene document (owner 2026-10-06: the count was missing there) */
	const groupHint = (g: any) => (g ? `${g.slices?.length ?? 0}${g.events ? ' · VN' : ''}` : '');
	/** linked groups (owner 2026-10-06): which groups flow into the next one on the reader's page (scenes resolved) */
	const resolved = $derived(groups.map((g) => resolveGroup(g)));

	let picked = $state<number[]>([]);
	let anchor = $state<number | null>(null);
	let dragging = $state<number[] | null>(null);
	let over = $state<number | null>(null);

	function clickGroup(index: number, event: MouseEvent) {
		const next = clickSelect(picked, anchor, index, event);
		picked = next.selected;
		anchor = next.anchor;
		// a plain click opens / folds it; Shift / Ctrl only pick
		if (!event.shiftKey && !event.ctrlKey && !event.metaKey) view.toggleGroup(index);
	}
	function dropGroups(target: number) {
		const moving = dragging;
		if (!moving?.length) return;
		let map = new Map<number, number>();
		chapter.structural((file) => {
			if (!Array.isArray(file.groups)) return;
			const result = moveMany(file.groups, moving, target);
			map = result.map;
			file.groups.splice(0, file.groups.length, ...result.list);
		});
		picked = moving.map((i) => map.get(i) ?? i).sort((a, b) => a - b);
		anchor = map.get(anchor ?? -1) ?? picked[0];
		view.groupIndex = map.get(view.groupIndex) ?? view.groupIndex;
	}
	async function removeGroup(index: number) {
		const g = groups[index];
		if (!(await confirmRemove(m['creator.remove_confirm']({ what: `group ${g?.label ?? g?.ref?.src ?? index + 1}` })))) return;
		chapter.structural((file) => file.groups.splice(index, 1));
		view.groupIndex = Math.max(0, index - 1);
		view.section = 'meta';
	}
	/** one "Add group" (owner 2026-10-03: "Add separate scene" was a storage detail); new groups live in the chapter */
	function addGroup() {
		const label = `scene-${groups.length + 1}`;
		chapter.structural((file) => file.groups.push(newGroup(label)));
		view.selectGroup(groups.length - 1);
	}
</script>

<div class="tree" data-testid="chapter-tree">
	<button class="node sc-nav-item" class:is-current={view.section === 'meta'} onclick={() => (view.section = 'meta')} data-testid="tree-meta">{m['creator.meta']()}</button>
	<button class="node sc-nav-item" class:is-current={view.section === 'audio'} onclick={() => (view.section = 'audio')} data-testid="tree-audio">{m['creator.audio']()}</button>
	<div class="sc-nav-head">{m['creator.groups']()}</div>
	{#each groups as g, gi (gi)}
		{@const open = gi === view.groupIndex && view.inGroup && !view.collapsed}
		<div class="group" class:open>
			<!-- svelte-ignore a11y_no_static_element_interactions -->
			<div
				class="group-row"
				class:picked={picked.length > 1 && picked.includes(gi)}
				class:over={over === gi && !dragging?.includes(gi)}
				class:below={over === gi && !!dragging?.length && gi > Math.min(...dragging)}
				class:moving={dragging?.includes(gi)}
				draggable="true"
				ondragstart={(e) => {
					dragging = picked.includes(gi) ? [...picked] : [gi];
					if (!picked.includes(gi)) picked = [gi];
					e.dataTransfer?.setData('text/plain', dragging.join(','));
					if (e.dataTransfer) e.dataTransfer.effectAllowed = 'move';
				}}
				ondragover={(e) => {
					if (!dragging) return;
					e.preventDefault();
					over = gi;
				}}
				ondragleave={() => (over = over === gi ? null : over)}
				ondrop={(e) => {
					if (!dragging) return;
					e.preventDefault();
					dropGroups(gi);
					dragging = over = null;
				}}
				ondragend={() => (dragging = over = null)}
			>
				<button
					class="node sc-nav-item"
					class:is-current={view.section === 'group' && gi === view.groupIndex}
					onclick={(e) => clickGroup(gi, e)}
					aria-expanded={open}
					data-nav-child={view.section === 'meta' || view.section === 'audio' ? '' : undefined}
					data-testid="group-{gi}"
				>
					{groupName(g, gi)}
					<span class="hint">
						{#if linksToNext(resolved, gi)}
							<span class="link" title={m['creator.group_linked']()} data-testid="group-linked-{gi}"><Icon name="link" size="0.875rem" label={m['creator.group_linked']()} /></span>
						{/if}
						{groupHint(resolved[gi])}
					</span>
				</button>
				<span class="tools">
					<button class="ghost" onclick={() => removeGroup(gi)} aria-label={m['creator.remove']()} title={m['creator.remove']()}><CloseIcon /></button>
				</span>
			</div>
			{#if open}
				{#if activeDoc && !activeDoc.loading}
					<div class="slices" data-child-slices={view.section === 'group' ? '' : undefined}>
						<SliceList doc={activeDoc} {groupPath} {chapterNumber} selected={view.section === 'slice' ? view.sliceIndex : -1} onselect={(i) => view.selectSlice(i)} />
					</div>
				{:else}
					<p class="hint pad">{m['creator.loading']()}</p>
				{/if}
			{/if}
		</div>
	{/each}
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
	}
	/* group drag (owner 2026-10-03): the drop line above / below the target, picked groups framed, the moving ones faint */
	.group-row.over {
		box-shadow: inset 0 3px 0 var(--cyan);
	}
	.group-row.over.below {
		box-shadow: inset 0 -3px 0 var(--cyan);
	}
	.group-row.picked {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	.group-row.moving {
		opacity: 0.5;
	}
	.group-row .node {
		flex: 1;
		min-width: 0;
		justify-content: space-between;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.node .hint {
		color: inherit;
	}
	/* the link mark of a group that continues into the next one */
	.node .hint .link {
		display: inline-flex;
		vertical-align: -0.125rem;
		margin-right: 0.25rem;
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
