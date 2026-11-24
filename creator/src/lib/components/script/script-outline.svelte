<script lang="ts" module>
	import type { ScriptOutlineItem } from '@scutoons/content';

	/** a Kanon text's `## ` heading (research/KANON.md §4); `known` = one of its role's sections */
	export interface SectionOutlineItem {
		kind: 'section';
		line: number;
		heading: string;
		known: boolean;
	}
	export type OutlineItem = ScriptOutlineItem | SectionOutlineItem;
</script>

<script lang="ts">
	import { FoldButton } from '@scutoons/ui';
	import { ListRow } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The script's outline (screenplay editor rules, `scriptOutline`): chapters, songs and scenes; a click jumps there, the
	 * cursor's item is marked. Its fold button folds the page's outline region to a rail labelled "Outline" (owner 2026-10-03;
	 * the rail and the region: script-editor.svelte, @scutoons/ui `Rail` / `dockRegion`). A Kanon text's outline is its
	 * sections (`label` "Sections"; a heading that is not one of its role's is muted).
	 */
	let {
		outline,
		current,
		label = m['creator.script_outline'](),
		foldLabel = m['creator.script_outline_fold'](),
		list = $bindable(),
		onfold,
		onjump
	}: {
		outline: OutlineItem[];
		current: OutlineItem | null;
		label?: string;
		foldLabel?: string;
		/** the scrolling list (its height is fitted to the action bar by the page) */
		list?: HTMLElement;
		onfold: () => void;
		onjump: (line: number) => void;
	} = $props();

	function itemLabel(item: OutlineItem): string {
		if (item.kind === 'scene' || item.kind === 'section') return item.heading;
		const head = item.kind === 'chapter' ? m['creator.entity_chapter']({ number: item.number }) : m['creator.script_song']({ number: item.number });
		return item.title ? `${head} · ${item.title}` : head;
	}
</script>

<section class="outline" data-testid="script-outline" aria-label={label}>
	<h2>
		<span>{label}</span>
		<FoldButton side="left" label={foldLabel} controls="script-outline" onclick={onfold} testid="script-outline-fold" />
	</h2>
	{#if outline.length}
		<ol bind:this={list} class="outline-list">
			{#each outline as item (item.line)}
				<li class={item.kind} class:unknown={item.kind === 'section' && !item.known}>
					<ListRow dense label={itemLabel(item)} selected={item === current} onclick={() => onjump(item.line)} title={item.kind === 'section' && !item.known ? `${itemLabel(item)} – ${m['creator.kanon_not_a_section']()} (${item.line})` : `${itemLabel(item)} (${item.line})`} data-line={item.line} data-testid="script-outline-item">
						{#snippet lead()}{#if item.kind === 'scene'}<span class="num">{item.number}</span>{/if}{/snippet}
					</ListRow>
				</li>
			{/each}
		</ol>
	{/if}
</section>

<style>
	h2 {
		padding: 0 0.5rem 0 1rem;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		color: var(--tool-text);
		text-transform: uppercase;
		margin: 0 0 0.3rem;
	}
	.outline-list {
		max-height: var(--outline-height, calc(100vh - 14rem));
		overflow: auto;
		/* thin, so the highlight reaches (almost) the divider */
		scrollbar-width: thin;
	}
	@media (max-width: 900px) {
		.outline-list {
			max-height: 15rem !important;
			/* the list goes on below: it fades out instead of cutting a row in half (owner 2026-10-21: the fade where a
			   list has more) */
			mask-image: linear-gradient(#000 calc(100% - 2rem), transparent);
			padding-bottom: 1rem;
		}
	}
	ol {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li.chapter :global(.kit-list-row) {
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		margin-top: 0.35rem;
	}
	li.song :global(.kit-list-row) {
		font-style: italic;
		margin-top: 0.35rem;
	}
	li.unknown :global(.kit-list-row:not(.selected)) {
		color: var(--tool-muted);
	}
	.num {
		min-width: 1.75rem;
		color: var(--tool-muted);
	}
</style>
