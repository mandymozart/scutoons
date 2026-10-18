<script lang="ts" module>
	import { DEFAULT_ZOOM, PHONE_ZOOM, ZOOMS } from './score-layout.ts';

	/** the view's texts (the app's language); English by default */
	export interface ScoreLabels {
		close: string;
		zoomIn: string;
		zoomOut: string;
		previous: string;
		next: string;
		/** "Seite {page} von {pages}" – `{page}` / `{pages}` are replaced */
		page: string;
		pdf: string;
		loading: string;
		failed: string;
		play: string;
		stop: string;
		/** the hint on the echo pad while playing */
		pad: string;
	}

	export const DEFAULT_SCORE_LABELS: ScoreLabels = {
		close: 'Close',
		zoomIn: 'Larger',
		zoomOut: 'Smaller',
		previous: 'Previous page',
		next: 'Next page',
		page: 'Page {page} of {pages}',
		pdf: 'PDF',
		loading: 'Loading the score …',
		failed: 'The score could not be loaded.',
		play: 'Play',
		stop: 'Stop',
		pad: 'Slide over the notes: echo'
	};

	/** the reader's last zoom on this device (a convenience – RULES "localStorage") */
	const ZOOM_KEY = 'scutoons_score_zoom';

	const remPx = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

	function startZoom(): number {
		try {
			const stored = Number(localStorage.getItem(ZOOM_KEY));
			if (ZOOMS.includes(stored)) return stored;
		} catch {
			/* no storage: the default */
		}
		return window.innerWidth / remPx() < 40 ? PHONE_ZOOM : DEFAULT_ZOOM;
	}
	/** the MusicXML the shared toolkit holds */
	let held: string | null = null;
	/** the look it was loaded with (a font change needs a new load) */
	let heldLook: unknown = null;
</script>

<script lang="ts">
	import { onDestroy, tick, untrack, type Snippet } from 'svelte';
	import type { CollagePiece, ScoreAt } from '@scutoons/content';
	import { CloseButton, Icon, IconButton } from '@scutoons/ui';
	import { cleanMusicXml, loadToolkit, type ScoreToolkit } from './verovio.ts';
	import ScoreAge from './score-age.svelte';
	import ScoreBoil from './score-boil.svelte';
	import ScorePieces from './score-pieces.svelte';
	import { barIndex, findBar, pageGeometry, type BarRef, type PageGeometry } from './score-anchors.ts';
	import { scoreOptions, stepZoom } from './score-layout.ts';
	import type { ScoreLook } from './score-looks.ts';
	import './score-looks.css';
	import { pageScroller, type PageScroller } from './score-pages.ts';
	import { ScoreSynth, notesFromTimemap, type ScoreNote, type TimemapEntry } from './synth.ts';

	/**
	 * The score view (research/SCORES.md §4): a full-screen sheet of music rendered from MusicXML by Verovio. Pages are
	 * the size of the view – a page turn never scrolls; − / + re-breaks the lines and pages at the new size and keeps the
	 * bar the reader was at. Pages sit on native CSS scroll-snap (score-pages.ts, owner 2026-10-17) – a swipe, a wheel or
	 * a trackpad turns them; so do ← / → / PageUp / PageDown / Space (Bluetooth page-turn pedals send these); + / − zoom,
	 * Escape closes. A page is rendered when it comes near (the current one ± 1). The PDF (the print file) is a plain
	 * download.
	 * Play (owner 2026-10-14) sounds the file's notes on a small synth from the current page on, lights the notes that
	 * sound and turns the pages along; while it plays the sheet is an echo pad – a finger sliding over it sets the delay
	 * (x: echo time and length, y: how much of it – the top all echo), lifting it glides back to dry (score/synth.ts).
	 * `look` (score-looks.ts): another music font, pen, colours and animations – a draft in the lab.
	 * Collage pieces (agents/research/COLLAGE.md §4.3): `pieces` stick to bars (score-anchors.ts) and are drawn on every
	 * rendered page (score-pieces.svelte) – behind the notes or over them, blending with the music and the paper; the
	 * music lifts to z 1 while there are any, the age lies over all (2). Without pieces the view is as before (no MEI read).
	 * `embedded` (the creator's score stage): inside its box, no portal, no window keys, no stored zoom.
	 * Over 400 lines: loading, layout, paging and playing share the toolkit, the page and the sheet – kept as one view.
	 */
	let {
		src,
		pdf = null,
		title,
		labels = DEFAULT_SCORE_LABELS,
		age = null,
		look = null,
		decor,
		pieces,
		pieceSrc,
		embedded = false,
		zoom = $bindable(typeof window === 'undefined' || embedded ? DEFAULT_ZOOM : startZoom()),
		page = $bindable(1),
		follow = null,
		onlayout,
		onclose
	}: {
		/** the MusicXML file's URL */
		src: string;
		/** the print PDF's URL */
		pdf?: string | null;
		title: string;
		labels?: ScoreLabels;
		/** an ageing book's look (`ageLook` + a seed – the book entry): tint and spots over every page */
		age?: { tint: number; spots: number; seed: string } | null;
		look?: ScoreLook | null;
		/**
		 * drawn over every page (a look's frame, figures, the stage's piece boxes …), given the page number and its
		 * geometry (null while the bars aren't read – no pieces, no `follow`, no `onlayout`); with pieces the music is at
		 * z 1 – a decor meant to lie over it brings its own z
		 */
		decor?: Snippet<[number, PageGeometry | null]>;
		/** the collage pieces of the opened file (score.collage / the part's) */
		pieces?: CollagePiece[];
		/** a piece's absolute file URL from its `src` (a catalog row's CDN-root path); none = no pieces drawn */
		pieceSrc?: (src: string) => string | null | undefined;
		/** inside the host's box (absolute, its own blend group) instead of a full-screen dialog */
		embedded?: boolean;
		/** Verovio's scale in % (ZOOMS); bind it to keep it */
		zoom?: number;
		/** the page the reader is on (bind it to read it; `follow` turns the view) */
		page?: number;
		/** a bar to keep in view: the layout's anchor instead of the page's first bar, and turned to when it changes */
		follow?: ScoreAt | null;
		/** after every layout: the file's bars (score-anchors.ts – the stage lists the pieces whose bar is missing) */
		onlayout?: (bars: BarRef[]) => void;
		/** none = no close button and no Escape */
		onclose?: () => void;
	} = $props();

	let sheet = $state<HTMLElement>();
	let width = $state(0);
	let height = $state(0);
	let pages = $state(0);
	/** the rendered pages (SVG and geometry by page number), cleared by every new layout */
	let rendered = $state.raw<Record<number, { svg: string; geometry: PageGeometry | null }>>({});
	let scroller = $state<HTMLElement>();
	let pager: PageScroller | null = null;
	let status = $state<'loading' | 'ready' | 'failed'>('loading');
	let toolkit = $state<ScoreToolkit | null>(null);
	let xml = $state<string | null>(null);
	/** the file's bars (read from the toolkit's MEI only when pieces or a host need them) and the file they belong to */
	let bars = $state.raw<BarRef[]>([]);
	let barsOf: string | null = null;
	const wantsBars = $derived(pieces !== undefined || !!follow || !!onlayout);
	const drawsPieces = $derived(!!pieceSrc && !!pieces?.length);

	$effect(() => {
		const url = src;
		status = 'loading';
		xml = null;
		let cancelled = false;
		Promise.all([loadToolkit(), fetch(url).then((r) => (r.ok ? r.text() : Promise.reject(new Error(String(r.status)))))])
			.then(([tk, text]) => {
				if (cancelled) return;
				toolkit = tk;
				xml = cleanMusicXml(text);
				notes = null;
			})
			.catch(() => !cancelled && (status = 'failed'));
		return () => (cancelled = true);
	});

	// the sheet's size: pages are exactly as big as what the reader sees
	$effect(() => {
		if (!sheet) return;
		const observer = new ResizeObserver(([e]) => {
			width = Math.floor(e.contentRect.width);
			height = Math.floor(e.contentRect.height);
		});
		observer.observe(sheet);
		return () => observer.disconnect();
	});

	// the score, a new size or zoom: lay out again, staying at the bar the page started with
	$effect(() => {
		if (!toolkit || !xml || !width || !height) return;
		try {
			layout(toolkit, xml, width, height, zoom, look, wantsBars);
		} catch {
			// the renderer gave up on this file: say so instead of loading forever
			held = null;
			status = 'failed';
		}
	});

	/** Verovio's page units at `zoom` % = the sheet's pixels (score-layout.ts) */
	function layout(tk: ScoreToolkit, data: string, w: number, h: number, scale: number, style: ScoreLook | null, withBars: boolean) {
		const first = untrack(() => (held === data ? firstMeasure() : null));
		tk.resetOptions();
		tk.setOptions(scoreOptions(w, h, scale, style, remPx()));
		// the toolkit is shared: load the data again when another view loaded its own (or another look) since
		if (held !== data || heldLook !== style) {
			tk.loadData(data);
			held = data;
			heldLook = style;
			// a new load may give the notes and bars new ids
			notes = null;
			barsOf = null;
		} else tk.redoLayout();
		if (withBars && barsOf !== data) {
			bars = barIndex(tk.getMEI());
			barsOf = data;
		}
		pages = tk.getPageCount();
		const anchor = untrack(() => (follow ? findBar(bars, follow)?.id : null)) ?? first;
		const at = anchor ? tk.getPageWithElement(anchor) : 1;
		rendered = {};
		status = 'ready';
		untrack(() => {
			show(at > 0 ? at : 1, true);
			onlayout?.(bars);
		});
	}

	// `follow` moved to another bar: turn to its page
	$effect(() => {
		const at = follow;
		if (!at || status !== 'ready') return;
		untrack(() => {
			const id = findBar(bars, at)?.id;
			const to = id && toolkit ? toolkit.getPageWithElement(id) : 0;
			if (to > 0 && to !== page) show(to);
		});
	});

	/** the first bar of the page the reader is on (the anchor across a new layout) */
	function firstMeasure(): string | null {
		return sheet?.querySelector(`[data-page="${page}"] g.measure`)?.id ?? null;
	}

	/** page `n` is the reader's: render it and its neighbours */
	function visit(n: number) {
		if (!toolkit || status !== 'ready') return;
		page = Math.min(Math.max(1, n), Math.max(1, pages));
		const next = { ...rendered };
		for (const p of [page - 1, page, page + 1]) {
			if (p < 1 || p > pages || next[p]) continue;
			const svg = toolkit.renderToSVG(p);
			next[p] = { svg, geometry: bars.length ? pageGeometry(svg, bars) : null };
		}
		rendered = next;
	}

	/** go to page `n`; `jump` = instant (a new layout, Play following the music), else a smooth turn */
	async function show(n: number, jump = false) {
		if (!toolkit || status !== 'ready') return;
		visit(n);
		await tick();
		pager?.go(page, !jump && !reduceMotion);
	}

	const turn = (by: number) => show(page + by);

	// the pages on scroll-snap: the reader's own scrolling lands on a page
	$effect(() => {
		if (!scroller) return;
		const instance = pageScroller(scroller, () => pages, (n) => n !== page && visit(n));
		pager = instance;
		return () => {
			instance.destroy();
			pager = null;
		};
	});

	function step(by: number) {
		zoom = stepZoom(zoom, by);
		if (embedded) return;
		try {
			localStorage.setItem(ZOOM_KEY, String(zoom));
		} catch {
			/* no storage: this view only */
		}
	}

	function onkeydown(e: KeyboardEvent) {
		if (e.metaKey || e.ctrlKey || e.altKey) return;
		const by = { ArrowRight: 1, PageDown: 1, ' ': 1, ArrowLeft: -1, PageUp: -1 }[e.key];
		if (by) turn(e.shiftKey && e.key === ' ' ? -1 : by);
		else if (e.key === '+' || e.key === '=') step(1);
		else if (e.key === '-') step(-1);
		else if (e.key === 'Escape' && onclose) onclose();
		else return;
		e.preventDefault();
	}

	/** the view lives at the page's top level: opened from inside a panel (the inventory) it must not be clipped,
	 *  positioned or recoloured by it */
	function portal(node: HTMLElement) {
		if (embedded) return;
		document.body.appendChild(node);
		return { destroy: () => node.remove() };
	}

	// ── playing ──
	let synth: ScoreSynth | null = null;
	let playing = $state(false);
	/** the loaded file's notes and timemap (made on the first Play) */
	let notes: ScoreNote[] | null = null;
	let timemap: TimemapEntry[] = [];
	let cursor = 0;
	let frame = 0;
	const lit = new Set<string>();
	/** the finger on the echo pad (0 … 1 of the sheet) */
	let finger = $state<{ x: number; y: number } | null>(null);
	const reduceMotion = typeof matchMedia !== 'undefined' && matchMedia('(prefers-reduced-motion: reduce)').matches;

	function togglePlay() {
		if (playing) return stopPlaying();
		const tk = toolkit;
		if (!tk || status !== 'ready' || held !== xml) return;
		if (!notes) {
			timemap = tk.renderToTimemap();
			notes = notesFromTimemap(timemap, (id) => tk.getMIDIValuesForElement(id).pitch ?? null);
		}
		// the audio starts inside this click (browsers allow sound only after a gesture)
		synth ??= new ScoreSynth();
		// from the first note on the page the reader is on
		const first = sheet?.querySelector(`[data-page="${page}"] g.note`)?.id;
		const from = page > 1 ? (notes.find((n) => n.id === first)?.at ?? 0) : 0;
		cursor = Math.max(0, timemap.findIndex((e) => e.tstamp >= from));
		// the echo pad lies over the pages: a finger plays the echo, it turns no page
		playing = true;
		synth.play(notes, from, stopPlaying);
		frame = requestAnimationFrame(keepUp);
	}

	function stopPlaying() {
		cancelAnimationFrame(frame);
		synth?.stop();
		playing = false;
		finger = null;
		for (const id of lit) light(id, false);
		lit.clear();
	}

	/** every frame: light the notes sounding now and turn to their page */
	function keepUp() {
		if (!synth || !playing) return;
		const t = synth.now();
		while (cursor < timemap.length && timemap[cursor].tstamp <= t) {
			const entry = timemap[cursor++];
			for (const id of entry.off ?? []) light(id, false);
			for (const id of entry.on ?? []) light(id, true);
			const on = entry.on?.[0];
			if (on && toolkit) {
				const at = toolkit.getPageWithElement(on);
				if (at > 0 && at !== page) show(at, true);
			}
		}
		frame = requestAnimationFrame(keepUp);
	}

	function light(id: string, on: boolean) {
		if (on) lit.add(id);
		else lit.delete(id);
		sheet?.querySelector(`#${CSS.escape(id)}`)?.classList.toggle('playing', on);
	}

	function padAt(e: PointerEvent) {
		const box = (e.currentTarget as HTMLElement).getBoundingClientRect();
		finger = { x: (e.clientX - box.left) / box.width, y: (e.clientY - box.top) / box.height };
		synth?.pad(finger.x, finger.y);
	}

	function padDown(e: PointerEvent) {
		(e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
		padAt(e);
	}

	function padUp() {
		finger = null;
		synth?.release();
	}

	onDestroy(() => {
		cancelAnimationFrame(frame);
		synth?.close();
	});

	// a dialog gives the focus back where it was (an embedded view never took it)
	let restoreFocus: Element | null = typeof document !== 'undefined' && !untrack(() => embedded) ? document.activeElement : null;
	onDestroy(() => (restoreFocus as HTMLElement | null)?.focus?.());
	const pageLabel = $derived(labels.page.replace('{page}', String(page)).replace('{pages}', String(pages)));
</script>

<svelte:window onkeydown={embedded ? undefined : onkeydown} />

<div class="score-view" class:embedded use:portal role={embedded ? 'region' : 'dialog'} aria-modal={embedded ? undefined : 'true'} aria-label={title} data-testid="score-view">
	<header>
		{#if onclose}<CloseButton label={labels.close} onclick={onclose} data-testid="score-close" />{/if}
		<h2>{title}</h2>
		<button type="button" class="play" class:on={playing} aria-label={playing ? labels.stop : labels.play} disabled={status !== 'ready'} onclick={togglePlay} data-testid="score-play">
			<Icon name={playing ? 'square' : 'play'} /><span class="wide">{playing ? labels.stop : labels.play}</span>
		</button>
		<div class="group" role="group">
			<button type="button" title={labels.zoomOut} aria-label={labels.zoomOut} disabled={zoom === ZOOMS[0]} onclick={() => step(-1)} data-testid="score-zoom-out"><Icon name="minus" /></button>
			<button type="button" title={labels.zoomIn} aria-label={labels.zoomIn} disabled={zoom === ZOOMS[ZOOMS.length - 1]} onclick={() => step(1)} data-testid="score-zoom-in"><Icon name="plus" /></button>
		</div>
		{#if pdf}
			<a class="pdf" href={pdf} download target="_blank" rel="noopener" data-testid="score-pdf"><Icon name="download" /> {labels.pdf}</a>
		{/if}
	</header>

	<!-- the sheet: its size is the page size -->
	<div class="sheet" bind:this={sheet} data-look={look?.id} data-boil={look?.boil && !reduceMotion ? "" : undefined} data-pieces={drawsPieces ? '' : undefined}>
		{#if look?.boil}<ScoreBoil />{/if}
		{#if status === 'ready'}
			<div class="pages" bind:this={scroller} data-testid="score-pages">
				{#each { length: pages }, i (i)}
					<!-- Verovio's SVG of our own MusicXML -->
					{@const leaf = rendered[i + 1]}
					<div class="leaf" data-page={i + 1}>
						{@html leaf?.svg ?? ''}
						{#if drawsPieces && leaf?.geometry}<ScorePieces pieces={pieces ?? []} geometry={leaf.geometry} pieceSrc={pieceSrc!} />{/if}
						{#if age}<ScoreAge tint={age.tint} spots={age.spots} seed="{age.seed}:{i + 1}" />{/if}
						{@render decor?.(i + 1, leaf?.geometry ?? null)}
					</div>
				{/each}
			</div>
			{#if playing}
				<!-- the echo pad: a finger sliding over the sheet plays with the delay (pointer only – an extra, the music plays without it) -->
				<div class="pad" role="presentation" onpointerdown={padDown} onpointermove={(e) => finger && padAt(e)} onpointerup={padUp} onpointercancel={padUp} data-testid="score-pad">
					{#if finger}
						<span class="ring" style="left: {finger.x * 100}%; top: {finger.y * 100}%; --wet: {1 - finger.y}"></span>
					{:else}
						<span class="hint">{labels.pad}</span>
					{/if}
				</div>
			{/if}
		{:else}
			<p class="note" role="status">{status === 'failed' ? labels.failed : labels.loading}</p>
		{/if}
	</div>

	{#if pages > 1}
		<footer>
			<IconButton label={labels.previous} disabled={page === 1} onclick={() => turn(-1)} data-testid="score-prev"><Icon name="chevron-left" /></IconButton>
			<span class="page" aria-live="polite" data-testid="score-page">{pageLabel}</span>
			<IconButton label={labels.next} disabled={page === pages} onclick={() => turn(1)} data-testid="score-next"><Icon name="chevron-right" /></IconButton>
		</footer>
	{/if}
</div>

<style>
	.score-view {
		position: fixed;
		inset: 0;
		/* above the viewer's inventory (items host 2030–2050), below its notices; an app may set --score-view-z */
		z-index: var(--score-view-z, 2080);
		display: flex;
		flex-direction: column;
		background: var(--paper);
		color: var(--ink);
	}

	/* the creator's score stage: inside its box, the blend group of its own */
	.score-view.embedded {
		position: absolute;
		z-index: auto;
		isolation: isolate;
	}

	header,
	footer {
		display: flex;
		align-items: center;
		gap: var(--space-075);
		padding: var(--space-075) var(--space-1);
		flex: none;
	}

	header {
		border-bottom: var(--border-width-regular) solid var(--ink);
	}

	footer {
		justify-content: center;
		border-top: var(--border-width-regular) solid var(--ink);
	}

	h2 {
		flex: 1;
		min-width: 0;
		margin: 0;
		font-size: var(--type-body-size);
		line-height: var(--type-body-line);
		font-weight: var(--font-weight-regular);
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}

	/* grouped controls are square (DESIGN "Round vs square") */
	.group {
		display: flex;
	}

	.group button,
	.play,
	.pdf {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-05);
		box-sizing: border-box;
		height: var(--control-height);
		min-width: var(--control-height);
		padding: 0 var(--space-075);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: var(--radius-none);
		font: inherit;
		text-decoration: none;
		cursor: pointer;
	}

	.group button + button {
		border-left: none;
	}

	.group button:hover:not(:disabled),
	.play:hover:not(:disabled),
	.play.on,
	.pdf:hover {
		background: var(--primary-cyan);
	}

	.group button:disabled,
	.play:disabled {
		color: var(--text-muted);
		cursor: default;
	}

	/* music is printed on paper: black on white in every theme */
	.sheet {
		position: relative;
		flex: 1;
		min-height: 0;
		overflow: hidden;
		user-select: none;
		background: #ffffff;
		color: #000000;
	}

	/* one page per snap point; every turn stops at the next page (no fling past it); the swipe stays in the view */
	.pages {
		display: flex;
		height: 100%;
		overflow: auto hidden;
		scroll-snap-type: x mandatory;
		overscroll-behavior-x: contain;
		scrollbar-width: none;
	}

	.pages::-webkit-scrollbar {
		display: none;
	}

	/* a system taller than the page (a big zoom on a phone) scrolls instead of being cut */
	.leaf {
		position: relative;
		flex: 0 0 100%;
		height: 100%;
		overflow: hidden auto;
		scroll-snap-align: start;
		scroll-snap-stop: always;
	}

	.sheet :global(svg) {
		display: block;
	}

	/* collage pieces (score-pieces.svelte): behind the notes z 0, the music z 1, over it z 2, the age over all (2,
	   later); `.pages` and `.leaf` stay no group, so the pieces blend with the music and the paper */
	.sheet[data-pieces] .leaf > :global(svg) {
		position: relative;
		z-index: 1;
	}

	.sheet[data-pieces] .leaf > :global(.score-age) {
		z-index: 2;
	}

	/* the notes sounding now */
	.sheet :global(g.note.playing) {
		fill: var(--primary-cyan);
		color: var(--primary-cyan);
	}

	.pad {
		position: absolute;
		inset: 0;
		z-index: 2;
		touch-action: none;
		cursor: crosshair;
	}

	.hint {
		position: absolute;
		left: 50%;
		bottom: var(--space-1);
		transform: translateX(-50%);
		padding: var(--space-025) var(--space-075);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		white-space: nowrap;
		pointer-events: none;
	}

	/* the finger: the more echo (higher up), the bigger and fuller the ring */
	.ring {
		position: absolute;
		width: calc(2.5rem + 5rem * var(--wet));
		aspect-ratio: 1;
		transform: translate(-50%, -50%);
		border: var(--border-width-regular) solid var(--ink);
		border-radius: 50%;
		background: color-mix(in srgb, var(--primary-cyan) calc(var(--wet) * 40%), transparent);
		pointer-events: none;
	}

	@media (max-width: 40rem) {
		.wide {
			display: none;
		}
	}

	.note {
		margin: var(--space-3) var(--space-1);
		text-align: center;
		color: var(--text-muted);
	}

	.page {
		min-width: 8rem;
		text-align: center;
		font-variant-numeric: tabular-nums;
	}
</style>
