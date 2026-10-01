<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { page } from '$app/state';
	import {
		entities,
		scriptOutline,
		SCRIPT_META_FIELDS,
		validateFile,
		validateLocales,
		type Problem,
		type ScriptMeta,
		type ScriptOutlineItem,
		type ToonLanguages,
		type ToonScript
	} from '@scutoons/content';
	import { toonDoc } from '$lib/editor/languages.ts';
	import { m } from '$lib/paraglide/messages.js';
	import MetaPage from '../meta-page.svelte';
	import ScriptRules from './script-rules.svelte';

	/**
	 * The toon's screenplay (entity `script:<toon>:<l>`, agents/DESIGN.md "Creator: script page"): one big monospace text
	 * per language, edited exactly as typed, versioned like every entity (bottom bar: save draft, publish, history with
	 * restore). Big texts stay fast: the textarea is the working copy and is written into the doc only after a short
	 * pause (and before every save / language switch / outline jump), so a keystroke never copies, stringifies or parses
	 * the whole script. The outline is parsed from the doc (screenplay editor rules, `scriptOutline`).
	 */
	let { toon, languages }: { toon: string; languages: ToonLanguages } = $props();

	// svelte-ignore state_referenced_locally
	const doc = toonDoc(entities.script(toon), languages);
	const SYNC_MS = 300;
	const INDENT = '    ';

	const requested = page.url.searchParams.get('lang');
	let locale = $state<string>(requested && doc.locales.includes(requested) ? requested : doc.primary);
	let area = $state<HTMLTextAreaElement | undefined>();
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

	/** Tab indents with spaces (the format counts spaces); Esc, then Tab leaves the text (keyboard users) */
	function onKeydown(event: KeyboardEvent) {
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

	const metaLabel: Record<keyof ScriptMeta, () => string> = {
		title: () => m['creator.script_meta_title'](),
		author: () => m['creator.script_meta_author'](),
		status: () => m['creator.script_meta_status'](),
		date: () => m['creator.script_meta_date'](),
		basedOn: () => m['creator.script_meta_basedOn'](),
		basedOnFrom: () => m['creator.script_meta_basedOnFrom']()
	};

	function itemLabel(item: ScriptOutlineItem): string {
		if (item.kind === 'scene') return item.heading;
		const head = item.kind === 'chapter' ? m['creator.entity_chapter']({ number: item.number }) : m['creator.script_song']({ number: item.number });
		return item.title ? `${head} · ${item.title}` : head;
	}
</script>

<MetaPage {toon} title={m['creator.script']()} docs={[doc]} {validate} wide help="script" translationsOptional beforeSave={flush}>
	<div class="script-page" data-testid="script-editor">
		<header class="top">
			<div class="locales" role="group" aria-label={m['creator.languages']()}>
				{#each doc.locales as l (l)}
					<button
						type="button"
						class="ghost"
						class:active={l === locale}
						aria-pressed={l === locale}
						onclick={() => switchLocale(l)}
						data-testid="script-locale-{l}"
					>
						{l}{l === doc.primary ? ` · ${m['creator.script_source']()}` : ''}{doc.dirtyLocales.includes(l) ? ' •' : ''}
					</button>
				{/each}
			</div>
			{#if script}
				<span class="hint" data-testid="script-stats">
					{m['creator.script_lines']({ lines: lineCount })} · {m['creator.script_position']({ line: cursor.line, column: cursor.column })}
				</span>
			{/if}
		</header>

		{#if !script}
			<section class="missing" data-testid="script-missing">
				<p>{m['creator.script_missing_locale']({ locale, source: doc.primary })}</p>
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
			<div class="layout">
				<aside class="side">
					<section class="outline" data-testid="script-outline" aria-label={m['creator.script_outline']()}>
						<h2>{m['creator.script_outline']()}</h2>
						{#if !outline.length}
							<p class="hint">{m['creator.script_outline_empty']()}</p>
						{:else}
							<ol>
								{#each outline as item (item.line)}
									<li class={item.kind}>
										<button
											type="button"
											class:current={item === current}
											onclick={() => jump(item.line)}
											title="{itemLabel(item)} ({item.line})"
											data-line={item.line}
											data-testid="script-outline-item"
										>
											{#if item.kind === 'scene'}<span class="num">{item.number}</span>{/if}
											<span class="label">{itemLabel(item)}</span>
										</button>
									</li>
								{/each}
							</ol>
						{/if}
					</section>
					<details class="rules-panel" data-testid="script-rules-panel">
						<summary>{m['creator.script_rules']()}</summary>
						<ScriptRules />
					</details>
				</aside>

				<div class="main">
					<details class="meta" data-testid="script-meta">
						<summary>{m['creator.script_meta']()}{script.meta?.title ? ` · ${script.meta.title}` : ''}</summary>
						<div class="meta-grid">
							{#each SCRIPT_META_FIELDS as field (field)}
								<label>
									<span>{metaLabel[field]()}</span>
									<input
										type="text"
										value={script.meta?.[field] ?? ''}
										oninput={(e) => setMeta(field, e.currentTarget.value)}
										data-testid="script-meta-{field}"
									/>
								</label>
							{/each}
						</div>
					</details>
					<textarea
						bind:this={area}
						class="text"
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
	.top {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		align-items: center;
		justify-content: space-between;
	}
	.locales {
		display: flex;
		gap: 0.25rem;
	}
	.locales button.active {
		background: var(--ink);
		color: var(--paper);
	}
	.layout {
		display: grid;
		grid-template-columns: minmax(200px, 260px) minmax(0, 1fr);
		gap: 1rem;
		align-items: start;
	}
	@media (max-width: 900px) {
		.layout {
			grid-template-columns: minmax(0, 1fr);
		}
		.side {
			position: static !important;
			max-height: 240px !important;
		}
	}
	.side {
		position: sticky;
		top: 0.5rem;
		max-height: calc(100vh - 220px);
		overflow: auto;
		border: 2px solid var(--ink);
		padding: 0.4rem 0.5rem;
	}
	.side h2 {
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		text-transform: uppercase;
		margin: 0 0 0.3rem;
	}
	ol {
		list-style: none;
		margin: 0;
		padding: 0;
	}
	li button {
		display: flex;
		justify-content: flex-start;
		gap: 0.4rem;
		width: 100%;
		text-align: left;
		background: none;
		border: 0;
		border-left: 3px solid transparent;
		padding: 0.1rem 0.3rem;
		font-size: var(--type-caption-size);
		color: var(--ink);
		cursor: pointer;
	}
	li button:hover {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: calc(-1 * var(--border-width-regular));
	}
	li button.current {
		border-left-color: var(--ink);
		background: var(--ink);
		color: var(--paper);
	}
	li.chapter button {
		font-weight: 700;
		margin-top: 0.35rem;
	}
	li.song button {
		font-style: italic;
		margin-top: 0.35rem;
	}
	li.scene button {
		padding-left: 16px;
	}
	.num {
		min-width: 28px;
		color: var(--muted);
	}
	.label {
		min-width: 0;
		overflow: hidden;
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	.rules-panel {
		margin-top: 0.6rem;
		border-top: 2px solid var(--ink);
		padding-top: 0.3rem;
	}
	.rules-panel summary,
	.meta summary {
		cursor: pointer;
		font-weight: 700;
	}
	.main {
		display: flex;
		flex-direction: column;
		gap: 0.5rem;
		min-width: 0;
	}
	.meta-grid {
		display: grid;
		grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
		gap: 0.4rem 0.75rem;
		padding-top: 0.4rem;
	}
	.meta-grid label {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
	}
	.meta-grid span {
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		text-transform: uppercase;
		color: var(--muted);
	}
	/* the screenplay: IBM Plex Mono, every space as typed, no wrapping (line n = row n), wide */
	.text {
		font-family: var(--font-mono);
		line-height: var(--type-body-line);
		tab-size: 4;
		white-space: pre;
		overflow: auto;
		width: 100%;
		min-height: 420px;
		height: calc(100vh - 260px);
		box-sizing: border-box;
		padding: var(--space-05) var(--space-075);
		border: 2px solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		resize: vertical;
	}
	.missing {
		border: 2px dashed var(--ink);
		padding: 1rem;
	}
	.buttons {
		display: flex;
		gap: 0.5rem;
	}
</style>
