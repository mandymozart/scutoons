<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { Icon } from '@scutoons/ui';
	import { CoverArt } from '@scutoons/reader';
	import {
		COVER_ALIGNS,
		COVER_COLORS,
		COVER_COLOR_VALUES,
		COVER_LIMITS,
		COVER_TEXT_SOURCES,
		coverLetteringOn,
		coverLines,
		coverLineText,
		defaultCoverLines,
		type Cover,
		type CoverFormat,
		type CoverLettering,
		type CoverLine,
		type CoverTexts,
		type Path
	} from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The cover editor (owner 2026-10-02, agents/DESIGN.md "Cover editor"): the text-free art with its lettering lines
	 * drawn over it – drag to move, handles for width / size / angle, number fields, add / remove / reorder lines, the
	 * text source per line – a preview language, and every language "as readers see it". One component for any doc
	 * holding a `Cover` at `path` (the toon entry today; a season's cover would work the same). It writes only
	 * `<path>.lettering` (`setShared`: the toon entry has one document, a season every locale).
	 * The square format only: the reader shows no portrait cover (owner 2026-10-02 "portrait I think we are not using").
	 */
	let {
		doc,
		path = ['cover'],
		toon,
		languages,
		source,
		texts,
		format = 'square'
	}: {
		doc: ContentDoc;
		/** where the `Cover` is in the doc */
		path?: Path;
		toon: string;
		/** the toon's languages (source first) */
		languages: string[];
		source: string;
		/** the title texts of a language (toon settings + season, missing ones fall back to the source) */
		texts: (locale: string) => CoverTexts;
		format?: CoverFormat;
	} = $props();

	const cover = $derived(doc.get(doc.primary, path) as Cover | undefined);
	const lettering = $derived(cover?.lettering);
	const mode = $derived<'off' | 'default' | 'custom'>(!coverLetteringOn(lettering) ? 'off' : coverLines(lettering, format) ? 'custom' : 'default');
	/** the lines on the canvas: the format's own, or the default layout's (editing them makes them the format's own) */
	const lines = $derived<CoverLine[]>(mode === 'off' ? [] : (coverLines(lettering, format) ?? defaultCoverLines(format)));
	const image = $derived(cover?.[format] ? asset(`${toon}/${cover[format]}`) : '');

	// the preview language: the top bar's i18n choice when it is one of the toon's languages, else the source
	let previewLang = $state('');
	$effect(() => {
		const chosen = textLanguage.chosen(languages);
		untrack(() => (previewLang = chosen ?? (languages.includes(previewLang) ? previewLang : source)));
	});
	const previewTexts = $derived(texts(previewLang || source));

	let selected = $state<number | null>(null);
	$effect(() => {
		if (selected !== null && selected >= lines.length) selected = lines.length ? lines.length - 1 : null;
	});
	const line = $derived(selected !== null ? lines[selected] : undefined);

	// ── writing ─────────────────────────────────────────────────────────────────────────────────────────────────
	const round = (v: number) => Math.round(v * 10) / 10;
	const clamp = (v: number, key: keyof typeof COVER_LIMITS) => Math.min(COVER_LIMITS[key].max, Math.max(COVER_LIMITS[key].min, v));

	function writeLettering(next: boolean | CoverLettering | undefined, key: string) {
		doc.setShared([...path, 'lettering'], next, key);
	}

	/** the format's lines become `next` (other formats' layouts are kept) */
	function writeLines(next: CoverLine[], key: string) {
		const base = lettering && typeof lettering === 'object' ? $state.snapshot(lettering) as CoverLettering : {};
		writeLettering({ ...base, [format]: next }, key);
	}

	function editLine(index: number, change: Partial<CoverLine>, key: string) {
		const next = $state.snapshot(lines) as CoverLine[];
		const merged = { ...next[index], ...change };
		for (const k of Object.keys(merged) as (keyof CoverLine)[]) if (merged[k] === undefined) delete merged[k];
		next[index] = merged;
		writeLines(next, key);
	}

	function setMode(next: 'off' | 'default' | 'custom') {
		if (next === mode) return;
		if (next === 'off') writeLettering(undefined, 'cover-mode');
		else if (next === 'default') {
			// keep the other formats' layouts; without any, `true` (the plain default)
			const base = lettering && typeof lettering === 'object' ? ($state.snapshot(lettering) as CoverLettering) : {};
			delete base[format];
			writeLettering(Object.keys(base).length ? base : true, 'cover-mode');
		} else writeLines(defaultCoverLines(format), 'cover-mode');
		selected = null;
	}

	function addLine() {
		const next = [...($state.snapshot(lines) as CoverLine[]), { text: 'custom', custom: {}, x: 50, y: 60, size: 8, maxWidth: 70, rotate: -5, skew: -4 } as CoverLine];
		writeLines(next, 'cover-add');
		selected = next.length - 1;
		tick().then(() => document.querySelector<HTMLInputElement>('[data-testid="cover-custom-text"] input')?.focus());
	}

	function removeLine(index: number) {
		const next = ($state.snapshot(lines) as CoverLine[]).filter((_, i) => i !== index);
		writeLines(next, 'cover-remove');
		selected = next.length ? Math.min(index, next.length - 1) : null;
	}

	function moveLine(index: number, by: number) {
		const next = $state.snapshot(lines) as CoverLine[];
		const to = index + by;
		if (to < 0 || to >= next.length) return;
		[next[index], next[to]] = [next[to], next[index]];
		writeLines(next, 'cover-order');
		selected = to;
	}

	function setText(index: number, text: CoverLine['text']) {
		const current = lines[index];
		const change: Partial<CoverLine> = { text, custom: text === 'custom' ? (current.custom ?? {}) : undefined };
		editLine(index, change, 'cover-text');
	}

	function setCustom(index: number, locale: string, value: string) {
		const custom = { ...($state.snapshot(lines[index].custom) ?? {}), [locale]: value };
		editLine(index, { custom }, `cover-custom-${index}-${locale}`);
	}

	function setNumber(index: number, key: keyof typeof COVER_LIMITS, raw: string) {
		const n = Number(raw);
		if (raw.trim() === '' || !Number.isFinite(n)) {
			if (key === 'rotate' || key === 'skew') editLine(index, { [key]: undefined }, `cover-${key}-${index}`);
			return;
		}
		editLine(index, { [key]: round(clamp(n, key)) }, `cover-${key}-${index}`);
	}

	// ── canvas: drag to move, handles for width / size / angle ──────────────────────────────────────────────────
	let stage = $state<HTMLDivElement | null>(null);
	/** the lines' rendered heights in % of the cover's width (cqw), measured from the reader's own DOM */
	let heights = $state<number[]>([]);
	let guide = $state(false);

	async function measure() {
		await tick();
		if (!stage) return;
		const width = stage.clientWidth || 1;
		heights = lines.map((_, i) => {
			const text = stage!.querySelector<HTMLElement>(`[data-line="${i}"] .text`);
			return text ? (text.offsetHeight / width) * 100 : 0;
		});
	}
	$effect(() => {
		void JSON.stringify(lines);
		void previewTexts.toon;
		void previewTexts.season;
		void previewLang;
		measure();
		document.fonts?.ready.then(measure).catch(() => {});
	});

	type Drag = { kind: 'move' | 'width' | 'size' | 'rotate'; index: number; startX: number; startY: number; start: CoverLine; cx: number; cy: number; startDist: number };
	let drag: Drag | null = null;

	function startDrag(event: PointerEvent, index: number, kind: Drag['kind']) {
		if (event.button !== 0 || !stage) return;
		event.preventDefault();
		event.stopPropagation();
		// the default layout becomes the format's own as soon as it is touched
		if (mode === 'default') writeLines(defaultCoverLines(format), 'cover-mode');
		selected = index;
		const rect = stage.getBoundingClientRect();
		const start = $state.snapshot(lines[index]) as CoverLine;
		const cx = rect.left + (start.x / 100) * rect.width;
		const cy = rect.top + (start.y / 100) * rect.height;
		drag = { kind, index, startX: event.clientX, startY: event.clientY, start, cx, cy, startDist: Math.hypot(event.clientX - cx, event.clientY - cy) || 1 };
		(event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
		(event.currentTarget as HTMLElement).closest<HTMLElement>('.box')?.focus({ preventScroll: true });
	}

	function onDrag(event: PointerEvent) {
		if (!drag || !stage) return;
		const rect = stage.getBoundingClientRect();
		const { kind, index, start } = drag;
		const key = `cover-drag-${kind}-${index}`;
		if (kind === 'move') {
			let x = clamp(start.x + ((event.clientX - drag.startX) / rect.width) * 100, 'x');
			const y = clamp(start.y + ((event.clientY - drag.startY) / rect.height) * 100, 'y');
			// snap to the centre line (Alt: free)
			guide = !event.altKey && Math.abs(x - 50) < 1.5;
			if (guide) x = 50;
			editLine(index, { x: round(x), y: round(y) }, key);
		} else if (kind === 'width') {
			const half = Math.abs(event.clientX - drag.cx);
			editLine(index, { maxWidth: round(clamp(((half * 2) / rect.width) * 100, 'maxWidth')) }, key);
		} else if (kind === 'size') {
			const dist = Math.hypot(event.clientX - drag.cx, event.clientY - drag.cy);
			editLine(index, { size: round(clamp(start.size * (dist / drag.startDist), 'size')) }, key);
		} else {
			// the handle sits above the line: straight up = 0°; Shift snaps to 5°
			let angle = (Math.atan2(event.clientY - drag.cy, event.clientX - drag.cx) * 180) / Math.PI + 90;
			if (angle > 180) angle -= 360;
			angle = event.shiftKey ? Math.round(angle / 5) * 5 : Math.round(angle);
			editLine(index, { rotate: clamp(angle, 'rotate') }, key);
		}
	}

	function endDrag() {
		drag = null;
		guide = false;
	}

	function onKey(event: KeyboardEvent, index: number) {
		const step = event.shiftKey ? 5 : 0.5;
		const moves: Record<string, [number, number]> = { ArrowLeft: [-step, 0], ArrowRight: [step, 0], ArrowUp: [0, -step], ArrowDown: [0, step] };
		if (moves[event.key]) {
			event.preventDefault();
			if (mode === 'default') writeLines(defaultCoverLines(format), 'cover-mode');
			const [dx, dy] = moves[event.key];
			const current = lines[index];
			editLine(index, { x: round(clamp(current.x + dx, 'x')), y: round(clamp(current.y + dy, 'y')) }, `cover-nudge-${index}`);
		} else if ((event.key === 'Delete' || event.key === 'Backspace') && mode === 'custom') {
			event.preventDefault();
			removeLine(index);
		} else if (event.key === 'Escape') selected = null;
	}

	const sourceLabel = (text: CoverLine['text']) =>
		text === 'toon' ? m['creator.cover_text_toon']() : text === 'season' ? m['creator.cover_text_season']() : m['creator.cover_text_custom']();
	const lineLabel = (l: CoverLine) => coverLineText(l, previewTexts, previewLang) || m['creator.cover_text_empty']();
	const colorLabel = (c: string) => (m as unknown as Record<string, () => string>)[`creator.cover_color_${c}`]?.() ?? c;
	const alignLabel = (a: string) => (m as unknown as Record<string, () => string>)[`creator.cover_align_${a}`]?.() ?? a;
	const customLanguages = $derived(textLanguage.visible(languages, source));
</script>

<div class="cover-editor" data-testid="cover-editor">
	<div class="toolbar">
		<span class="sc-label">{m['creator.cover_mode']()}</span>
		<span class="seg" role="group" aria-label={m['creator.cover_mode']()}>
			{#each [['off', m['creator.cover_mode_off']()], ['default', m['creator.cover_mode_default']()], ['custom', m['creator.cover_mode_custom']()]] as [value, label] (value)}
				<button type="button" class="ghost" class:active={mode === value} aria-pressed={mode === value} onclick={() => setMode(value as typeof mode)} data-testid="cover-mode-{value}">{label}</button>
			{/each}
		</span>
		{#if languages.length > 1}
			<span class="sc-label">{m['creator.cover_preview_language']()}</span>
			<span class="seg" role="group" aria-label={m['creator.cover_preview_language']()}>
				{#each languages as l (l)}
					<button type="button" class="ghost" class:active={previewLang === l} aria-pressed={previewLang === l} onclick={() => (previewLang = l)} data-testid="cover-lang-{l}">{l.toUpperCase()}</button>
				{/each}
			</span>
		{/if}
	</div>
	<p class="hint">{mode === 'off' ? m['creator.cover_mode_hint_off']() : mode === 'default' ? m['creator.cover_mode_hint_default']() : m['creator.cover_mode_hint_custom']()}</p>

	<div class="work">
		<div class="canvas">
			{#if image}
				<!-- the reader's own CoverArt, the editing boxes on top (same geometry: % of the cover, cqw) -->
				<div class="stage {format}" bind:this={stage} data-testid="cover-stage" role="presentation" onpointermove={onDrag} onpointerup={endDrag} onpointercancel={endDrag} onpointerdown={() => (selected = null)}>
					<CoverArt src={image} {format} lettering={mode === 'off' ? false : { [format]: lines }} title={previewTexts.toon} subtitle={previewTexts.season} lang={previewLang} />
					<div class="boxes">
						{#if guide}<div class="guide" aria-hidden="true"></div>{/if}
						{#each lines as l, i (i)}
							{@const height = Math.max(heights[i] || l.size, 2)}
							<div
								class="box"
								class:selected={selected === i}
								style="left: {l.x}%; top: {l.y}%; width: {l.maxWidth}%; height: {height}cqw; transform: translate(-50%, -50%) rotate({l.rotate ?? 0}deg) skewX({l.skew ?? 0}deg)"
								role="button"
								tabindex="0"
								aria-label="{sourceLabel(l.text)}: {lineLabel(l)}"
								title="{sourceLabel(l.text)}: {lineLabel(l)}"
								onpointerdown={(e) => startDrag(e, i, 'move')}
								onkeydown={(e) => onKey(e, i)}
								onfocus={() => (selected = i)}
								data-testid="cover-box-{i}"
							>
								{#if selected === i}
									<span class="handle width" role="presentation" title={m['creator.cover_handle_width']()} onpointerdown={(e) => startDrag(e, i, 'width')} data-testid="cover-handle-width"></span>
									<span class="handle size" role="presentation" title={m['creator.cover_handle_size']()} onpointerdown={(e) => startDrag(e, i, 'size')} data-testid="cover-handle-size"></span>
									<span class="handle rotate" role="presentation" title={m['creator.cover_handle_rotate']()} onpointerdown={(e) => startDrag(e, i, 'rotate')} data-testid="cover-handle-rotate"></span>
								{/if}
							</div>
						{/each}
					</div>
				</div>
			{:else}
				<p class="hint empty">{m['creator.cover_no_art']()}</p>
			{/if}
		</div>

		<aside class="inspector" data-testid="cover-inspector">
			<p class="sc-label">{m['creator.cover_lines']()}</p>
			{#if mode !== 'off'}
				<ul class="lines">
					{#each lines as l, i (i)}
						<li>
							<button type="button" class="ghost row" class:active={selected === i} onclick={() => (selected = i)} data-testid="cover-line-{i}">
								<span class="src">{sourceLabel(l.text)}</span>
								<span class="txt">{lineLabel(l)}</span>
							</button>
						</li>
					{/each}
				</ul>
				<button type="button" class="secondary" onclick={() => (mode === 'default' ? (writeLines(defaultCoverLines(format), 'cover-mode'), addLine()) : addLine())} data-testid="cover-add-line"><Icon name="plus" /> {m['creator.cover_add_line']()}</button>
			{/if}

			{#if line && selected !== null}
				{@const i = selected}
				<div class="props" data-testid="cover-props">
					<label class="field">
						<span class="sc-label">{m['creator.cover_text']()}</span>
						<select value={line.text} onchange={(e) => setText(i, e.currentTarget.value as CoverLine['text'])} data-testid="cover-text-source">
							{#each COVER_TEXT_SOURCES as s (s)}<option value={s} selected={line.text === s}>{sourceLabel(s)}</option>{/each}
						</select>
					</label>
					{#if line.text === 'custom'}
						<div class="custom" data-testid="cover-custom-text">
							{#each customLanguages as l (l)}
								<label class="field">
									<span class="sc-label">{l}</span>
									<input type="text" value={line.custom?.[l] ?? ''} lang={l} oninput={(e) => setCustom(i, l, e.currentTarget.value)} data-testid="cover-custom-{l}" />
								</label>
							{/each}
						</div>
					{:else}
						<p class="hint">{m['creator.cover_text_hint']()}</p>
					{/if}
					<label class="check">
						<input type="checkbox" checked={line.quotes === true} onchange={(e) => editLine(i, { quotes: e.currentTarget.checked || undefined }, 'cover-quotes')} />
						<span>{m['creator.cover_quotes']()}</span>
					</label>
					<div class="grid">
						{#each [['x', m['creator.cover_x']()], ['y', m['creator.cover_y']()], ['size', m['creator.cover_size']()], ['maxWidth', m['creator.cover_width']()], ['rotate', m['creator.cover_rotate']()], ['skew', m['creator.cover_skew']()]] as [key, label] (key)}
							{@const k = key as keyof typeof COVER_LIMITS}
							<label class="field">
								<span class="sc-label">{label}</span>
								<input type="number" step="0.5" min={COVER_LIMITS[k].min} max={COVER_LIMITS[k].max} value={line[k] ?? 0} onchange={(e) => setNumber(i, k, e.currentTarget.value)} data-testid="cover-{key}" />
							</label>
						{/each}
					</div>
					<div class="field">
						<span class="sc-label">{m['creator.cover_align']()}</span>
						<span class="seg" role="group" aria-label={m['creator.cover_align']()}>
							{#each COVER_ALIGNS as a (a)}
								<button type="button" class="ghost" class:active={(line.align ?? 'center') === a} aria-pressed={(line.align ?? 'center') === a} title={alignLabel(a)} aria-label={alignLabel(a)} onclick={() => editLine(i, { align: a === 'center' ? undefined : a }, 'cover-align')}><Icon name="align-{a}" /></button>
							{/each}
						</span>
					</div>
					<div class="grid">
						{#each [['color', m['creator.cover_color'](), 'paper'], ['outline', m['creator.cover_outline'](), 'ink']] as [key, label, fallback] (key)}
							{@const k = key as 'color' | 'outline'}
							<label class="field">
								<span class="sc-label">{label}</span>
								<span class="color">
									<span class="swatch" style="background: {COVER_COLOR_VALUES[line[k] ?? (fallback as 'paper')]}"></span>
									<select onchange={(e) => editLine(i, { [k]: e.currentTarget.value === fallback ? undefined : e.currentTarget.value }, `cover-${k}`)} data-testid="cover-{key}">
										{#each COVER_COLORS as c (c)}<option value={c} selected={(line[k] ?? fallback) === c}>{colorLabel(c)}</option>{/each}
									</select>
								</span>
							</label>
						{/each}
					</div>
					<div class="line-actions">
						<button type="button" class="secondary" disabled={i === 0} title={m['creator.cover_line_up']()} aria-label={m['creator.cover_line_up']()} onclick={() => moveLine(i, -1)}><Icon name="arrow-up" /></button>
						<button type="button" class="secondary" disabled={i === lines.length - 1} title={m['creator.cover_line_down']()} aria-label={m['creator.cover_line_down']()} onclick={() => moveLine(i, 1)}><Icon name="arrow-down" /></button>
						<button type="button" class="secondary" title={m['creator.cover_remove_line']()} onclick={() => (mode === 'default' ? (writeLines(defaultCoverLines(format), 'cover-mode'), removeLine(i)) : removeLine(i))} data-testid="cover-remove-line"><Icon name="trash-2" /> {m['creator.cover_remove_line']()}</button>
					</div>
				</div>
			{/if}
		</aside>
	</div>

	{#if image}
		<section class="readers" data-testid="cover-readers">
			<h3 class="sc-heading">{m['creator.cover_readers']()}</h3>
			<div class="reader-grid">
				{#each languages as l (l)}
					{@const t = texts(l)}
					<figure class="reader" data-testid="cover-reader-{l}">
						<CoverArt src={image} {format} lettering={mode === 'off' ? false : { [format]: lines }} title={t.toon} subtitle={t.season} lang={l} />
						<figcaption class="hint">{l.toUpperCase()}{l === source ? ` · ${m['creator.source']()}` : ''}</figcaption>
					</figure>
				{/each}
			</div>
		</section>
	{/if}
</div>

<style>
	.cover-editor {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
	}
	.toolbar {
		display: flex;
		flex-wrap: wrap;
		align-items: center;
		gap: var(--space-05) var(--space-1);
	}
	/* segmented control: one 2px ink frame, the chosen part inverted */
	.seg {
		display: inline-flex;
		border: var(--border-width-regular) solid var(--ink);
	}
	.seg button {
		min-height: var(--control-height);
	}
	.hint {
		margin: 0;
	}
	.work {
		display: grid;
		/* the art as big as the screen's height allows (top bar + toolbar + ActionBar), 36rem at most */
		grid-template-columns: minmax(0, min(36rem, max(18rem, calc(100dvh - 19rem)))) minmax(16rem, 1fr);
		gap: var(--space-15);
		align-items: start;
	}
	@media (max-width: 900px) {
		.work {
			grid-template-columns: minmax(0, 1fr);
		}
	}
	.canvas {
		min-width: 0;
	}
	.stage {
		position: relative;
		border: var(--border-width-regular) solid var(--ink);
		touch-action: none;
		user-select: none;
	}
	.boxes {
		position: absolute;
		inset: 0;
		container-type: inline-size;
	}
	.box {
		position: absolute;
		box-sizing: border-box;
		border: var(--border-width-regular) dashed rgb(255 255 255 / 0.7);
		outline: var(--border-width-hairline) dashed rgb(0 0 0 / 0.6);
		cursor: move;
	}
	.box:hover {
		border-style: solid;
	}
	.box.selected,
	.box:focus-visible {
		border: var(--border-width-regular) solid var(--primary-cyan);
		outline: var(--border-width-hairline) solid var(--ink);
	}
	.guide {
		position: absolute;
		left: 50%;
		top: 0;
		bottom: 0;
		border-left: var(--border-width-hairline) solid var(--primary-cyan);
		pointer-events: none;
	}
	.handle {
		position: absolute;
		width: 0.75rem;
		height: 0.75rem;
		background: var(--paper);
		border: var(--border-width-regular) solid var(--ink);
		box-sizing: border-box;
	}
	.handle.width {
		right: -0.4rem;
		top: 50%;
		transform: translateY(-50%);
		cursor: ew-resize;
	}
	.handle.size {
		right: -0.4rem;
		bottom: -0.4rem;
		cursor: nwse-resize;
		background: var(--ink);
	}
	.handle.rotate {
		left: 50%;
		top: -1.5rem;
		transform: translateX(-50%);
		border-radius: var(--radius-full);
		cursor: grab;
	}
	@media (pointer: coarse) {
		.handle {
			width: 1.5rem;
			height: 1.5rem;
		}
		.handle.width,
		.handle.size {
			right: -0.75rem;
		}
		.handle.size {
			bottom: -0.75rem;
		}
		.handle.rotate {
			top: -2.25rem;
		}
	}
	.empty {
		border: var(--border-width-regular) dashed var(--border-muted);
		padding: var(--space-3) var(--space-1);
		text-align: center;
	}
	.inspector {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		min-width: 0;
	}
	.inspector > .sc-label {
		margin: 0;
	}
	.lines {
		list-style: none;
		margin: 0;
		padding: 0;
		border: var(--border-width-regular) solid var(--ink);
	}
	.lines li + li {
		border-top: var(--border-width-hairline) solid var(--ink);
	}
	.row {
		width: 100%;
		justify-content: flex-start;
		gap: var(--space-05);
		text-align: left;
	}
	.row .src {
		flex: none;
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	.row .txt {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.props {
		display: flex;
		flex-direction: column;
		gap: var(--space-1);
		border-top: var(--border-width-regular) solid var(--ink);
		padding-top: var(--space-1);
	}
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	.custom {
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	.grid {
		display: grid;
		grid-template-columns: repeat(2, minmax(0, 1fr));
		gap: var(--space-1);
	}
	.check {
		display: flex !important;
		align-items: center;
		gap: var(--space-05);
	}
	.color {
		display: flex;
		align-items: stretch;
		gap: var(--space-05);
	}
	.swatch {
		flex: none;
		width: var(--control-height);
		border: var(--border-width-regular) solid var(--ink);
		box-sizing: border-box;
	}
	.line-actions {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
	}
	.readers h3 {
		margin: 0 0 var(--space-05);
	}
	.reader-grid {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-1);
	}
	.reader {
		margin: 0;
		width: 14rem;
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
	}
</style>
