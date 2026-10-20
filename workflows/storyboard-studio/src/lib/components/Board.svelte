<script>
	// The main view's editor "Variants" (agents/research/VARIANTS-CANVAS.md §3, issue #62 stage 2a; owner 2026-10-19:
	// "variants can be a huge open canvas with each generation next to the others, we can scroll and compare generation
	// to each other. and pick the one we like"): every generation of the PICKED shots, a band each in toon order – the
	// same set and order as the timeline's clips (band n = clip n). Over the Viewport, which stays mounted under it (App).
	// The tool row: the editor switch (V) and the keys. A click in a band = its shot current (a card: the current card too;
	// Ctrl / ⌘ + click = in / out of the selection, the current shot stays); the board scrolls to the current band when it
	// changes from elsewhere (the list, ‹ ›, the timeline). Keys on the focused board: ← → in the row, ↑ ↓ the rows –
	// over a band's edge into the next band, its shot current –, Enter chosen, R review / none, X reject / unreject, Esc
	// the selection off. The state and the statuses: board.svelte.js; the layout: board-layout.js.
	import { onMount, tick, untrack } from 'svelte';
	import { find, pickedShots, pickShot, studio } from '../studio.svelte.js';
	import { tl } from '../timeline.svelte.js';
	import { gen, loadGenerators } from '../generate.svelte.js';
	import { bandLayout, neighbour } from '../board-layout.js';
	import { board, choose, reject, review, setStatus, toggleSelected, useBoard } from '../board.svelte.js';
	import EditorSwitch from './EditorSwitch.svelte';
	import Band from './Band.svelte';

	useBoard();
	let scroller = $state();
	const HOW = 'Click: the current card (its shot current) · Ctrl + click: select (A–D) · ← → ↑ ↓ move · Enter chosen · R review · X reject (again: back) · Esc: selection off · V: the Viewport';
	const SHORT = 'click current · Ctrl + click select · arrows · Enter ✓ · R ? · X ✕';

	/** a shot's takes with a handheld render (GET /api/clips – the newest take first there, its beauty pass first): one
	 *  each, the beauty pass where there is one, in take order */
	function takesOf(id) {
		const map = new Map();
		for (const r of tl.clips[id]?.rendered ?? []) if (!map.has(r.take)) map.set(r.take, r);
		return new Map([...map].sort(([a], [b]) => a.localeCompare(b, 'en', { numeric: true })));
	}
	const bands = $derived(pickedShots().map((shot) => {
		const data = find(shot)?.shot ?? null;
		const takes = takesOf(shot);
		const records = board.records[shot] ?? [];
		return { shot, data, takes, records, layout: bandLayout({ shot, takes: [...takes.keys()], records, aspect: data?.aspect }) };
	}));
	/** the current card: the last clicked in the current shot's band – else that shot's chosen (the video first), its render */
	const cur = $derived.by(() => {
		const id = studio.sel?.id;
		const band = bands.find((b) => b.shot === id);
		if (!band) return null;
		const c = board.current;
		if (c?.shot === id && band.layout.nodes.some((n) => n.key === c.key)) return c;
		const nodes = band.layout.nodes;
		const chosen = nodes.find((n) => n.record?.approved && n.record.kind === 'video') ?? nodes.find((n) => n.record?.approved);
		return { shot: id, key: (chosen ?? nodes[0]).key };
	});
	const letters = $derived(new Map(board.selected.map((s, i) => [s.key, 'ABCD'[i]])));
	const nodeOf = (at) => bands.find((b) => b.shot === at?.shot)?.layout.nodes.find((n) => n.key === at.key) ?? null;
	const layoutOf = (shot) => bands.find((b) => b.shot === shot)?.layout;

	// the euro rate and the models' labels come with the generators' list
	onMount(() => {
		if (!gen.rate) loadGenerators().catch(() => {});
		// opened by V / the switch: the keys work at once (the Viewport's switch, just clicked, is inert under the board now)
		const at = document.activeElement;
		if (!at || at === document.body || at.closest('[inert]')) tick().then(() => cardEl(cur)?.focus({ preventScroll: true }));
	});

	// the board scrolls to the current band when the current shot changes from elsewhere (and when it opens) – not after
	// a click in a band (it would move under the pointer), not when a reload (an approval, Save, Revert) replaces the
	// same shot's copy
	let mine = null;
	let shown = null;
	$effect(() => {
		const id = studio.sel?.id;
		untrack(() => {
			if (id === shown) return;
			shown = id;
			const own = id === mine;
			mine = null;
			if (!own && id) tick().then(() => scroller?.querySelector(`[data-band="${CSS.escape(id)}"]`)?.scrollIntoView({ block: 'start' }));
		});
	});
	/** a band's shot current – from the board (no scroll) */
	function makeCurrent(shot) {
		if (shot === studio.sel?.id) return Promise.resolve(true);
		mine = shot;
		// kept the old shot ("Discard the changes?" → no): a later change from elsewhere scrolls again
		return pickShot(shot).then((ok) => {
			if (!ok && mine === shot) mine = null;
			return ok;
		});
	}
	const cardEl = (at) => (at ? scroller?.querySelector(`[data-key="${CSS.escape(at.key)}"]`) : null);

	function pick(shot, node, e) {
		e.stopPropagation();
		if (e.ctrlKey || e.metaKey) return toggleSelected({ shot, key: node.key });
		board.current = { shot, key: node.key };
		makeCurrent(shot);
	}
	/** the keys move the current card: its shot current, the card focused and in view */
	async function go(to) {
		board.current = to;
		if (!(await makeCurrent(to.shot))) return;
		await tick();
		const el = cardEl(to);
		el?.focus({ preventScroll: true });
		el?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
	}

	const DIRS = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down' };
	function keys(e) {
		if (e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented || e.target.closest?.('input, textarea, select, [contenteditable]')) return;
		// Enter on a button of the actions is that button's
		if (e.key === 'Enter' && e.target.closest?.('button:not(.card)')) return;
		const node = nodeOf(cur);
		if (DIRS[e.key]) {
			const to = cur && neighbour(bands, cur, DIRS[e.key]);
			if (to) go(to);
		} else if (e.repeat && /^(Enter|r|R|x|X)$/.test(e.key)) {
			// held down: once
		} else if (e.key === 'Enter') {
			if (node) choose(node);
		} else if (e.key === 'r' || e.key === 'R') {
			if (node) review(node);
		} else if (e.key === 'x' || e.key === 'X') {
			if (node) reject(layoutOf(cur.shot), node);
		} else if (e.key === 'Escape' && board.selected.length) {
			board.selected = [];
		} else return;
		e.preventDefault();
	}
</script>

<section class="board" aria-label="Variants">
	<div class="tools">
		<EditorSwitch />
		{#if board.selected.length}<span class="state" title="Selected for Compare (stage 4) – Ctrl + click takes one off, Esc all">{board.selected.length} selected</span>{/if}
		<span class="hint" title={HOW}><span>{SHORT}</span></span>
	</div>
	<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
	<div class="scroll" bind:this={scroller} data-board-scroll role="group" aria-label="Every generation of the picked shots – one band each, in toon order" onkeydown={keys}>
		{#if !studio.sel}
			<p class="sc-empty">Pick a shot on the left.</p>
		{:else}
			<div class="bands">
				{#each bands as band (band.shot)}
					<Band {band} current={band.shot === studio.sel?.id} cur={cur?.shot === band.shot ? cur.key : null} {letters} onpick={(node, e) => pick(band.shot, node, e)} onband={() => makeCurrent(band.shot)} onstatus={(node, s) => setStatus(band.layout, node, s)} />
				{/each}
			</div>
		{/if}
	</div>
</section>

<style>
	/* over the Viewport (App's main), the same tool row on top */
	.board {
		position: absolute;
		inset: 0;
		z-index: 2;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--paper);
	}
	.tools {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-025) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.state {
		font-size: var(--type-caption-size);
		padding: 0 var(--space-05);
		background: var(--ink);
		color: var(--paper);
		white-space: nowrap;
	}
	/* as the Viewport's: one line in the room the tools leave, the whole list in its tooltip; too narrow, it goes whole */
	.hint {
		flex: 1 1 0;
		min-width: 0;
		container-type: inline-size;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		font-size: var(--type-caption-size);
		opacity: 0.7;
	}
	@container (max-width: 22rem) {
		.hint > span {
			display: none;
		}
	}
	.scroll {
		flex: 1;
		min-height: 0;
		overflow: auto;
		scrollbar-width: thin;
	}
	/* as wide as the widest band (each scrolls with the board sideways), at least the board's width */
	.bands {
		display: flex;
		flex-direction: column;
		min-width: 100%;
		width: max-content;
	}
	.sc-empty {
		padding: var(--space-1);
	}
</style>
