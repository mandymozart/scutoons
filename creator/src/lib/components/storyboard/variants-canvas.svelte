<script lang="ts">
	// The main view's editor "Variants" (#125; VARIANTS-CANVAS §3, owner 2026-10-19: "variants can be a huge open canvas
	// with each generation next to the others, we can scroll and compare generation to each other. and pick the one we
	// like"): every generation of the PICKED shots, a band each in toon order. Over the Viewport (main-view). The tool row:
	// the editor switch (V), Compare · n, Show rejected · n, the zoom, + (files for the current shot – variants-tools). A
	// click in a band = its shot current (a card: the current card too; Ctrl / ⌘ + click = in / out of the selection; a
	// strip: a look into it); the board scrolls to the current band when the shot changes from elsewhere. Keys on the
	// focused board: ← → ↑ ↓ (across bands; strips and folded bands skipped), Enter chosen, R review, X reject / back, I
	// improve on this, + / − / 0 zoom; C Compare, Esc closes a look, then Compare, then the selection. COMPARE (§3.4): the
	// selection right of the bands, a Divider between (45 % by default, remembered); open, a click on a card fills its
	// first empty side, else replaces the last; Space plays its clock. State: StoryboardPage.board; layout: storyboard-
	// board-layout.ts; pan / zoom: storyboard-board-view.ts.
	import { tick, untrack, type Snippet } from 'svelte';
	import { Divider, resizable } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { toast } from '$lib/toasts.ts';
	import { api } from '$lib/api.ts';
	import { bandFacts, bandLayout, lookKeys, neighbour, type BandLayout, type BoardNode, type CardAt, type Dir } from '$lib/editor/storyboard-board-layout.ts';
	import { fitZoom, zoomStep } from '$lib/editor/storyboard-board-zoom.ts';
	import { panZoom, remPx, zoomAt } from '$lib/editor/storyboard-board-view.ts';
	import { currentCard } from '$lib/editor/storyboard-audition.ts';
	import { lettersOf, type Side } from '$lib/editor/storyboard-compare.ts';
	import { recordUpload, takenNames } from '$lib/editor/storyboard-board-files.ts';
	import type { StoryboardPage } from '$lib/editor/storyboard-page.svelte.ts';
	import VariantsTools from '$lib/components/storyboard/variants-tools.svelte';
	import VariantsBand from '$lib/components/storyboard/variants-band.svelte';
	import ComparePanel from '$lib/components/storyboard/compare-panel.svelte';

	let { sb, lead }: { sb: StoryboardPage; lead: Snippet } = $props();
	const board = $derived(sb.board);
	let scroller = $state<HTMLDivElement>();

	const sources = $derived(
		sb.picked.map((shot) => {
			const entry = sb.entries.find((e) => e.id === shot) ?? null;
			return { shot, entry, takes: (entry?.shot.takes?.takes ?? []).map((t) => t.id), records: sb.generations.filter((g) => g.shot === shot) };
		})
	);
	type Source = (typeof sources)[number];
	const layoutAt = (s: Source, zoom: number): BandLayout =>
		bandLayout({ shot: s.shot, takes: s.takes, records: s.records, aspect: s.entry?.shot.aspect, zoom, showRejected: board.showRejected, open: board.look?.shot === s.shot ? board.look.keys : [] });
	const bands = $derived(sources.map((s) => ({ ...s, folded: !!board.folded[s.shot], layout: layoutAt(s, board.zoom) })));
	const rejected = $derived(sources.reduce((n, s) => n + bandFacts(s.records).rejected, 0));
	$effect(() => {
		if (!rejected) untrack(() => (board.showRejected = false));
	});
	const bandOf = (shot: string | null | undefined) => bands.find((b) => b.shot === shot);
	/** the card (the tree's – also one folded into a strip) */
	const nodeOf = (at: Side | null | undefined): BoardNode | null => (at ? (bandOf(at.shot)?.layout.tree.get(at.key) ?? null) : null);
	/** the current card: the last clicked in the current shot's band – else its chosen (the video first), else its render */
	const cur = $derived.by((): CardAt | null => {
		const band = bandOf(sb.current);
		if (!band || !sb.current) return null;
		return currentCard({ shot: sb.current, current: board.current, records: band.records, has: (k) => band.layout.tree.has(k) }) ?? { shot: sb.current, key: band.layout.nodes[0].key };
	});
	$effect(() => {
		board.alive = (s) => !!nodeOf(s);
	});
	const sel = $derived(board.selection);
	const letters = $derived(lettersOf(sel));

	// Compare: its width (45 % of the main by default, at least 27rem, the bands keep 18rem; remembered in rem)
	let split = $state<HTMLDivElement>();
	const room = () => split?.clientWidth ?? innerWidth;
	const size = resizable({ storageKey: 'scutoons_creator:storyboard:compare', edge: 'left', anchor: () => split?.getBoundingClientRect().right ?? innerWidth, min: () => 18 * remPx(), normal: () => Math.max(27 * remPx(), 0.45 * room()), max: () => Math.max(18 * remPx(), room() - 18 * remPx()) });
	size.restore();
	const cmpWidth = $derived(`min(${size.width != null ? `${(size.width / remPx()).toFixed(2)}rem` : 'max(27rem, 45%)'}, calc(100% - 18rem))`);
	const sides = $derived(sel.map((s) => (s ? { ...s, node: nodeOf(s), aspect: bandOf(s.shot)?.entry?.shot.aspect ?? null } : null)));
	let pane = $state<{ space: () => void; focus: () => void }>();
	const compare = () => board.toggleCompare(nodeOf, cur);
	const button = () => {
		compare();
		if (board.compare) void tick().then(() => pane?.focus());
	};
	function closed() {
		board.closeCompare();
		void tick().then(() => cardEl(cur)?.focus({ preventScroll: true }));
	}

	// the board scrolls to the current band when the current shot changes from elsewhere – not after a click in a band
	let mine: string | null = null;
	let shown: string | null = null;
	$effect(() => {
		const id = sb.current;
		untrack(() => {
			if (id === shown) return;
			shown = id;
			if (board.look && board.look.shot !== id) board.look = null;
			const own = id === mine;
			mine = null;
			if (!own && id) void tick().then(() => scroller?.querySelector(`[data-band="${CSS.escape(id)}"]`)?.scrollIntoView({ block: 'start' }));
		});
	});
	function makeCurrent(shot: string) {
		if (shot === sb.current) return;
		mine = shot;
		sb.focus(shot);
	}
	const cardEl = (at: CardAt | null) => (at ? scroller?.querySelector<HTMLElement>(`[data-key="${CSS.escape(at.key)}"], [data-keys~="${CSS.escape(at.key)}"]`) : null);

	function pick(shot: string, node: BoardNode, e: MouseEvent) {
		e.stopPropagation();
		if (e.ctrlKey || e.metaKey) {
			if (node.kind === 'strip') return void toast(m['storyboard.board_rejected_out']({ name: '' }));
			return board.toggleSelected({ shot, key: node.key }, node);
		}
		if (node.kind === 'strip') {
			board.look = { shot, keys: lookKeysOf(shot, node.key) };
			board.current = { shot, key: node.key };
		} else board.setCurrent({ shot, key: node.key });
		if (board.compare && node.kind !== 'strip') board.compareClick({ shot, key: node.key }, node);
		makeCurrent(shot);
		if (node.kind === 'strip') void tick().then(() => cardEl({ shot, key: node.key })?.focus({ preventScroll: true }));
	}
	const lookKeysOf = (shot: string, key: string) => (bandOf(shot) ? lookKeys(bandOf(shot)!.layout, key) : [key]);

	async function go(to: CardAt) {
		board.setCurrent(to);
		makeCurrent(to.shot);
		await tick();
		const el = cardEl(to);
		el?.focus({ preventScroll: true });
		el?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
	}

	const setZoom = (z: number) => (board.zoom = z);
	function zoomKey(dir: number) {
		const el = cardEl(cur)?.querySelector('.pic') ?? cardEl(cur);
		const r = el?.getBoundingClientRect();
		const v = scroller?.getBoundingClientRect();
		const inView = r && v && r.right > v.left && r.left < v.right && r.bottom > v.top && r.top < v.bottom;
		void zoomAt(scroller, zoomStep(board.zoom, dir), board.zoom, setZoom, inView ? r.left + r.width / 2 : undefined, inView ? r.top + r.height / 2 : undefined);
	}
	async function fit() {
		const open = bands.filter((b) => !b.folded);
		if (!scroller || !open.length) return;
		board.zoom = fitZoom((z) => Math.max(...open.map((s) => layoutAt(s, z).width)), scroller.clientWidth / remPx());
		await tick();
		scroller.scrollLeft = 0;
		cardEl(cur)?.scrollIntoView({ block: 'nearest', inline: 'nearest' });
	}
	async function refocus(done: unknown) {
		await done;
		await tick();
		const at = document.activeElement;
		if (!at || at === document.body || !at.isConnected) cardEl(cur)?.focus({ preventScroll: true });
	}

	/** files for a shot: one source card each (the member's folder, else the work storage) */
	async function upload(shot: string | null, files: File[]) {
		if (!shot) return;
		const at = { toon: sb.toon, chapter: sb.chapter, shot };
		const ready = sb.workspaceState === 'ready';
		const deps = { api, machine: sb.machine, workspace: ready ? sb.workspace : null, taken: () => takenNames(api, at, sb.generations) };
		let added = 0;
		for (const file of files) {
			try {
				sb.upsert(await recordUpload(deps, at, file, bandOf(shot)?.entry?.shot.aspect));
				added++;
			} catch (e) {
				toast(m['storyboard.board_not_added']({ file: file.name, error: e instanceof Error ? e.message : String(e) }), { type: 'error' });
			}
		}
		if (ready && added) await sb.refreshLocal();
		if (added) toast(m['storyboard.board_added']({ n: added, shot }), { type: 'success' });
	}
	const carriesFiles = (e: DragEvent) => [...(e.dataTransfer?.types ?? [])].includes('Files');
	function nowhere(e: DragEvent) {
		if (e.defaultPrevented || !carriesFiles(e)) return;
		e.preventDefault();
		if (e.type === 'dragover' && e.dataTransfer) e.dataTransfer.dropEffect = 'none';
	}

	const DIRS: Record<string, Dir> = { ArrowLeft: 'left', ArrowRight: 'right', ArrowUp: 'up', ArrowDown: 'down' };
	function keys(e: KeyboardEvent) {
		const t = e.target as Element;
		if (e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented || t.closest?.('input, textarea, select, [contenteditable], .sc-menu .popup, .sc-menu > [aria-expanded="true"]')) return;
		if ((e.key === 'Enter' && t.closest?.('button:not(.card)')) || (e.key === ' ' && t.closest?.('button:not(.card, .strip)'))) return;
		const node = nodeOf(cur);
		const band = bandOf(cur?.shot);
		const hidden = !!band?.folded;
		if (DIRS[e.key]) {
			const to = cur && neighbour(bands, cur, DIRS[e.key]);
			if (to) void go(to);
		} else if (e.repeat && /^(Enter|r|R|x|X|i|I)$/.test(e.key)) {
			// held down: once
		} else if (e.key === 'Enter') {
			if (node && !hidden) void board.choose(node);
		} else if (e.key === 'r' || e.key === 'R') {
			if (node && !hidden) void board.review(node);
		} else if (e.key === 'x' || e.key === 'X') {
			if (node && band && !hidden) void refocus(board.reject(band.layout, node));
		} else if (e.key === 'i' || e.key === 'I') {
			if (node && cur && !hidden) void board.improve.improveOn(cur.shot, node);
		} else if (e.key === '+' || e.key === '=') zoomKey(1);
		else if (e.key === '-') zoomKey(-1);
		else if (e.key === '0') void fit();
		else if (e.key === ' ' && board.compare) {
			if (!e.repeat) pane?.space();
		} else return;
		e.preventDefault();
	}
	function outer(e: KeyboardEvent) {
		const t = e.target as Element;
		if (e.ctrlKey || e.metaKey || e.altKey || e.defaultPrevented || e.repeat || t.closest?.('.compare, input, textarea, select, [contenteditable], .sc-menu .popup, .sc-menu > [aria-expanded="true"]')) return;
		if (e.key === 'c' || e.key === 'C') compare();
		else if (e.key === 'Escape' && board.look) void refocus((board.look = null));
		else if (e.key === 'Escape' && board.compare) closed();
		else if (e.key === 'Escape' && board.selected.length) board.selected = [];
		else return;
		e.preventDefault();
	}
</script>

<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
<section class="board" aria-label={m['storyboard.editor_variants']()} onkeydown={outer} data-testid="variants-canvas">
	<VariantsTools {sb} {lead} {rejected} selected={sel.filter(Boolean).length} oncompare={button} onzoom={(z) => zoomAt(scroller, z, board.zoom, setZoom)} onfiles={(files) => upload(sb.current, files)} />
	<div class="split" bind:this={split}>
		<!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
		<div class="scroll" class:panning={board.panning} bind:this={scroller} data-board-scroll role="group" aria-label={m['storyboard.board_label']()} onkeydown={keys} ondragover={nowhere} ondrop={nowhere} use:panZoom={{ zoom: () => board.zoom, set: setZoom, panning: (on) => (board.panning = on) }}>
			{#if !bands.length}
				<p class="sc-empty">{m['storyboard.no_shot']()}</p>
			{:else}
				<div class="bands">
					{#each bands as band (band.shot)}
						<VariantsBand {sb} {band} current={band.shot === sb.current} cur={cur?.shot === band.shot ? cur.key : null} {letters} onpick={(node, e) => pick(band.shot, node, e)} onband={() => makeCurrent(band.shot)} onfold={() => board.toggleFold(band.shot)} ondrop={(files) => upload(band.shot, files)} />
					{/each}
				</div>
			{/if}
		</div>
		{#if board.compare}
			<Divider {size} edge="left" at={cmpWidth} label={m['storyboard.compare_resize']()} />
			<ComparePanel bind:this={pane} {sb} {sides} width={cmpWidth} onclose={closed} />
		{/if}
	</div>
</section>

<style>
	.board {
		position: absolute;
		inset: 0;
		z-index: 2;
		display: flex;
		flex-direction: column;
		min-height: 0;
		background: var(--paper);
	}
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
	.scroll.panning {
		cursor: grabbing;
		user-select: none;
	}
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
