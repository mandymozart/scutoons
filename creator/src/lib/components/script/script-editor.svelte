<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { page } from '$app/state';
	import {
		entities,
		NEUTRAL_LOCALE,
		scriptOutline,
		validateFile,
		validateLocales,
		type Problem,
		type ScriptMeta,
		type ScriptOutlineItem,
		type ToonLanguages,
		type Language,
		type ToonScript,
		withToonLanguages
	} from '@scutoons/content';
	import { toonDoc, toonIndexDoc } from '$lib/editor/languages.ts';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { Divider, Icon, Rail, confirmDialog, dockRegion } from '@scutoons/ui';
	import MetaPage from '$lib/components/page/meta-page.svelte';
	import ScriptOutline from '$lib/components/script/script-outline.svelte';
	import ScriptTitlePage from '$lib/components/script/script-meta.svelte';
	import { isEditKey } from '$lib/editor/script-edit.ts';

	/**
	 * The toon's screenplay (entity `script:<toon>:<l>`, agents/DESIGN.md "Creator: script page"): one big monospace text
	 * per language, edited exactly as typed, versioned like every entity (bottom bar: save draft, publish, history with
	 * restore). Big texts stay fast: the textarea is the working copy and is written into the doc only after a short
	 * pause (and before every save / language switch / outline jump), so a keystroke never copies, stringifies or parses
	 * the whole script. The outline is parsed from the doc (screenplay editor rules, `scriptOutline`).
	 * Parts (COMPONENTS.md step 9): ScriptOutline (the index, folds to a rail), ScriptMeta (the title page); the text, its
	 * sync with the doc and the layout stay here.
	 */
	let { toon, languages }: { toon: string; languages: ToonLanguages } = $props();

	// svelte-ignore state_referenced_locally
	const doc = toonDoc(entities.script(toon), languages);
	const SYNC_MS = 300;
	const INDENT = '    ';

	const requested = page.url.searchParams.get('lang');
	// svelte-ignore state_referenced_locally
	let locale = $state<string>(requested && doc.locales.includes(requested) ? requested : (textLanguage.chosen(doc.locales) ?? doc.primary));
	// the top bar's TEXT language decides which one is shown (one script at a time)
	let followed = textLanguage.value;
	$effect(() => {
		if (textLanguage.value === followed) return;
		followed = textLanguage.value;
		// the header's language; one this toon has not = the script's source language
		const chosen = textLanguage.chosen(doc.locales) ?? doc.primary;
		untrack(() => switchLocale(chosen));
	});
	let area = $state<HTMLTextAreaElement | undefined>();

	// ── layout (agents/DOCKING.md): the outline is a left region of the page – a Divider between it and the text drags it
	// (16rem … half the page), it folds to a rail labelled "Outline" (its fold button, dragging it narrow; remembered) –
	// the sidebar's dockRegion; the text ends 1rem above the footer ──
	let layoutEl = $state<HTMLElement>();
	const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	const outlineRegion = dockRegion({
		key: () => 'sc-script-outline-width',
		foldKey: () => 'sc-script-outline-folded',
		side: 'left',
		anchor: () => layoutEl?.getBoundingClientRect().left ?? 0,
		normal: () => 16 * rem(),
		max: () => (layoutEl?.clientWidth ?? 50 * rem()) / 2
	});
	outlineRegion.restore();
	const outlineFolded = $derived(outlineRegion.folded);
	const outlineCss = $derived(outlineFolded ? 'calc(var(--control-height) + 1rem + var(--border-width-regular))' : outlineRegion.width !== null ? `${outlineRegion.width}px` : '16rem');
	let textHeight = $state<number | null>(null);
	let outlineHeight = $state<number | null>(null);
	// folding changes where the list starts: fit the heights again
	$effect(() => {
		void outlineFolded;
		queueMicrotask(fitHeight);
	});
	let outlineList = $state<HTMLElement>();
	/** the text (and the index) reach down to 1rem above the bottom action bar – never under it */
	function fitHeight() {
		if (!area) return;
		const bar = document.querySelector<HTMLElement>('.sc-action-bar');
		const barHeight = bar ? bar.getBoundingClientRect().height : 0;
		textHeight = Math.max(15 * rem(), Math.floor(window.innerHeight - area.getBoundingClientRect().top - barHeight - rem()));
		// the outline's list scrolls down to the bar exactly (owner 2026-10-03); its heading stays above it
		if (outlineList) outlineHeight = Math.max(7.5 * rem(), Math.floor(window.innerHeight - outlineList.getBoundingClientRect().top - barHeight));
	}
	$effect(() => {
		if (!area) return;
		fitHeight();
		const observer = new ResizeObserver(() => fitHeight());
		observer.observe(document.documentElement);
		window.addEventListener('resize', fitHeight);
		return () => {
			observer.disconnect();
			window.removeEventListener('resize', fitHeight);
		};
	});
	/** the locale the textarea currently shows (the one a pending edit belongs to) */
	let shown: string | null = null;
	let timer: ReturnType<typeof setTimeout> | null = null;
	let pending = false;
	let escaped = false;
	let cursor = $state({ line: 1, column: 1 });
	let lineHeight = 20;

	onMount(() => {
		doc.load();
		return () => flush();
	});

	// A toon without a script: an empty source text, not dirty until something is typed (no "missing" noise)
	$effect(() => {
		if (doc.loading || doc.data[doc.primary] != null) return;
		untrack(() => {
			const empty: ToonScript = { text: '' };
			doc.data[doc.primary] = empty;
			doc.saved[doc.primary] = JSON.stringify(empty);
		});
	});

	const script = $derived<ToonScript | null>(doc.data[locale] ?? null);
	const text = $derived(typeof script?.text === 'string' ? script.text : '');
	const outline = $derived(scriptOutline(text));
	const lineCount = $derived(text ? (text.match(/\n/g)?.length ?? 0) + (text.endsWith('\n') ? 0 : 1) : 0);
	/** the outline item the cursor is in */
	const current = $derived.by(() => {
		let found: ScriptOutlineItem | null = null;
		for (const item of outline) {
			if (item.line > cursor.line) break;
			found = item;
		}
		return found;
	});

	// The doc → the textarea (load, restore, undo/redo, language switch); never while an edit is pending
	$effect(() => {
		const value = text;
		const el = area;
		const l = locale;
		if (!el || script == null) return;
		untrack(() => {
			if (pending && shown === l) return;
			const switched = shown !== l;
			shown = l;
			if (el.value === value) return;
			const { scrollTop, selectionStart, selectionEnd } = el;
			el.value = value;
			if (switched) {
				el.scrollTop = 0;
				el.setSelectionRange(0, 0);
			} else {
				el.setSelectionRange(Math.min(selectionStart, value.length), Math.min(selectionEnd, value.length));
				el.scrollTop = scrollTop;
			}
			updateCursor();
		});
	});

	$effect(() => {
		if (!area) return;
		const parsed = parseFloat(getComputedStyle(area).lineHeight);
		if (parsed > 0) lineHeight = parsed;
	});

	/** The textarea → the doc (one undo step per typing burst, like every other field) */
	function flush() {
		if (timer) clearTimeout(timer);
		timer = null;
		if (!pending || !area || !shown) return;
		pending = false;
		const value = area.value;
		if (doc.data[shown] && doc.data[shown].text !== value) doc.setLocal(shown, ['text'], value, `script-text:${shown}`);
	}

	function onInput() {
		pending = true;
		if (timer) clearTimeout(timer);
		timer = setTimeout(flush, SYNC_MS);
	}

	let cursorFrame = 0;
	function updateCursor() {
		cancelAnimationFrame(cursorFrame);
		cursorFrame = requestAnimationFrame(() => {
			if (!area) return;
			const before = area.value.slice(0, area.selectionStart);
			const lineStart = before.lastIndexOf('\n') + 1;
			cursor = { line: (before.match(/\n/g)?.length ?? 0) + 1, column: before.length - lineStart + 1 };
		});
	}

	/** read-only until the user confirms they mean to edit (owner 2026-10-03); per visit, not remembered */
	let editing = $state(false);
	let asking = false;
	async function requestEdit() {
		if (editing || asking) return;
		asking = true;
		editing = await confirmDialog({
			title: m['creator.script_edit_title'](),
			message: m['creator.script_edit_confirm'](),
			confirmLabel: m['creator.script_edit']()
		});
		asking = false;
		if (editing) area?.focus({ preventScroll: true });
	}

	/** Tab indents with spaces (the format counts spaces); Esc, then Tab leaves the text (keyboard users) */
	function onKeydown(event: KeyboardEvent) {
		if (!editing) {
			if (isEditKey(event)) {
				event.preventDefault();
				requestEdit();
			}
			return;
		}
		if (event.key === 'Escape') {
			escaped = true;
			return;
		}
		if (event.key !== 'Tab' || event.ctrlKey || event.altKey || event.metaKey) {
			escaped = false;
			return;
		}
		if (escaped) {
			escaped = false;
			return;
		}
		event.preventDefault();
		if (event.shiftKey) outdent();
		else insertText(INDENT);
	}

	/** Edits through the browser's own command keep its undo (Ctrl+Z in the text) */
	function insertText(value: string) {
		if (!area) return;
		if (!document.execCommand('insertText', false, value)) {
			area.setRangeText(value, area.selectionStart, area.selectionEnd, 'end');
			onInput();
		}
	}

	function outdent() {
		if (!area) return;
		const caret = area.selectionStart;
		const lineStart = area.value.lastIndexOf('\n', caret - 1) + 1;
		const spaces = /^ {0,4}/.exec(area.value.slice(lineStart, lineStart + 4))?.[0].length ?? 0;
		if (!spaces) return;
		area.setSelectionRange(lineStart, lineStart + spaces);
		if (!document.execCommand('delete')) {
			area.setRangeText('', lineStart, lineStart + spaces, 'start');
			onInput();
		}
		const next = Math.max(lineStart, caret - spaces);
		area.setSelectionRange(next, next);
	}

	/** Puts the cursor at the start of a line and scrolls it to the top of the text (no wrapping: line n = row n) */
	function jump(line: number) {
		if (!area) return;
		flush();
		const value = area.value;
		let offset = 0;
		for (let i = 1; i < line && offset !== -1; i++) offset = value.indexOf('\n', offset) + 1 || -1;
		if (offset < 0) offset = value.length;
		area.focus({ preventScroll: true });
		area.setSelectionRange(offset, offset);
		const padding = parseFloat(getComputedStyle(area).paddingTop) || 0;
		area.scrollTop = Math.max(0, (line - 1) * lineHeight + padding - lineHeight);
		area.scrollLeft = 0;
		updateCursor();
	}

	function switchLocale(next: string) {
		if (next === locale) return;
		flush();
		locale = next;
	}

	function startLocale(copy: boolean) {
		doc.createLocale(locale, copy ? undefined : () => ({ text: '' }));
	}

	/**
	 * The script is the ultimate source (owner 2026-10-14): a translation's script can make its language the toon's source
	 * language – asked first, saved as a draft of the toon entry (`toon:<toon>`), the page reloads in the new order.
	 * Not with unsaved script changes (the reload would drop them).
	 */
	let switching = $state(false);
	async function makeSource() {
		if (switching || locale === doc.primary || doc.dirty) return;
		const ok = await confirmDialog({
			title: m['creator.make_source_title']({ locale }),
			message: m['creator.make_source_message']({ locale, source: doc.primary }),
			confirmLabel: m['creator.make_source']({ locale })
		});
		if (!ok) return;
		switching = true;
		try {
			const entry = toonIndexDoc(toon);
			await entry.load();
			const next = withToonLanguages(entry.data[NEUTRAL_LOCALE] ?? {}, locale as Language, languages.languages);
			entry.setLocal(NEUTRAL_LOCALE, ['sourceLanguage'], next.sourceLanguage, 'languages');
			entry.setLocal(NEUTRAL_LOCALE, ['languages'], next.languages, 'languages');
			await entry.save(`source language ${locale}`);
			window.location.reload();
		} finally {
			switching = false;
		}
	}

	function setMeta(field: keyof ScriptMeta, value: string) {
		const meta: ScriptMeta = { ...(doc.data[locale]?.meta ?? {}) };
		if (value) meta[field] = value;
		else delete meta[field];
		doc.setLocal(locale, ['meta'], Object.keys(meta).length ? meta : undefined, `script-meta:${locale}:${field}`);
	}

	function validate(): Problem[] {
		const out: Problem[] = [];
		for (const l of doc.locales) if (doc.data[l] != null) out.push(...validateFile('script', doc.data[l], l, { file: doc.entity }));
		out.push(...validateLocales('script', doc.entity, doc.data, doc.primary));
		return out;
	}

</script>

<MetaPage {toon} title={m['creator.script']()} docs={[doc]} {validate} wide help="script" beforeSave={flush}>
	<!-- format rules + line / column in the file status line (owner 2026-10-03: no row of their own) -->
	{#snippet status()}
		<a class="rules-link" href="/guide#script-format" target="_blank" rel="noopener" data-testid="script-rules-link">{m['creator.script_rules']()}</a>
		{#if script}
			<span class="hint" data-testid="script-stats">
				{m['creator.script_lines']({ lines: lineCount })} · {m['creator.script_position']({ line: cursor.line, column: cursor.column })}
			</span>
		{/if}
	{/snippet}
	<div class="script-page" data-testid="script-editor">

		{#if !script}
			<section class="missing" data-testid="script-missing">
				<p class="sc-empty">{m['creator.script_missing_locale']({ locale, source: doc.primary })}</p>
				<p class="buttons">
					<button type="button" onclick={() => startLocale(false)} data-testid="script-start-empty">{m['creator.script_start_empty']()}</button>
					{#if doc.data[doc.primary]?.text}
						<button type="button" class="secondary" onclick={() => startLocale(true)} data-testid="script-copy-source">
							{m['creator.script_copy_source']({ source: doc.primary })}
						</button>
					{/if}
				</p>
			</section>
		{:else}
			<div class="layout" class:folded={outlineFolded} bind:this={layoutEl} style:--outline-width={outlineCss} style:--script-height={textHeight !== null ? `${textHeight}px` : undefined} style:--outline-height={outlineHeight !== null ? `${outlineHeight}px` : undefined}>
				<aside class="side" id="script-outline">
					{#if outlineFolded}
						<Rail side="left" label={m['creator.script_outline']()} unfoldLabel={m['creator.script_outline_show']()} controls="script-outline" onunfold={() => outlineRegion.fold(false)} testid="script-outline-rail" />
					{:else}
						<ScriptOutline {outline} {current} bind:list={outlineList} onfold={() => outlineRegion.fold(true)} onjump={jump} />
					{/if}
				</aside>
				<Divider size={outlineRegion} edge="right" at="var(--outline-width)" label={m['creator.resize_panel']()} controls="script-outline" class="outline-resize" />

				<div class="main">
					<div class="head">
					<ScriptTitlePage meta={script.meta} {editing} onset={setMeta} onrequestedit={requestEdit} />
					{#if locale !== doc.primary}
						<button type="button" class="edit-script secondary" onclick={makeSource} disabled={switching || doc.dirty} title={doc.dirty ? m['creator.make_source_save_first']() : undefined} data-testid="script-make-source">
							{m['creator.make_source']({ locale })}
						</button>
					{/if}
					{#if !editing}
						<button type="button" class="edit-script secondary" onclick={requestEdit} data-testid="script-edit">
							<Icon name="lock" />{m['creator.script_edit']()}
						</button>
					{/if}
					</div>
					<textarea
						bind:this={area}
						class="text"
						readonly={!editing}
						lang={locale}
						wrap="off"
						spellcheck="false"
						autocomplete="off"
						autocapitalize="off"
						aria-label={m['creator.script_editor_label']({ locale })}
						oninput={onInput}
						onkeydown={onKeydown}
						onkeyup={updateCursor}
						onclick={updateCursor}
						onselect={updateCursor}
						onblur={flush}
						data-testid="script-text"
					></textarea>
				</div>
			</div>
		{/if}
	</div>
</MetaPage>

<style>
	.script-page {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
	}
	/* the outline starts at the sidebar's divider (no page padding before it); its right border is the Divider's line */
	.layout {
		position: relative;
		display: grid;
		grid-template-columns: var(--outline-width, 16rem) minmax(0, 1fr);
		gap: 0;
		align-items: start;
		/* flush with the sidebar divider, up to the status line and down to the action bar: the border runs the full
		   height; only the text keeps its 1rem above the bar (fitHeight) */
		margin: -1rem 0 -1rem -1rem;
	}
	.side {
		align-self: stretch;
		box-sizing: border-box;
		min-width: 0;
		padding: 1rem 0 0;
		border-right: var(--border-width-regular) solid var(--ink);
	}
	.layout.folded .side {
		padding: 0;
	}
	/* the content 1rem from the border; its 1rem below the text keeps the border running down to the action bar */
	.main {
		padding: 1rem 0 1rem 1rem;
	}
	/* the title page row; "Edit script" at its right while the script is read-only */
	.head {
		display: flex;
		align-items: flex-start;
		gap: 1rem;
	}
	.edit-script {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		flex: none;
	}
	.rules-link {
		color: var(--ink);
	}
	/* narrower: the outline above the text (no rail, no divider) */
	@media (max-width: 900px) {
		.layout {
			grid-template-columns: minmax(0, 1fr);
		}
		.side {
			border-right: 0;
		}
		.layout :global(.outline-resize) {
			display: none;
		}
	}
	.main {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		min-width: 0;
	}
	/* the screenplay: IBM Plex Mono, every space as typed, no wrapping (line n = row n), wide */
	.text {
		font-family: var(--font-mono);
		line-height: var(--type-body-line);
		tab-size: 4;
		white-space: pre;
		overflow: auto;
		width: 100%;
		height: var(--script-height, calc(100vh - 16rem));
		box-sizing: border-box;
		padding: var(--space-05) var(--space-075);
		border: 2px solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		resize: none;
	}
	.missing {
		display: grid;
		gap: var(--space-1);
	}
	.missing p {
		margin: 0;
	}
	.buttons {
		display: flex;
		gap: 0.5rem;
	}
</style>
