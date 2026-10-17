<script lang="ts" module>
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

	/** Verovio scales in percent; the steps − / + walk through */
	const ZOOMS = [20, 25, 30, 35, 40, 45, 50, 60, 70, 85, 100];
	const DEFAULT_ZOOM = 40;
	/** a phone (narrower than 40rem) starts smaller: a few bars per line instead of one */
	const PHONE_ZOOM = 30;
	/** the reader's last zoom on this device (a convenience – RULES "localStorage") */
	const ZOOM_KEY = 'scutoons_score_zoom';

	function startZoom(): number {
		try {
			const stored = Number(localStorage.getItem(ZOOM_KEY));
			if (ZOOMS.includes(stored)) return stored;
		} catch {
			/* no storage: the default */
		}
		const rem = parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
		return window.innerWidth / rem < 40 ? PHONE_ZOOM : DEFAULT_ZOOM;
	}
	/** the MusicXML the shared toolkit holds */
	let held: string | null = null;
	/** the look it was loaded with (a font change needs a new load) */
	let heldLook: unknown = null;
</script>

<script lang="ts">
	import { onDestroy, tick, untrack, type Snippet } from 'svelte';
	import Swiper from 'swiper';
	import 'swiper/css';
	import { Icon, IconButton } from '@scutoons/ui';
	import { cleanMusicXml, loadToolkit, type ScoreToolkit } from './verovio.ts';
	import ScoreAge from './score-age.svelte';
	import ScoreBoil from './score-boil.svelte';
	import type { ScoreLook } from './score-looks.ts';
	import './score-looks.css';
	import { ScoreSynth, notesFromTimemap, type ScoreNote, type TimemapEntry } from './synth.ts';

	/**
	 * The score view (research/SCORES.md §4): a full-screen sheet of music rendered from MusicXML by Verovio. Pages are
	 * the size of the view – a page turn never scrolls; − / + re-breaks the lines and pages at the new size and keeps the
	 * bar the reader was at. Pages are slides of the app's Swiper (the horizontal reader's) – a swipe turns them; so do
	 * ← / → / PageUp / PageDown / Space (Bluetooth page-turn pedals send these); + / − zoom, Escape closes. A page is
	 * rendered when it comes near (the current one ± 1). The PDF (the print file) is a plain download.
	 * Play (owner 2026-10-14) sounds the file's notes on a small synth from the current page on, lights the notes that
	 * sound and turns the pages along; while it plays the sheet is an echo pad – a finger sliding over it sets the delay
	 * (x: echo time and length, y: how much of it – the top all echo), lifting it glides back to dry (score/synth.ts).
	 * `look` (score-looks.ts): another music font, pen, colours and animations – a draft in the lab.
	 */
	let {
		src,
		pdf = null,
		title,
		labels = DEFAULT_SCORE_LABELS,
		age = null,
		look = null,
		decor,
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
		/** drawn over every page (a look's frame, figures …), given the page number */
		decor?: Snippet<[number]>;
		onclose: () => void;
	} = $props();

	let sheet = $state<HTMLElement>();
	let width = $state(0);
	let height = $state(0);
	let zoom = $state(typeof window === 'undefined' ? DEFAULT_ZOOM : startZoom());
	let page = $state(1);
	let pages = $state(0);
	/** the rendered pages (SVG by page number), cleared by every new layout */
	let rendered = $state<Record<number, string>>({});
	let slides = $state<HTMLElement>();
	let swiper: Swiper | null = null;
	let status = $state<'loading' | 'ready' | 'failed'>('loading');
	let toolkit = $state<ScoreToolkit | null>(null);
	let xml = $state<string | null>(null);

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
			layout(toolkit, xml, width, height, zoom, look);
		} catch {
			// the renderer gave up on this file: say so instead of loading forever
			held = null;
			status = 'failed';
		}
	});

	/** Verovio's page units at `zoom` % = the sheet's pixels (its tutorial's way to fit a page to a box) */
	function layout(tk: ScoreToolkit, data: string, w: number, h: number, scale: number, style: ScoreLook | null) {
		const anchor = untrack(() => (held === data ? firstMeasure() : null));
		tk.resetOptions();
		tk.setOptions({
			...style?.options,
			pageWidth: Math.round((w * 100) / scale),
			pageHeight: Math.round((h * 100) / scale),
			scale,
			adjustPageHeight: false,
			breaks: 'auto',
			header: 'auto',
			footer: 'none',
			pageMarginTop: margin(style?.margin?.top, scale),
			pageMarginBottom: margin(style?.margin?.bottom, scale),
			pageMarginLeft: margin(style?.margin?.side, scale),
			pageMarginRight: margin(style?.margin?.side, scale)
		});
		// the toolkit is shared: load the data again when another view loaded its own (or another look) since
		if (held !== data || heldLook !== style) {
			tk.loadData(data);
			held = data;
			heldLook = style;
			// a new load may give the notes new ids
			notes = null;
		} else tk.redoLayout();
		pages = tk.getPageCount();
		const at = anchor ? tk.getPageWithElement(anchor) : 1;
		rendered = {};
		status = 'ready';
		untrack(() => show(at > 0 ? at : 1, true));
	}

	/** a look's margin (rem) in Verovio's units at `scale` %; without one Verovio's 40 */
	function margin(rem: number | undefined, scale: number): number {
		if (rem === undefined) return 40;
		const px = rem * (parseFloat(getComputedStyle(document.documentElement).fontSize) || 16);
		return Math.round((px * 100) / scale);
	}

	/** the first bar of the page the reader is on (the anchor across a new layout) */
	function firstMeasure(): string | null {
		return sheet?.querySelector(`[data-page="${page}"] g.measure`)?.id ?? null;
	}

	/** go to page `n` (rendering it and its neighbours); `jump` = no slide animation (a new layout) */
	async function show(n: number, jump = false) {
		if (!toolkit || status !== 'ready') return;
		page = Math.min(Math.max(1, n), Math.max(1, pages));
		const next = { ...rendered };
		for (const p of [page - 1, page, page + 1]) if (p >= 1 && p <= pages && !next[p]) next[p] = toolkit.renderToSVG(p);
		rendered = next;
		await tick();
		if (!swiper) return;
		if (jump) swiper.update();
		if (swiper.activeIndex !== page - 1) swiper.slideTo(page - 1, jump ? 0 : undefined);
	}

	const turn = (by: number) => show(page + by);

	// the pages as Swiper slides
	$effect(() => {
		if (!slides) return;
		const instance = new Swiper(slides, { slidesPerView: 1, initialSlide: untrack(() => page - 1) });
		instance.on('slideChange', () => {
			if (instance.activeIndex !== page - 1) show(instance.activeIndex + 1);
		});
		swiper = instance;
		return () => {
			instance.destroy();
			swiper = null;
		};
	});

	function step(by: number) {
		const i = ZOOMS.indexOf(zoom);
		zoom = ZOOMS[Math.min(Math.max(0, (i < 0 ? ZOOMS.indexOf(DEFAULT_ZOOM) : i) + by), ZOOMS.length - 1)];
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
		else if (e.key === 'Escape') onclose();
		else return;
		e.preventDefault();
	}

	/** the view lives at the page's top level: opened from inside a panel (the inventory) it must not be clipped,
	 *  positioned or recoloured by it */
	function portal(node: HTMLElement) {
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
		playing = true;
		if (swiper) swiper.allowTouchMove = false;
		synth.play(notes, from, stopPlaying);
		frame = requestAnimationFrame(follow);
	}

	function stopPlaying() {
		cancelAnimationFrame(frame);
		synth?.stop();
		playing = false;
		finger = null;
		if (swiper) swiper.allowTouchMove = true;
		for (const id of lit) light(id, false);
		lit.clear();
	}

	/** every frame: light the notes sounding now and turn to their page */
	function follow() {
		if (!synth || !playing) return;
		const t = synth.now();
		while (cursor < timemap.length && timemap[cursor].tstamp <= t) {
			const entry = timemap[cursor++];
			for (const id of entry.off ?? []) light(id, false);
			for (const id of entry.on ?? []) light(id, true);
			const on = entry.on?.[0];
			if (on && toolkit) {
				const at = toolkit.getPageWithElement(on);
				if (at > 0 && at !== page) show(at, reduceMotion);
			}
		}
		frame = requestAnimationFrame(follow);
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

	let restoreFocus: Element | null = typeof document !== 'undefined' ? document.activeElement : null;
	onDestroy(() => (restoreFocus as HTMLElement | null)?.focus?.());
	const pageLabel = $derived(labels.page.replace('{page}', String(page)).replace('{pages}', String(pages)));
</script>

<svelte:window {onkeydown} />

<div class="score-view" use:portal role="dialog" aria-modal="true" aria-label={title} data-testid="score-view">
	<header>
		<IconButton label={labels.close} onclick={onclose} data-testid="score-close"><Icon name="x" /></IconButton>
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
	<div class="sheet" bind:this={sheet} data-look={look?.id} data-boil={look?.boil && !reduceMotion ? "" : undefined}>
		{#if look?.boil}<ScoreBoil />{/if}
		{#if status === 'ready'}
			<div class="swiper" bind:this={slides}>
				<div class="swiper-wrapper">
					{#each { length: pages }, i (i)}
						<!-- Verovio's SVG of our own MusicXML -->
						<div class="swiper-slide" data-page={i + 1}>
							{@html rendered[i + 1] ?? ''}
							{#if age}<ScoreAge tint={age.tint} spots={age.spots} seed="{age.seed}:{i + 1}" />{/if}
							{@render decor?.(i + 1)}
						</div>
					{/each}
				</div>
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

	.swiper {
		height: 100%;
	}

	/* a system taller than the page (a big zoom on a phone) scrolls instead of being cut */
	.swiper-slide {
		position: relative;
		height: 100%;
		overflow: hidden auto;
	}

	.sheet :global(svg) {
		display: block;
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
