<script lang="ts">
	import { tick } from 'svelte';
	import { Segmented, liveRegion, region } from '@scutoons/ui';
	import { moveAt } from '$lib/editor/move.ts';
	import {
		COVER_LIMITS,
		clone,
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
	import { toonUrl } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import CoverLetteringCanvas from '$lib/components/cover/cover-lettering-canvas.svelte';
	import CoverInspector from '$lib/components/cover/cover-inspector.svelte';
	import CoverPreviews from '$lib/components/cover/cover-previews.svelte';

	/**
	 * The cover editor (owner 2026-10-02, agents/DESIGN.md "Cover editor"): the text-free art with its lettering lines
	 * drawn over it – drag to move, handles for width / size / angle, number fields, add / remove / reorder lines, the
	 * text source per line – a preview language, and every language "as readers see it". One component for any doc
	 * holding a `Cover` at `path`: the season's (owner 2026-10-02: covers are per season, the toon is shown with its
	 * avatar). It writes `<path>.lettering` and `<path>.square` once into the language-neutral season document
	 * (agents/NEUTRAL.md: the art is chosen once; a line's `custom` text is a locale map, the other lines' texts come
	 * from the title fields).
	 * The square format only: the reader shows no portrait cover (owner 2026-10-02 "portrait I think we are not using").
	 * Parts (COMPONENTS.md step 9): CoverLetteringCanvas (drag / handles / keys), CoverInspector (art, lines, the line's
	 * properties), CoverPreviews; the writing stays here.
	 */
	let {
		doc,
		path = ['cover'],
		toon,
		languages,
		source,
		texts,
		format = 'square',
		place = ''
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
		/** where an uploaded art goes by the content rules (CDN-root folder of the season, e.g. catpope/s1/) */
		place?: string;
	} = $props();

	const cover = $derived(doc.get(doc.primary, path) as Cover | undefined);
	const lettering = $derived(cover?.lettering);
	const mode = $derived<'off' | 'default' | 'custom'>(!coverLetteringOn(lettering) ? 'off' : coverLines(lettering, format) ? 'custom' : 'default');
	/** the lines on the canvas: the format's own, or the default layout's (editing them makes them the format's own) */
	const lines = $derived<CoverLine[]>(mode === 'off' ? [] : (coverLines(lettering, format) ?? defaultCoverLines(format)));
	const image = $derived(cover?.[format] ? toonUrl(toon, cover[format]) : '');

	// the preview language = the header's text language switch (owner 2026-10-03: no own switch here; 2026-10-06: one
	// language, default the source) – the canvas, an own line's text and "as readers see it" all show it
	const previewLang = $derived(textLanguage.current(languages, source));
	const previewTexts = $derived(texts(previewLang || source));

	let selected = $state<number | null>(null);
	$effect(() => {
		if (selected !== null && selected >= lines.length) selected = lines.length ? lines.length - 1 : null;
	});
	const line = $derived(selected !== null ? lines[selected] : undefined);

	// ── writing ─────────────────────────────────────────────────────────────────────────────────────────────────
	const round = (v: number) => Math.round(v * 10) / 10;
	const clamp = (v: number, key: keyof typeof COVER_LIMITS) => Math.min(COVER_LIMITS[key].max, Math.max(COVER_LIMITS[key].min, v));

	/** a cover field of the language-neutral season (once; the lettering's custom texts are locale maps) */
	function writeCover(field: 'lettering' | CoverFormat, value: unknown, key: string) {
		doc.set([...path, field], value === undefined ? undefined : clone(value), key);
	}

	function writeLettering(next: boolean | CoverLettering | undefined, key: string) {
		writeCover('lettering', next, key);
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

	function moveLine(index: number, to: number) {
		const next = $state.snapshot(lines) as CoverLine[];
		if (!moveAt(next, index, to)) return;
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

	const lineLabel = (l: CoverLine) => coverLineText(l, previewTexts, previewLang) || m['creator.cover_text_empty']();
	/** the default layout becomes the format's own (touched on the canvas, a line added / removed) */
	const ownLayout = () => writeLines(defaultCoverLines(format), 'cover-mode');
	const customLanguages = $derived([previewLang]);
</script>

<div class="cover-editor" data-testid="cover-editor">
	<div class="toolbar">
		<span class="sc-label">{m['creator.cover_mode']()}</span>
		<Segmented
			label={m['creator.cover_mode']()}
			value={mode}
			onchange={setMode}
			options={[
				{ value: 'off', label: m['creator.cover_mode_off'](), testid: 'cover-mode-off' },
				{ value: 'default', label: m['creator.cover_mode_default'](), testid: 'cover-mode-default' },
				{ value: 'custom', label: m['creator.cover_mode_custom'](), testid: 'cover-mode-custom' }
			]}
		/>
	</div>
	<p class="hint">{mode === 'off' ? m['creator.cover_mode_hint_off']() : mode === 'default' ? m['creator.cover_mode_hint_default']() : m['creator.cover_mode_hint_custom']()}</p>

	<!-- the cover canvas + its Lines list = ONE selection region (DESIGN.md "Selection"): live, the selected line shows its
	     handles and takes the keys -->
	<div class="work" use:region={{ name: 'cover', initial: true }}>
		<CoverLetteringCanvas {image} {format} {lines} {mode} texts={previewTexts} lang={previewLang} bind:selected live={liveRegion.is('cover')} {lineLabel} onedit={editLine} onown={ownLayout} onremove={removeLine} />
		<CoverInspector
			{toon}
			{place}
			art={cover?.[format]}
			{mode}
			{lines}
			bind:selected
			{customLanguages}
			{source}
			{lineLabel}
			onart={(v) => writeCover(format, v, 'cover-art')}
			onadd={() => (mode === 'default' ? (ownLayout(), addLine()) : addLine())}
			onedit={editLine}
			onnumber={setNumber}
			ontext={setText}
			oncustom={setCustom}
			onmove={moveLine}
			onremove={(i) => (mode === 'default' ? (ownLayout(), removeLine(i)) : removeLine(i))}
		/>
	</div>

	{#if image}
		<CoverPreviews {image} {format} {lines} off={mode === 'off'} languages={[previewLang]} {source} {texts} />
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
</style>
