<script lang="ts">
	import { confirmRemove } from '$lib/editor/ask.ts';
	import { CloseIcon, Icon, SortGrip, region, sortable } from '@scutoons/ui';
	import { untrack } from 'svelte';
	import { linksToNext, type Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { ChapterView } from '$lib/editor/chapter-state.svelte.ts';
	import { clickSelect, moveMany } from '$lib/editor/multi-select.ts';
	import { applyGroupSettings, clipPlacements, makeClip, mergeSounds, prepareGroup, remapSounds, soundsOf } from '$lib/editor/clip.ts';
	import { clipWhat, pastedMessage, useClipHandler } from '$lib/editor/clipboard.svelte.ts';
	import { boxIdSource, getClipContext } from '$lib/editor/clip-context.ts';
	import { newGroup } from '$lib/editor/defaults.ts';
	import SliceList from '$lib/components/chapter/slice-list.svelte';
	import ReachChip, { type ReachMark } from '$lib/components/chapter/reach-chip.svelte';

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
		markAt = () => null,
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
		/** the reachability mark at a chapter path (groups.g, groups.g.slices.s – @scutoons/content reach.ts) */
		markAt?: (...path: (string | number)[]) => ReachMark;
		/** a group as stored: a `ref` group's separate scene document (undefined while it loads), else the group itself */
		resolveGroup?: (g: any) => any;
	} = $props();
	/** slices · VN of a group – a `ref` group's from its scene document (owner 2026-10-06: the count was missing there) */
	const groupHint = (g: any) => (g ? `${g.slices?.length ?? 0}${g.events ? ' · VN' : ''}` : '');
	/** linked groups (owner 2026-10-06): which groups flow into the next one on the reader's page (scenes resolved) */
	const resolved = $derived(groups.map((g) => resolveGroup(g)));

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
		if (!(await confirmRemove(m['creator.remove_confirm']({ what: `group ${g?.label ?? g?.ref?.src ?? index + 1}` })))) return;
		chapter.edit((file) => file.groups.splice(index, 1));
		view.groupIndex = Math.max(0, index - 1);
		view.section = 'meta';
	}
	// ── copy / cut / paste (BACKLOG 10.39 step 5): the picked groups (else the open one) – a separate scene is copied as
	// its content and pasted as a group of the chapter; a paste goes after the last picked / the open group (else at the
	// end), fresh ids, unique labels, media of another toon referenced; Shift+V puts the copied group's renderer, end and
	// `continues` onto every picked group of the chapter ──
	const pickedGroups = () => (picked.length ? [...picked] : view.inGroup ? [view.groupIndex] : []).filter((i) => i < groups.length).sort((a, b) => a - b);
	useClipHandler(() => 'tree', {
		copy() {
			const clipCtx = getClipContext();
			const list = pickedGroups().map((i) => resolveGroup(groups[i])).filter(Boolean);
			if (!list.length || !clipCtx) return null;
			const entries = $state.snapshot(list).map((g: any) => {
				const { ref: _ref, ...rest } = g;
				return rest;
			});
			const slices = entries.flatMap((g: any) => g?.slices ?? []);
			const clip = makeClip('groups', chapter.toon, entries, { sounds: soundsOf(slices, chapter.get(chapter.primary, ['audio'])), chapter: chapterNumber });
			return { clip, what: clipWhat(clip) };
		},
		remove() {
			const gone = pickedGroups();
			if (!gone.length) return;
			chapter.edit((file) => {
				if (Array.isArray(file.groups)) for (const i of [...gone].reverse()) file.groups.splice(i, 1);
			});
			picked = [];
			anchor = null;
			view.groupIndex = Math.max(0, gone[0] - 1);
			view.section = 'meta';
		},
		paste(clip) {
			const clipCtx = getClipContext();
			if (clip.kind !== 'groups' || !clipCtx) return null;
			const refused = clipCtx.placementLimit(clipPlacements(clip));
			if (refused) return refused;
			const now = pickedGroups();
			const at = now.length ? Math.max(...now) + 1 : groups.length;
			const labels = new Set(clipCtx.groupLabels());
			const nextBoxId = boxIdSource(clipCtx, (_s, i) => i + 1);
			const prepared = clip.entries.map((g) => prepareGroup(clip, g, { toon: chapter.toon, nextBoxId }, labels));
			chapter.edit((file) => {
				const map = mergeSounds(file, clip.sounds, clip.toon, chapter.toon);
				for (const g of prepared) for (const s of g.slices ?? []) remapSounds(s, map);
				file.groups ??= [];
				file.groups.splice(at, 0, ...prepared);
			});
			view.selectGroup(at);
			picked = prepared.map((_, n) => at + n);
			anchor = at;
			return { message: pastedMessage(clip, chapter.toon) };
		},
		pasteSettings(clip) {
			const clipCtx = getClipContext();
			if (clip.kind !== 'groups') return null;
			const source = clip.entries[0];
			// a separate scene keeps its renderer / end / continues in its own document: only the chapter's groups here
			const targets = pickedGroups().filter((i) => !groups[i]?.ref);
			if (!targets.length || !source) return { message: m['creator.clip_no_settings']() };
			chapter.edit((file) => {
				for (const i of targets) if (file.groups?.[i]) applyGroupSettings(file.groups[i], source);
			});
			return { message: pastedMessage(clip, chapter.toon, true, m['creator.clip_what_groups']({ count: targets.length })) };
		}
	});

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
	}
	/* the name comes first (BACKLOG 10.50 g: grip + remove left it a letter or two in a narrow sidebar): it keeps its full
	   width (cut only when even the whole row is too narrow), the count / VN wraps below it when there is no room */
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
		flex: 1 0 auto;
		max-width: 100%;
		min-width: 0;
		overflow-wrap: anywhere;
		text-align: left;
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
