<script lang="ts" module>
	import type { FlowNode } from '@scutoons/content';

	/** what a box of the Flow panel shows (flow-graph.svelte formats it) */
	export interface FlowNodeData extends Record<string, unknown> {
		flow: FlowNode;
		title: string;
		meta?: string;
		/** the warning chip: unreachable / never true / endless loop */
		chip?: string;
		hint?: string;
		/** the group open in the editor */
		current?: boolean;
		/** a click opens something (navigation) */
		link?: boolean;
	}
</script>

<script lang="ts">
	import { Handle, Position, type NodeProps, type Node } from '@xyflow/svelte';
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import FlowPic from '$lib/components/flow/flow-pic.svelte';
	import { getFlowPictures } from '$lib/components/flow/flow-pictures.svelte.ts';
	import { iconOf, layoutIcon, picSlot } from '$lib/components/flow/flow-texts.ts';

	let { data }: NodeProps<Node<FlowNodeData>> = $props();

	/** the graph's pictures (images in the Flow, owner 2026-10-06): resolved here, so the layout never waits for them */
	const pictures = getFlowPictures();
	const n = $derived(data.flow);
	const slot = $derived(picSlot(n));
	const pic = $derived(slot ? (pictures?.picture(n.image) ?? { src: '', title: '' }) : null);
	const badge = $derived(n.badge ? pictures?.picture(n.badge) : null);
	/** a group: up to three faces, then "+n"; a run / step / question: the first speaker */
	const MAX_FACES = 3;
	const faces = $derived((n.faces ?? []).slice(0, n.kind === 'group' ? MAX_FACES : 1).map((f) => pictures?.face(f) ?? { src: '', title: f.character }));
	const more = $derived(n.kind === 'group' ? Math.max(0, (n.faces?.length ?? 0) - MAX_FACES) : 0);
	const icon = $derived(iconOf(n));
	const layout = $derived(layoutIcon(n));
	/** the face beside a run / step / question (speaker) */
	const speaker = $derived(n.kind !== 'group' && faces.length ? faces[0] : null);
</script>

<Handle type="target" position={Position.Top} isConnectable={false} />
<div
	class="box kind-{n.kind} is-{n.status} slot-{slot ?? 'none'}"
	class:current={data.current}
	class:link={!!data.link}
	class:stub={!!n.stub}
	class:has-test={!!n.condition}
	title={data.hint ? `${data.title} – ${data.hint}` : data.title}
	data-testid="flow-node-{n.id}"
>
	{#if pic}
		<span class="picture">
			<FlowPic src={pic.src} title={pic.title} shape={slot === 'round' ? 'round' : 'square'} testid="flow-pic-{n.id}" />
			{#if badge}<span class="badge"><FlowPic src={badge.src} title={badge.title} shape="round" testid="flow-badge-{n.id}" /></span>{/if}
		</span>
	{/if}
	{#if speaker}
		<span class="speaker"><FlowPic src={speaker.src} title={speaker.title} shape="round" testid="flow-face-{n.id}" /></span>
	{/if}
	<span class="body">
		<span class="title">
			{#if n.entry}<span class="icon" title={m['creator.flow_entry']()}><Icon name="log-in" size="1em" /></span>{/if}
			{#if icon}<span class="icon"><Icon name={icon} size="1em" /></span>{/if}
			<span class="text">{data.title}</span>
			{#if layout}<span class="icon end" title={layout.title}><Icon name={layout.name} size="1em" /></span>{/if}
		</span>
		{#if data.meta || data.chip}
			<span class="meta">
				{#if data.chip}<span class="sc-chip is-warning">{data.chip}</span>{/if}
				{#if data.meta}<span class="text">{data.meta}</span>{/if}
			</span>
		{/if}
		{#if n.kind === 'group' && faces.length}
			<span class="faces" data-testid="flow-faces-{n.id}">
				{#each faces as face, i (i)}<FlowPic src={face.src} title={face.title} shape="round" />{/each}
				{#if more}<span class="more" title={(n.faces ?? []).slice(MAX_FACES).map((f) => pictures?.face(f).title ?? f.character).join(', ')}>+{more}</span>{/if}
			</span>
		{/if}
	</span>
</div>
<Handle type="source" position={Position.Bottom} isConnectable={false} />

<style>
	/* DESIGN.md "Brutalist tools": ink on paper, 2px frames; a group is a strong box, a script point a lighter one, an
	   exit a pill; only through a condition = dashed; unreachable / never / loop = the warning colour (like the chips).
	   Pictures (owner 2026-10-06): fixed rem places per kind of box (flow-texts.ts boxSize), dashed while empty. */
	.box {
		box-sizing: border-box;
		width: 100%;
		height: 100%;
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: 0 var(--space-05);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		overflow: hidden;
		font-size: var(--type-caption-size);
		line-height: 1.25;
	}
	.body {
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: var(--space-025);
		min-width: 0;
		flex: 1;
	}
	.link {
		cursor: pointer;
	}
	.title,
	.meta {
		display: flex;
		align-items: center;
		gap: var(--space-025);
		min-width: 0;
	}
	.text {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		min-width: 0;
	}
	.icon {
		flex: none;
		display: inline-flex;
	}
	.icon.end {
		margin-left: auto;
		color: var(--text-muted);
	}
	.kind-group .title,
	.kind-chapter .title,
	.kind-toon .title,
	.kind-slice .title {
		font-size: 1rem;
	}
	.meta {
		color: var(--text-muted);
	}
	.meta .sc-chip {
		flex: none;
	}
	/* the picture's place: the left edge, full height (group: a slice is tall; chapter: a thumbnail is wide; toon: round) */
	.picture {
		position: relative;
		flex: none;
		display: flex;
		align-self: stretch;
		align-items: center;
	}
	.slot-tall .picture,
	.slot-wide .picture,
	.slot-round .picture,
	.slot-small .picture {
		margin-left: calc(-1 * var(--space-05));
	}
	.slot-tall .picture > :global(.pic) {
		width: 3rem;
		height: 100%;
		border-width: 0 var(--border-width-hairline) 0 0;
	}
	.slot-wide .picture > :global(.pic) {
		width: 6.5rem;
		height: 100%;
		border-width: 0 var(--border-width-hairline) 0 0;
	}
	.slot-round .picture {
		margin-left: 0;
	}
	.slot-round .picture > :global(.pic) {
		width: 3.25rem;
		height: 3.25rem;
	}
	.slot-small .picture > :global(.pic) {
		width: 2rem;
		height: 100%;
		border-width: 0 var(--border-width-hairline) 0 0;
	}
	/* another toon's avatar on the landing place's corner */
	.badge {
		position: absolute;
		right: -0.5rem;
		bottom: 0.125rem;
	}
	.badge :global(.pic) {
		width: 1.25rem;
		height: 1.25rem;
		border-width: var(--border-width-hairline);
		border-style: solid;
	}
	/* a slice: its image on top, its name below */
	.slot-top {
		flex-direction: column;
		align-items: stretch;
		padding: 0 0 var(--space-025);
		gap: var(--space-025);
	}
	.slot-top .picture > :global(.pic) {
		width: 100%;
		height: 8.5rem;
		border-width: 0 0 var(--border-width-hairline);
	}
	.slot-top .body {
		padding: 0 var(--space-05);
	}
	.speaker :global(.pic) {
		width: 1.75rem;
		height: 1.75rem;
	}
	.kind-choices .speaker :global(.pic),
	.kind-label .speaker :global(.pic) {
		width: 1.25rem;
		height: 1.25rem;
	}
	.speaker {
		flex: none;
		display: flex;
	}
	/* the characters acting in a group's script: a stack of faces, then "+n" */
	.faces {
		display: flex;
		align-items: center;
	}
	.faces :global(.pic) {
		width: 1.375rem;
		height: 1.375rem;
	}
	.faces :global(.pic + .pic) {
		margin-left: -0.375rem;
	}
	.more {
		margin-left: var(--space-025);
		color: var(--text-muted);
	}
	.kind-label,
	.kind-choices,
	.kind-divert,
	.kind-jump,
	.kind-trigger,
	.kind-run,
	.kind-step,
	.kind-place {
		border-width: var(--border-width-hairline);
	}
	/* only if its test holds (an askText step without one is solid) */
	.kind-step.has-test {
		border-style: dashed;
	}
	.kind-exit {
		border-radius: calc(infinity * 1rem);
		justify-content: center;
		text-align: center;
	}
	.kind-exit .body {
		align-items: center;
	}
	/* a pill with a picture: the picture sits in the round end */
	.kind-exit.slot-small {
		justify-content: flex-start;
		text-align: left;
	}
	.kind-exit.slot-small .body {
		align-items: flex-start;
	}
	.kind-exit.slot-small .picture > :global(.pic) {
		width: 2.5rem;
		border-radius: calc(infinity * 1rem) 0 0 calc(infinity * 1rem);
	}
	.is-conditional {
		border-style: dashed;
	}
	.is-unreachable,
	.is-never,
	.is-loop {
		border-color: var(--badge-warning);
		border-width: var(--border-width-regular);
		border-style: solid;
	}
	.is-unreachable .title {
		color: var(--text-muted);
	}
	/* the group open in the editor: inverted (selected = inverted) */
	.current {
		background: var(--ink);
		color: var(--paper);
	}
	.current .meta,
	.current .icon.end {
		color: var(--paper);
	}
	/* drawn for context (another group, a point of the script): lighter */
	.stub {
		border-width: var(--border-width-hairline);
		border-style: dashed;
		color: var(--text-muted);
	}
	.link:hover {
		outline: var(--border-width-focus) solid var(--ink);
	}
	/* zoomed far out (below ~0.6): faces and pictures step back, their places stay (the layout doesn't move) */
	:global(.flow-small) .faces,
	:global(.flow-small) .speaker,
	:global(.flow-small) .badge {
		visibility: hidden;
	}
	:global(.flow-small) .picture > :global(.pic) {
		opacity: 0.35;
	}
</style>
