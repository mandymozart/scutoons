<script lang="ts">
	import { CloseButton, Segmented } from '@scutoons/ui';
	import { chapterFlow, groupFlow, sliceFlow, type ChapterReach, type CharacterRegistry, type FlowNode, type ToonFacts } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import type { FlowTextContext } from '$lib/components/flow/flow-texts.ts';
	import { FlowPictures } from '$lib/components/flow/flow-pictures.svelte.ts';

	/**
	 * The chapter's story flow (owner 2026-10-06, agents/DESIGN.md "Flow panel"): a read-only, zoomable graph of what leads
	 * where – @scutoons/content `chapterFlow` / `groupFlow` / `sliceFlow` over the editor's own `chapterReach`, so it
	 * agrees with the "unreachable" chips. The level follows what is selected in the editor (a group → its group level, a
	 * slice → its slice level); the small level switch goes back up. Docked on the right like Problems / History (left of
	 * the Ask panel). The graph (Svelte Flow + dagre) is loaded only when the panel opens.
	 */
	let {
		groups,
		reach,
		chapter,
		current = -1,
		slice = -1,
		name,
		text,
		who,
		toon,
		registry = null,
		facts,
		nextChapter,
		onopen,
		onclose
	}: {
		/** the groups `reach` was computed from (scenes in place of their refs) */
		groups: readonly any[] | null;
		reach: ChapterReach | null;
		chapter: number;
		/** the group open in the editor (-1: none) */
		current?: number;
		/** the slice open in the editor (-1: none) */
		slice?: number;
		name: (group: any, index: number) => string;
		text: (value: unknown) => string;
		/** a character key (`boy`, `catpope·cat`) as shown */
		who?: (key: string) => string;
		/** the chapter's toon (the pictures: slices, other chapters, toons) */
		toon: string;
		/** the character registry: the faces */
		registry?: CharacterRegistry | null;
		/** the toon's facts: which groups are entry points */
		facts?: ToonFacts;
		/** the season's next chapter (the follow-up bar's picture after the last group) */
		nextChapter?: number;
		onopen: (node: FlowNode) => void;
		onclose: () => void;
	} = $props();

	const graph = import('$lib/components/flow/flow-graph.svelte');

	type Level = 'chapter' | 'group' | 'slice';
	/** the deepest level the selection allows */
	const deepest = $derived<Level>(current >= 0 && slice >= 0 ? 'slice' : current >= 0 ? 'group' : 'chapter');
	/** the level the author picked (null: follow the selection) – a new selection follows again */
	let picked = $state<Level | null>(null);
	$effect(() => {
		void current;
		void slice;
		picked = null;
	});
	const level = $derived<Level>(picked && (picked === 'chapter' || (picked === 'group' && current >= 0) || (picked === 'slice' && slice >= 0)) ? picked : deepest);
	const levels = $derived(
		[
			{ value: 'chapter' as Level, label: m['creator.flow_level_chapter'](), testid: 'flow-level-chapter' },
			...(current >= 0 ? [{ value: 'group' as Level, label: m['creator.flow_level_group'](), testid: 'flow-level-group' }] : []),
			...(current >= 0 && slice >= 0 ? [{ value: 'slice' as Level, label: m['creator.flow_level_slice'](), testid: 'flow-level-slice' }] : [])
		]
	);

	const flow = $derived.by(() => {
		if (!groups || !reach) return null;
		const options = { chapter, toon, reach, name, text, facts, nextChapter };
		if (level === 'slice') return sliceFlow(groups, current, slice, options);
		if (level === 'group') return groupFlow(groups, current, options);
		return chapterFlow(groups, options);
	});
	const ctx = $derived<FlowTextContext>({ chapter, who });
	/** the boxes' pictures: slices, chapters, toons, items, faces (resolved in the boxes, loaded lazily) */
	// svelte-ignore state_referenced_locally
	const pictures = new FlowPictures(toon, { registry, items: true });
	$effect(() => {
		pictures.registry = registry;
	});
	/** the box of what's open: the group on the chapter level, the head of the group / slice level */
	const isCurrent = (n: FlowNode) =>
		level === 'chapter' ? n.kind === 'group' && n.group === current : level === 'group' ? n.kind === 'group' && n.group === current && !n.stub : n.kind === 'slice';
</script>

<section class="flow" data-testid="flow-panel" data-level={level}>
	<header>
		<h2>{m['creator.flow']()}</h2>
		<CloseButton label={m['creator.close']()} onclick={onclose} />
	</header>
	{#if levels.length > 1}
		<div class="levels">
			<Segmented options={levels} value={level} onchange={(v) => (picked = v)} label={m['creator.flow_levels']()} testid="flow-levels" />
		</div>
	{/if}
	<div class="canvas">
		{#if !flow}
			<p class="hint">{m['creator.loading']()}</p>
		{:else if groups?.length}
			{#await graph}
				<p class="hint">{m['creator.loading']()}</p>
			{:then mod}
				{#key `${level}:${current}:${slice}`}
					<mod.default {flow} {ctx} current={isCurrent} {pictures} {onopen} />
				{/key}
			{:catch error}
				<p class="hint error">{String(error?.message ?? error)}</p>
			{/await}
		{/if}
	</div>
</section>

<style>
	.flow {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}
	/* the Ask panel's header (owner 2026-10-06): one control row, the title fills, edge to edge */
	header {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		margin: 0;
		flex: 1;
	}
	.levels {
		padding: var(--space-05) var(--space-1) 0;
	}
	.hint {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
		margin: var(--space-05) var(--space-1);
	}
	.error {
		color: var(--error);
	}
	.canvas {
		flex: 1;
		min-height: 0;
		position: relative;
	}
</style>
