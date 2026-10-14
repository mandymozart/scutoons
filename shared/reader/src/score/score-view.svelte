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
		failed: 'The score could not be loaded.'
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
</script>

<script lang="ts">
	import { onDestroy, tick, untrack } from 'svelte';
	import Swiper from 'swiper';
	import 'swiper/css';
	import { Icon, IconButton } from '@scutoons/ui';
	import { cleanMusicXml, loadToolkit, type ScoreToolkit } from './verovio.ts';
	import ScoreAge from './score-age.svelte';

	/**
	 * The score view (research/SCORES.md §4): a full-screen sheet of music rendered from MusicXML by Verovio. Pages are
	 * the size of the view – a page turn never scrolls; − / + re-breaks the lines and pages at the new size and keeps the
	 * bar the reader was at. Pages are slides of the app's Swiper (the horizontal reader's) – a swipe turns them; so do
	 * ← / → / PageUp / PageDown / Space (Bluetooth page-turn pedals send these); + / − zoom, Escape closes. A page is
	 * rendered when it comes near (the current one ± 1). The PDF (the print file) is a plain download.
	 */
	let {
		src,
		pdf = null,
		title,
		labels = DEFAULT_SCORE_LABELS,
		age = null,
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
			layout(toolkit, xml, width, height, zoom);
		} catch {
			// the renderer gave up on this file: say so instead of loading forever
			held = null;
			status = 'failed';
		}
	});

	/** Verovio's page units at `zoom` % = the sheet's pixels (its tutorial's way to fit a page to a box) */
	function layout(tk: ScoreToolkit, data: string, w: number, h: number, scale: number) {
		const anchor = untrack(() => (held === data ? firstMeasure() : null));
		tk.setOptions({
			pageWidth: Math.round((w * 100) / scale),
			pageHeight: Math.round((h * 100) / scale),
			scale,
			adjustPageHeight: false,
			breaks: 'auto',
			header: 'auto',
			footer: 'none',
			pageMarginTop: 40,
			pageMarginBottom: 40,
			pageMarginLeft: 40,
			pageMarginRight: 40
		});
		// the toolkit is shared: load the data again when another view loaded its own since
		if (held !== data) {
			tk.loadData(data);
			held = data;
		} else tk.redoLayout();
		pages = tk.getPageCount();
		const at = anchor ? tk.getPageWithElement(anchor) : 1;
		rendered = {};
		status = 'ready';
		untrack(() => show(at > 0 ? at : 1, true));
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

	let restoreFocus: Element | null = typeof document !== 'undefined' ? document.activeElement : null;
	onDestroy(() => (restoreFocus as HTMLElement | null)?.focus?.());
	const pageLabel = $derived(labels.page.replace('{page}', String(page)).replace('{pages}', String(pages)));
</script>

<svelte:window {onkeydown} />

<div class="score-view" use:portal role="dialog" aria-modal="true" aria-label={title} data-testid="score-view">
	<header>
		<IconButton label={labels.close} onclick={onclose} data-testid="score-close"><Icon name="x" /></IconButton>
		<h2>{title}</h2>
		<div class="group" role="group">
			<button type="button" title={labels.zoomOut} aria-label={labels.zoomOut} disabled={zoom === ZOOMS[0]} onclick={() => step(-1)} data-testid="score-zoom-out"><Icon name="minus" /></button>
			<button type="button" title={labels.zoomIn} aria-label={labels.zoomIn} disabled={zoom === ZOOMS[ZOOMS.length - 1]} onclick={() => step(1)} data-testid="score-zoom-in"><Icon name="plus" /></button>
		</div>
		{#if pdf}
			<a class="pdf" href={pdf} download target="_blank" rel="noopener" data-testid="score-pdf"><Icon name="download" /> {labels.pdf}</a>
		{/if}
	</header>

	<!-- the sheet: its size is the page size -->
	<div class="sheet" bind:this={sheet}>
		{#if status === 'ready'}
			<div class="swiper" bind:this={slides}>
				<div class="swiper-wrapper">
					{#each { length: pages }, i (i)}
						<!-- Verovio's SVG of our own MusicXML -->
						<div class="swiper-slide" data-page={i + 1}>
							{@html rendered[i + 1] ?? ''}
							{#if age}<ScoreAge tint={age.tint} spots={age.spots} seed="{age.seed}:{i + 1}" />{/if}
						</div>
					{/each}
				</div>
			</div>
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
	.pdf:hover {
		background: var(--primary-cyan);
	}

	.group button:disabled {
		color: var(--text-muted);
		cursor: default;
	}

	/* music is printed on paper: black on white in every theme */
	.sheet {
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
