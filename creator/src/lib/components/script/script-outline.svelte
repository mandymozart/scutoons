<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import type { ScriptOutlineItem } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The script's outline (screenplay editor rules, `scriptOutline`): chapters, songs and scenes; a click jumps there, the
	 * cursor's item is marked. Folds to a rail labelled "Outline" (owner 2026-10-03).
	 */
	let {
		outline,
		current,
		folded,
		list = $bindable(),
		onfold,
		onjump
	}: {
		outline: ScriptOutlineItem[];
		current: ScriptOutlineItem | null;
		folded: boolean;
		/** the scrolling list (its height is fitted to the action bar by the page) */
		list?: HTMLElement;
		onfold: (folded: boolean) => void;
		onjump: (line: number) => void;
	} = $props();

	function itemLabel(item: ScriptOutlineItem): string {
		if (item.kind === 'scene') return item.heading;
		const head = item.kind === 'chapter' ? m['creator.entity_chapter']({ number: item.number }) : m['creator.script_song']({ number: item.number });
		return item.title ? `${head} · ${item.title}` : head;
	}
</script>

{#if folded}
	<button type="button" class="fold sc-round" onclick={() => onfold(false)} aria-label={m['creator.script_outline_show']()} title={m['creator.script_outline_show']()} aria-expanded="false" data-testid="script-outline-unfold"><Icon name="chevrons-right" /></button>
	<!-- folded: written vertically, like the main sidebar's rail (owner 2026-10-03) -->
	<span class="rail-label">{m['creator.script_outline']()}</span>
{:else}
	<section class="outline" data-testid="script-outline" aria-label={m['creator.script_outline']()}>
		<h2>
			<span>{m['creator.script_outline']()}</span>
			<button type="button" class="fold sc-round" onclick={() => onfold(true)} aria-label={m['creator.script_outline_fold']()} title={m['creator.script_outline_fold']()} aria-expanded="true" data-testid="script-outline-fold"><Icon name="chevrons-left" /></button>
		</h2>
		{#if outline.length}
			<ol bind:this={list} class="outline-list">
				{#each outline as item (item.line)}
					<li class={item.kind}>
						<button type="button" class:current={item === current} onclick={() => onjump(item.line)} title="{itemLabel(item)} ({item.line})" data-line={item.line} data-testid="script-outline-item">
							{#if item.kind === 'scene'}<span class="num">{item.number}</span>{/if}
							<span class="label">{itemLabel(item)}</span>
						</button>
					</li>
				{/each}
			</ol>
		{/if}
	</section>
{/if}

<style>
	h2 {
		padding: 0 0.5rem 0 1rem;
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.5rem;
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		text-transform: uppercase;
		margin: 0 0 0.3rem;
	}
	/* like the app shell's rail label */
	.rail-label {
		writing-mode: vertical-rl;
		transform: rotate(180deg);
		color: var(--muted);
		white-space: nowrap;
	}
	.outline-list {
		max-height: var(--outline-height, calc(100vh - 220px));
		overflow: auto;
		/* thin, so the highlight reaches (almost) the divider */
		scrollbar-width: thin;
	}
	@media (max-width: 900px) {
		.outline-list {
			max-height: 240px !important;
		}
	}
	ol {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li button {
		display: flex;
		align-items: center;
		justify-content: flex-start;
		gap: 0.4rem;
		width: 100%;
		text-align: left;
		background: none;
		border: 0;
		/* every entry's text on the same 1rem line; the highlight runs edge to edge (owner 2026-10-03) */
		padding: 0 0.5rem 0 1rem;
		font-size: var(--type-caption-size);
		color: var(--ink);
		cursor: pointer;
	}
	li button:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	li button.current {
		background: var(--ink);
		color: var(--paper);
	}
	li.chapter button {
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		margin-top: 0.35rem;
	}
	li.song button {
		font-style: italic;
		margin-top: 0.35rem;
	}
	.num {
		min-width: 28px;
		color: var(--muted);
	}
	.label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
</style>
