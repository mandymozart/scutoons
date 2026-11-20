<script lang="ts" module>
	import type { Snippet } from 'svelte';

	/** one cell of the bar: an action (`run`), a group (`items`: its own row, ‹ back), or a custom cell (`content`) */
	export interface GroupBarItem {
		id: string;
		/** a Feather icon name */
		icon?: string;
		/** the accessible name + tooltip */
		label: string;
		run?: () => void;
		/** a group: a tap shows these instead of the main row (‹ goes back) */
		items?: GroupBarItem[];
		/** a custom cell (a language switch, a status chip, a note field …) */
		content?: Snippet;
		disabled?: boolean;
		/** pressed / on (a toggle, an open panel) */
		active?: boolean;
		/** the primary action: filled (save while there is something to save) */
		primary?: boolean;
		/** a small mark on the icon (a count, a dot) */
		badge?: string | number;
		/** a group stays open after one of its actions ran (undo / redo: tapped repeatedly) */
		stay?: boolean;
		testid?: string;
	}
</script>

<script lang="ts">
	import Icon from './icon.svelte';

	/**
	 * The creator editors' grouped action bar, rendered in the ActionBar's `groups`: a row of icon squares at the bar's
	 * end (the word is the tooltip and the accessible name); a group replaces the row with its own actions (‹ back at the
	 * left). A row that does not fit scrolls sideways; a fade at an edge shows that more follows there.
	 */
	let {
		items,
		label = 'Actions',
		backLabel = 'Back'
	}: {
		items: GroupBarItem[];
		label?: string;
		backLabel?: string;
	} = $props();

	let openGroup = $state<string | null>(null);
	const group = $derived(openGroup ? items.find((i) => i.id === openGroup && i.items) : undefined);
	const row = $derived(group ? (group.items ?? []) : items);
	const back = $derived(group ? () => (openGroup = null) : null);

	function tap(item: GroupBarItem) {
		if (item.items) {
			openGroup = item.id;
			return;
		}
		item.run?.();
		if (group && !item.stay) openGroup = null;
	}

	// the faded edges: more to scroll to on that side
	let scroller = $state<HTMLElement>();
	let fadeStart = $state(false);
	let fadeEnd = $state(false);
	function edges() {
		if (!scroller) return;
		fadeStart = scroller.scrollLeft > 2;
		fadeEnd = scroller.scrollLeft + scroller.clientWidth < scroller.scrollWidth - 2;
	}
	$effect(() => {
		const el = scroller;
		if (!el) return;
		void row;
		el.scrollLeft = 0;
		edges();
		const observer = new ResizeObserver(edges);
		observer.observe(el);
		const mutations = new MutationObserver(edges);
		mutations.observe(el, { childList: true, subtree: true });
		return () => {
			observer.disconnect();
			mutations.disconnect();
		};
	});
</script>

<div class="group-bar" role="toolbar" aria-label={group ? group.label : label} data-testid="group-bar">
	{#if back}
		<button type="button" class="back" aria-label={backLabel} title={backLabel} onclick={back} data-testid="group-bar-back"><Icon name="chevron-left" /></button>
	{/if}
	<div class="wrap" class:fade-start={fadeStart} class:fade-end={fadeEnd}>
		<div class="row" bind:this={scroller} onscroll={edges}>
			{#each row as item (item.id)}
				{#if item.content}
					<div class="cell custom" data-testid={item.testid ?? `group-bar-${item.id}`}>{@render item.content()}</div>
				{:else}
					<button
						type="button"
						class="cell"
						class:primary={item.primary}
						class:active={item.active}
						disabled={item.disabled}
						aria-pressed={item.active ?? undefined}
						aria-haspopup={item.items ? 'true' : undefined}
						title={item.label}
						aria-label={item.label}
						onclick={() => tap(item)}
						data-testid={item.testid ?? `group-bar-${item.id}`}
					>
						<span class="icon">{#if item.icon}<Icon name={item.icon} />{/if}{#if item.badge !== undefined && item.badge !== ''}<span class="badge">{item.badge}</span>{/if}</span>
						<span class="word">{item.label}</span>
					</button>
				{/if}
			{/each}
		</div>
	</div>
</div>

<style>
	.group-bar {
		display: flex;
		justify-content: flex-end;
		align-items: center;
		gap: var(--space-05);
		width: 100%;
		min-width: 0;
		height: 100%;
	}
	.wrap {
		position: relative;
		flex: 0 1 auto;
		min-width: 0;
		display: flex;
	}
	.wrap::before,
	.wrap::after {
		content: '';
		position: absolute;
		z-index: 1;
		top: 0;
		bottom: 0;
		width: 2.5rem;
		pointer-events: none;
		opacity: 0;
		transition: opacity 0.15s;
	}
	.wrap::before {
		left: 0;
		background: linear-gradient(to left, transparent, var(--paper));
	}
	.wrap::after {
		right: 0;
		background: linear-gradient(to right, transparent, var(--paper));
	}
	.wrap.fade-start::before,
	.wrap.fade-end::after {
		opacity: 1;
	}
	.row {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: var(--space-025);
		overflow-x: auto;
		scrollbar-width: none;
		overscroll-behavior-x: contain;
	}
	.row::-webkit-scrollbar {
		display: none;
	}
	.cell,
	.back {
		all: unset;
		box-sizing: border-box;
		flex: 0 0 auto;
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-05);
		min-width: var(--control-height);
		min-height: var(--control-height);
		padding: 0;
		border: var(--border-width-regular) solid transparent;
		color: var(--ink);
		cursor: pointer;
	}
	.back {
		border-color: var(--border-muted);
	}
	.cell:hover:not(:disabled),
	.back:hover {
		border-color: var(--ink);
	}
	.cell.custom {
		cursor: default;
	}
	.cell:disabled {
		opacity: 0.35;
		cursor: default;
	}
	.cell.primary:not(:disabled) {
		background: var(--ink);
		color: var(--paper);
	}
	.cell.active {
		border-color: var(--ink);
	}
	.cell:focus-visible,
	.back:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-focus));
	}
	.icon {
		position: relative;
		display: inline-flex;
	}
	.badge {
		position: absolute;
		top: -0.45rem;
		right: -0.6rem;
		min-width: 0.9rem;
		padding: 0 0.15rem;
		box-sizing: border-box;
		border-radius: 999rem;
		background: var(--ink);
		color: var(--paper);
		font-size: var(--type-nav-micro-size);
		line-height: 0.9rem;
		text-align: center;
	}
	.cell.primary .badge {
		background: var(--paper);
		color: var(--ink);
	}
	/* icons only (the word is the tooltip and the aria-label) */
	.word {
		display: none;
		white-space: nowrap;
	}
</style>
