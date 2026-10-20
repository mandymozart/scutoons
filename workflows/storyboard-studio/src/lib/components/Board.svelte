<script>
	// The main view's editor "Variants" (agents/research/VARIANTS-CANVAS.md §3, issue #62 stage 2; owner 2026-10-19:
	// "variants can be a huge open canvas with each generation next to the others, we can scroll and compare generation
	// to each other. and pick the one we like"): every generation of the PICKED shots, a band each in toon order – the
	// same set and order as the timeline's clips (band n = clip n). Over the Viewport, which stays mounted under it (App).
	// It holds what the Variants dock panel held (§3.1 – the panel retired): the saved frame (the render card), the layout
	// variants (source cards – Load · Take this), the generated files, the drops (files dropped onto a band = uploads for
	// its shot; the tool row's + for the current one), and the running jobs as placeholders (board-sources.svelte.js).
	// The tool row: the editor switch (V), the keys, Show rejected · n, the zoom, + (BoardTools). A click in a band = its
	// shot current (a card: the current card too; Ctrl / ⌘ + click = in / out of the selection, the current shot stays; a
	// strip: a look into it); the board scrolls to the current band when it changes from elsewhere (the list, ‹ ›, the
	// timeline). Keys on the focused board: ← → in the row, ↑ ↓ the rows – over a band's edge into the next band, its shot
	// current; strips and folded bands skipped –, Enter chosen, R review / none, X reject / unreject, I improve on this
	// (fills Generate – improve.svelte.js), + / − zoom, 0 fits the widest band; C Compare, Esc closes a look, then
	// Compare, then the selection (also from the tool row, the Divider). COMPARE (stage 4, §3.4): the selection right of
	// the bands, a Divider between (compare.svelte.js – the width remembered); open, a click on a card fills its first
	// empty side, else replaces the last; Space on the board plays / pauses its clock (not a click on the focused card);
	// opened by its button, the focus goes into it. Pan / zoom: board-view.svelte.js; the state and the statuses:
	// board.svelte.js; the layout: board-layout.js.
	import { onDestroy, onMount, tick, untrack } from 'svelte';
	import { Divider } from '@scutoons/ui';
	import { fail, notify, find, pickedShots, pickShot, studio } from '../studio.svelte.js';
	import { tl } from '../timeline.svelte.js';
	import { gen, loadGenerators } from '../generate.svelte.js';
	import { bandFacts, bandLayout, neighbour } from '../board-layout.js';
	import { fitZoom, zoomStep } from '../board-zoom.js';
	import { board, choose, openLook, reject, review, selection, setAlive, setCurrent, setStatus, toggleFold, toggleSelected, useBoard } from '../board.svelte.js';
	import { carriesFiles, sourceAction, upload, useJobs } from '../board-sources.svelte.js';
	import { improve } from '../improve.svelte.js';
	import { panZoom, remPx, view, zoomAt } from '../board-view.svelte.js';
	import { closeCompare, compareClick, compareSplit, toggleCompare } from '../compare.svelte.js';
	import { currentCard } from '../audition.js';
	import { lettersOf } from '../compare.js';
	import BoardTools from './BoardTools.svelte';
	import Band from './Band.svelte';
	import Compare from './Compare.svelte';

	useBoard();
	useJobs();
	let scroller = $state();

	/** a shot's takes with a handheld render (GET /api/clips – the newest take first there, its beauty pass first): one
	 *  each, the beauty pass where there is one, in take order */
	function takesOf(id) {
		const map = new Map();
		for (const r of tl.clips[id]?.rendered ?? []) if (!map.has(r.take)) map.set(r.take, r);
		return new Map([...map].sort(([a], [b]) => a.localeCompare(b, 'en', { numeric: true })));
	}
	/** what a band's layout is made of (the zoom, Show rejected and the look added – layoutAt) */
	const sources = $derived(pickedShots().map((shot) => {
		const data = find(shot)?.shot ?? null;
		const takes = takesOf(shot);
		return { shot, data, takes, records: board.records[shot] ?? [], variants: board.variants[shot] ?? [], jobs: board.jobs[shot] ?? [] };
	}));
	const layoutAt = (s, zoom) => bandLayout({ shot: s.shot, variants: s.variants, takes: [...s.takes.keys()], records: s.records, jobs: s.jobs, aspect: s.data?.aspect, zoom, showRejected: board.showRejected, open: board.look?.shot === s.shot ? board.look.keys : [] });
	const bands = $derived(sources.map((s) => ({ ...s, folded: !!board.folded[s.shot], layout: layoutAt(s, board.zoom) })));
	const rejected = $derived(sources.reduce((n, s) => n + bandFacts(s.records).rejected, 0));
	// "Show rejected" is hidden at 0 – and off then (not remembered): the next X folds again
	$effect(() => {
		if (!rejected) untrack(() => (board.showRejected = false));
	});
	/** the current card: the last clicked in the current shot's band – else that shot's chosen (the video first), its
	 *  render (audition.js currentCard – the timeline's audition takes the same) */
	const cur = $derived.by(() => {
		const [id, band] = [studio.sel?.id, bandOf(studio.sel?.id)];
		return band ? (currentCard({ shot: id, current: board.current, records: band.records, has: (k) => band.layout.tree.has(k) }) ?? { shot: id, key: band.layout.nodes[0].key }) : null;
	});
	const bandOf = (shot) => bands.find((b) => b.shot === shot);
	/** the card (the tree's – also one folded into a strip) */
	const nodeOf = (at) => bandOf(at?.shot)?.layout.tree.get(at.key) ?? null;
	// the selection as it stands (board.svelte.js selection): a card gone from the board is an empty side, not a letter
	setAlive((s) => !!nodeOf(s));
	onDestroy(() => setAlive(null));
	const sel = $derived(selection());
	const letters = $derived(lettersOf(sel));

	// --- Compare (§3.4): its width (compare.svelte.js compareSplit – remembered), its sides = the selection (holes: null)
	let split = $state();
	const cmp = compareSplit(() => split);
	const cmpWidth = $derived(cmp.width());
	const sides = $derived(sel.map((s) => s && { ...s, node: nodeOf(s), takes: bandOf(s.shot)?.takes }));
	const compare = () => toggleCompare({ shot: studio.sel?.id, records: board.records[studio.sel?.id] ?? [], current: cur, nodeOf });
	/** Compare's pane (its clock for Space on the board, its focus) */
	let pane = $state();
	/** "Compare · n": opened by the button, the focus goes into Compare – Space, Esc and C are its keys there */
	const button = () => compare() && tick().then(() => pane?.focus());
	/** Compare closed (✕, Esc, C): the board's keys go on */
	function closed() {
		closeCompare();
		tick().then(() => cardEl(cur)?.focus({ preventScroll: true }));
	}

	// the euro rate and the models' labels come with the generators' list
	onMount(() => {
		if (!gen.rate) loadGenerators().catch(() => {});
		// opened by V / the switch: the keys work at once (the Viewport's switch, just clicked, is inert under the board now)
		const at = document.activeElement;
		if (!at || at === document.body || at.closest('[inert]')) tick().then(() => cardEl(cur)?.focus({ preventScroll: true }));
	});

	// the board scrolls to the current band when the current shot changes from elsewhere (and when it opens) – not after
	// a click in a band (it would move under the pointer), not when a reload (an approval, Save, Revert) replaces the
	// same shot's copy; a look into another shot's strip closes
	let mine = null;
	let shown = null;
	$effect(() => {
		const id = studio.sel?.id;
		untrack(() => {
			if (id === shown) return;
			shown = id;
			if (board.look && board.look.shot !== id) board.look = null;
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
	/** a card's element – or the strip it is folded into */
	const cardEl = (at) => (at ? scroller?.querySelector(`[data-key="${CSS.escape(at.key)}"], [data-keys~="${CSS.escape(at.key)}"]`) : null);

	async function pick(shot, node, e) {
		e.stopPropagation();
		if (e.ctrlKey || e.metaKey) return node.kind === 'strip' ? notify('A rejected card stays out of Compare (X brings it back).') : toggleSelected({ shot, key: node.key }, node);
		// a strip: a look into it and its branch (§3.5 – "a rejected tree is folded. unfolding makes it always possible to dive in that tree")
		if (node.kind === 'strip') openLook(shot, bandOf(shot).layout, node.key);
		else setCurrent({ shot, key: node.key });
		// Compare open: the card comes in as its last side – A stays the reference
		if (board.compare && node.kind !== 'strip') compareClick({ shot, key: node.key }, node);
		const ok = await makeCurrent(shot);
		// kept the old shot ("Discard the changes?" → no): no look into another shot's band
		if (node.kind === 'strip' && !ok) board.look = null;
		else if (node.kind === 'strip') tick().then(() => cardEl({ key: node.key })?.focus({ preventScroll: true }));
	}
	/** the keys move the current card: its shot current, the card focused and in view */
	async function go(to) {
		setCurrent(to);
		if (!(await makeCurrent(to.shot))) return;
		await tick();
		const el = cardEl(to);
		el?.focus({ preventScroll: true });
		el?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
	}

	// --- the zoom (one for all bands) -------------------------------------------------------------------------------------
	/** + / −: about the current card when it is in view, else the view's middle */
	function zoomKey(dir) {
		const el = cardEl(cur)?.querySelector('.pic') ?? cardEl(cur);
		const r = el?.getBoundingClientRect();
		const v = scroller?.getBoundingClientRect();
		const inView = r && v && r.right > v.left && r.left < v.right && r.bottom > v.top && r.top < v.bottom;
		zoomAt(scroller, zoomStep(board.zoom, dir), inView ? r.left + r.width / 2 : undefined, inView ? r.top + r.height / 2 : undefined);
	}
	/** 0: the widest open band fits the board's width; the current card stays in view */
	async function fit() {
		const open = bands.filter((b) => !b.folded);
		if (!scroller || !open.length) return;
		board.zoom = fitZoom((z) => Math.max(...open.map((s) => layoutAt(s, z).width)), scroller.clientWidth / remPx());
		await tick();
		scroller.scrollLeft = 0;
		cardEl(cur)?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
	}

	/** after a key folded / unfolded the focused card (a card ↔ its strip – another element): the focus back on the current one */
	async function refocus(done) {
		await done;
		await tick();
		const at = document.activeElement;
		if (!at || at === document.body || !at.isConnected) cardEl(cur)?.focus({ preventScroll: true });
	}

	/** a file dragged over the board but not over a band (a band took it first: defaultPrevented): no drop here */
	function nowhere(e) {
		if (e.defaultPrevented || !carriesFiles(e)) return;
		e.preventDefault();
		if (e.type === 'dragover') e.dataTransfer.dropEffect = 'none';
	}

	const DIRS = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down' };
	function keys(e) {
		// More ▾'s OPEN list (a Menu) keeps its own keys – its closed button (the focus after Escape / an item) is the board's
		if (e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented || e.target.closest?.('input, textarea, select, [contenteditable], .sc-menu .popup, .sc-menu > [aria-expanded="true"]')) return;
		// Enter / Space on a button of the actions or a band's fold is that button's; Enter on a strip opens the look (its click)
		if ((e.key === 'Enter' && e.target.closest?.('button:not(.card)')) || (e.key === ' ' && e.target.closest?.('button:not(.card, .strip)'))) return;
		const node = nodeOf(cur);
		// a folded band's card: the keys leave it (arrows), nothing else acts on it unseen
		const hidden = !!bandOf(cur?.shot)?.folded;
		if (DIRS[e.key]) {
			const to = cur && neighbour(bands, cur, DIRS[e.key]);
			if (to) go(to);
		} else if (e.repeat && /^(Enter|r|R|x|X|i|I)$/.test(e.key)) {
			// held down: once
		} else if (e.key === 'Enter') {
			if (node && !hidden) choose(node);
		} else if (e.key === 'r' || e.key === 'R') {
			if (node && !hidden) review(node);
		} else if (e.key === 'x' || e.key === 'X') {
			if (node && !hidden) refocus(reject(bandOf(cur.shot).layout, node));
		} else if (e.key === 'i' || e.key === 'I') {
			// Improve on this – before App's I (the trim at the playhead): the board has the focus
			if (node && !hidden) improve(cur.shot, node, node.source === 'take' ? bandOf(cur.shot).takes.get(node.take) : null).catch(fail);
		} else if (e.key === '+' || e.key === '=') {
			zoomKey(1);
		} else if (e.key === '-') {
			zoomKey(-1);
		} else if (e.key === '0') {
			fit();
		} else if (e.key === ' ' && board.compare) {
			// Compare open: Space plays / pauses its clock – not a click on the focused card (it would replace a side)
			if (!e.repeat) pane?.space();
		} else return;
		e.preventDefault();
	}
	/** C and Esc – on the bands, in the tool row, on the Divider (Compare keeps its own): Esc closes a look, then
	 *  Compare, then the selection */
	function outer(e) {
		if (e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented || e.repeat || e.target.closest?.('.compare, input, textarea, select, [contenteditable], .sc-menu .popup, .sc-menu > [aria-expanded="true"]')) return;
		if (e.key === 'c' || e.key === 'C') compare();
		else if (e.key === 'Escape' && board.look) refocus((board.look = null));
		else if (e.key === 'Escape' && board.compare) closed();
		else if (e.key === 'Escape' && board.selected.length) board.selected = [];
		else return;
		e.preventDefault();
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section class="board" aria-label="Variants" onkeydown={outer}>
	<BoardTools {rejected} selected={sel.filter(Boolean).length} oncompare={button} onzoom={(z) => zoomAt(scroller, z)} onfiles={(files) => upload(studio.sel?.id, files)} />
	<div class="split" bind:this={split}>
		<!-- files dropped between the bands land nowhere (the browser would open them in place of the studio) -->
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div class="scroll" class:panning={view.panning} bind:this={scroller} data-board-scroll role="group" aria-label="Every generation of the picked shots – one band each, in toon order" onkeydown={keys} ondragover={nowhere} ondrop={nowhere} use:panZoom>
			{#if !studio.sel}
				<p class="sc-empty">Pick a shot on the left.</p>
			{:else}
				<div class="bands">
					{#each bands as band (band.shot)}
						<Band {band} current={band.shot === studio.sel?.id} cur={cur?.shot === band.shot ? cur.key : null} {letters} onpick={(node, e) => pick(band.shot, node, e)} onband={() => makeCurrent(band.shot)} onfold={() => toggleFold(band.shot)} onstatus={(node, s) => setStatus(band.layout, node, s)} onaction={(node, a) => sourceAction(band.shot, node, a)} ondrop={(files) => upload(band.shot, files)} />
					{/each}
				</div>
			{/if}
		</div>
		{#if board.compare}
			<Divider size={cmp.size} edge="left" at={cmpWidth} label="Resize Compare – the bands get the rest (Home: back to its normal width)" />
			<Compare bind:this={pane} {sides} width={cmpWidth} onclose={closed} />
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
	/* the bands | Compare (the Divider's container) */
	.split {
		position: relative;
		flex: 1;
		display: flex;
		min-height: 0;
	}
	.scroll {
		flex: 1;
		min-width: 0;
		min-height: 0;
		overflow: auto;
		scrollbar-width: thin;
	}
	/* a drag on the background pans (board-view.svelte.js) */
	.scroll.panning {
		cursor: grabbing;
		user-select: none;
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
