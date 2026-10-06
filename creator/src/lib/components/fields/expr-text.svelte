<script lang="ts">
	import { tick, untrack } from 'svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { chipEndingAt, chipInside, chipStartingAt, chipSummary, expressionView, fieldOffset, growSelection, marksIn, snapCaret, textView, type Chip, type Mark } from '$lib/editor/expr-edit.ts';
	import { exprContext } from '$lib/editor/expr-context.svelte.ts';
	import { exprText, exprTextAt, summaryWords } from '$lib/editor/expr-messages.ts';
	import { parseText } from '@scutoons/content';
	import ExprBuilder from '$lib/components/fields/expr-builder.svelte';

	/**
	 * An expression-aware text field (agents/EXPRESSIONS.md §4, step 4; DESIGN.md "Expressions in text fields"). The value
	 * stays plain text with `{…}`; the field only SHOWS more:
	 *  - a top-level `{…}` is a semi-locked CHIP: click = the builder, double-click / long-press = edit in place (plain
	 *    again until the caret leaves it, Escape, blur or its `}`), Backspace / Delete next to it removes it whole, the
	 *    arrow keys step over it, a selection grows to whole chips; Ctrl+Enter next to a chip opens it in the builder
	 *    (elsewhere: the builder for a new one);
	 *  - typing `{` shows "Enter = builder" at the caret – Enter opens the builder there, typing on writes the code;
	 *  - problems: a red (error) / orange (warning) wavy underline on the chip or the range, the problem with its place
	 *    and suggestion as the chip's / range's tooltip (`title`), worded in the creator's language; debounced.
	 * `expression`: the whole value is one expression (a condition) – no chips, Enter opens the builder for it.
	 *
	 * Technique (decided 2026-10-06): a real <textarea> (caret, selection, IME, undo, spellcheck, paste as plain text,
	 * labels and screen readers as usual) with transparent text, over a mirror that draws the same text with the chips
	 * and underlines; a second transparent mirror on top carries only the chips' / ranges' hit areas (click, double
	 * click, tooltips). Mirror and textarea share font, padding and wrapping, the mirror sizes the field (it grows with
	 * its text). Edits the field makes itself (a chip removed, the builder's code) go through `insertText`, so Ctrl+Z
	 * undoes them. Single-line fields are a one-row textarea that refuses line breaks. The browser's spell check is off
	 * while the text has chips (it would underline the code in red too).
	 */
	let {
		value,
		onchange,
		multiline = true,
		rows = 3,
		expression = false,
		paragraphs = false,
		lang,
		focusKey,
		testid,
		placeholder,
		id,
		tone
	}: {
		value: string;
		onchange: (value: string) => void;
		multiline?: boolean;
		rows?: number;
		/** the value is one expression (a condition), not a text with `{…}` */
		expression?: boolean;
		/** a text box's lines (string[] shown with blank lines between): each analysed on its own, as validation does */
		paragraphs?: boolean;
		/** the text's language (the builder's preview) */
		lang: string;
		/** where the field is (expr-context fieldKey): the Problems panel's jumps find it */
		focusKey?: string;
		testid?: string;
		placeholder?: string;
		id?: string;
		/** the field's state frame (locale-text: a missing text / a long translation) */
		tone?: 'missing' | 'long';
	} = $props();

	const uid = $props.id();
	let area: HTMLTextAreaElement | null = $state(null);
	let field: HTMLDivElement | null = $state(null);
	let backdrop: HTMLDivElement | null = $state(null);

	// ── what the field shows ────────────────────────────────────────────────────────────────────────────────────────
	const known = $derived(exprContext.variables ?? undefined);
	/** chips follow every keystroke (a `}` makes one at once); problems settle 300 ms after the last change */
	const chips = $derived<Chip[]>(expression ? [] : textView(value, { paragraphs }).chips);
	let settled = $state(untrack(() => value));
	$effect(() => {
		const v = value;
		if (v === untrack(() => settled)) return;
		const timer = setTimeout(() => (settled = v), 300);
		return () => clearTimeout(timer);
	});
	const marks = $derived<Mark[]>(settled !== value ? [] : expression ? expressionView(value, known).marks : textView(value, { knownNames: known, paragraphs }).marks);
	const errors = $derived(marks.filter((x) => x.level === 'error').length);

	/** the caret's chip while editing in place: shown as plain text with its exact ranges */
	let editing = $state(false);
	let caret = $state(0);
	/** a top-level construct (closed or still open) around the caret, borders included */
	function regionAround(at: number): { start: number; end: number } | null {
		if (expression) return null;
		const chip = chips.find((c) => c.start <= at && at <= c.end);
		if (chip) return chip;
		const open = parseText(value).nodes.find((n) => n.type === 'bad' && n.start <= at && at <= n.end);
		return open ? { start: open.start, end: open.end } : null;
	}
	const editedChip = $derived(editing ? chips.findIndex((c) => c.start <= caret && caret <= c.end) : -1);

	interface Piece {
		start: number;
		end: number;
		text: string;
		/** the chip it belongs to (-1: none / the edited one) */
		chip: number;
		level: '' | 'error' | 'warning';
		first: boolean;
		last: boolean;
	}
	const worst = (list: Mark[]): Piece['level'] => (list.some((x) => x.level === 'error') ? 'error' : list.length ? 'warning' : '');
	/** the text cut at every chip / mark border: what both mirrors draw */
	const pieces = $derived.by<Piece[]>(() => {
		const cuts = new Set([0, value.length]);
		const whole = chips.filter((_, i) => i !== editedChip);
		for (const c of whole) [c.start, c.end].forEach((x) => cuts.add(x));
		// a chip stays one piece (its underline covers it all); problem ranges cut only the text outside chips
		for (const x of marks) for (const at of [Math.min(x.start, value.length), Math.min(x.end, value.length)]) if (!whole.some((c) => c.start < at && at < c.end)) cuts.add(at);
		const at = [...cuts].sort((a, b) => a - b);
		const out: Piece[] = [];
		for (let i = 0; i + 1 < at.length; i++) {
			const [start, end] = [at[i], at[i + 1]];
			if (start === end) continue;
			const chip = chips.findIndex((c, ci) => ci !== editedChip && c.start <= start && end <= c.end);
			const c = chips[chip];
			const level = c ? worst(marksIn(marks, c.start, c.end)) : worst(marksIn(marks, start, end));
			out.push({ start, end, text: value.slice(start, end), chip, level, first: !!c && c.start === start, last: !!c && c.end === end });
		}
		return out;
	});

	/** a chip's tooltip: its problems (worded, with their place), else what it does + how to open it */
	function chipTitle(i: number): string {
		const c = chips[i];
		const list = marksIn(marks, c.start, c.end);
		if (list.length) return list.map((x) => exprTextAt({ ...x, start: x.start - c.start, end: x.end - c.start })).join('\n');
		return m['creator.expr_chip_hint']({ summary: chipSummary(value.slice(c.start, c.end), summaryWords) });
	}
	const rangeTitle = (p: Piece) => marksIn(marks, p.start, p.end).map((x) => exprTextAt(x)).join('\n');
	/** for screen readers: every problem of the field */
	const described = $derived(marks.map((x) => exprTextAt(x)).join('. '));

	// ── caret + editing ─────────────────────────────────────────────────────────────────────────────────────────────
	let lastCaret = 0;
	let pointerDown = false;

	function setCaret(start: number, end = start) {
		area?.setSelectionRange(start, end);
		caret = start;
		lastCaret = start;
	}

	/** an edit of the field's own (a chip removed, the builder's code): through the browser's undo stack */
	function replace(start: number, end: number, text: string) {
		if (!area) return;
		area.focus();
		area.setSelectionRange(start, end);
		const done = text ? document.execCommand('insertText', false, text) : document.execCommand('delete');
		if (!done) {
			area.setRangeText(text, start, end, 'end');
			area.dispatchEvent(new Event('input', { bubbles: true }));
		}
		caret = area.selectionStart;
		lastCaret = caret;
	}

	function onSelection() {
		if (!area || document.activeElement !== area) return;
		const { selectionStart: s, selectionEnd: e } = area;
		caret = s;
		if (editing) {
			if (!regionAround(s) || (e !== s && !regionAround(e))) editing = false;
			lastCaret = s;
			return;
		}
		if (pointerDown) return;
		if (s === e) {
			const snapped = snapCaret(chips, s, lastCaret);
			if (snapped !== s) area.setSelectionRange(snapped, snapped);
			lastCaret = snapped;
			caret = snapped;
		} else {
			const [a, b] = growSelection(chips, s, e);
			if (a !== s || b !== e) area.setSelectionRange(a, b, area.selectionDirection);
		}
	}
	$effect(() => {
		document.addEventListener('selectionchange', onSelection);
		return () => document.removeEventListener('selectionchange', onSelection);
	});

	// ── "Enter = builder" after a typed `{` ─────────────────────────────────────────────────────────────────────────
	let hint = $state<{ left: string; top: string } | null>(null);
	const rootPx = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
	const rem = (px: number) => `${px / rootPx()}rem`;

	/** the screen box of a text offset (the mirror's text there) */
	function rectAt(offset: number): DOMRect | null {
		if (!backdrop) return null;
		for (const span of backdrop.querySelectorAll<HTMLElement>('[data-start]')) {
			const start = Number(span.dataset.start);
			const end = Number(span.dataset.end);
			const node = span.firstChild;
			if (!node || offset < start || offset > end) continue;
			const range = document.createRange();
			range.setStart(node, Math.min(offset - start, node.textContent?.length ?? 0));
			range.collapse(true);
			const rects = range.getClientRects();
			return rects[0] ?? range.getBoundingClientRect();
		}
		return backdrop.getBoundingClientRect();
	}

	async function showHint() {
		await tick();
		const r = rectAt(caret);
		const box = field?.getBoundingClientRect();
		if (!r || !box) return;
		hint = { left: rem(r.left - box.left), top: rem(r.bottom - box.top) };
	}

	function onInput(e: Event & { currentTarget: HTMLTextAreaElement }) {
		const t = e.currentTarget;
		caret = t.selectionStart;
		lastCaret = caret;
		const data = (e as unknown as InputEvent).data;
		hint = null;
		if (data === '{' && !expression && t.value[caret - 2] !== '\\') showHint();
		if (data === '}' && editing) editing = false;
		onchange(t.value);
	}

	function onBeforeInput(e: InputEvent) {
		if (multiline) return;
		if (e.inputType === 'insertLineBreak' || e.inputType === 'insertParagraph') e.preventDefault();
		else if ((e.inputType === 'insertFromPaste' || e.inputType === 'insertFromDrop') && /[\r\n]/.test(e.dataTransfer?.getData('text/plain') ?? e.data ?? '')) {
			e.preventDefault();
			const text = (e.dataTransfer?.getData('text/plain') ?? e.data ?? '').replace(/\s*[\r\n]+\s*/g, ' ');
			if (area) replace(area.selectionStart, area.selectionEnd, text);
		}
	}

	function onKeydown(e: KeyboardEvent) {
		if (!area) return;
		const { selectionStart: s, selectionEnd: end } = area;
		const collapsed = s === end;
		const plain = !e.shiftKey && !e.altKey && !e.ctrlKey && !e.metaKey;
		if (e.key === 'Enter' && hint && plain) {
			e.preventDefault();
			hint = null;
			openBuilder({ start: s - 1, end: s, code: '' });
			return;
		}
		hint = null;
		if (e.key === 'Enter' && (e.ctrlKey || e.metaKey)) {
			e.preventDefault();
			const chip = collapsed ? (chipEndingAt(chips, s) ?? chipStartingAt(chips, s) ?? chipInside(chips, s)) : undefined;
			if (expression) openBuilder({ start: 0, end: value.length, code: value });
			else if (chip) openBuilder({ start: chip.start, end: chip.end, code: value.slice(chip.start, chip.end) });
			else openBuilder({ start: s, end, code: '' });
			return;
		}
		if (e.key === 'Enter' && expression && plain) {
			e.preventDefault();
			openBuilder({ start: 0, end: value.length, code: value });
			return;
		}
		if (e.key === 'Enter' && !multiline) {
			e.preventDefault();
			return;
		}
		if (editing) {
			if (e.key === 'Escape') {
				const region = regionAround(s);
				editing = false;
				if (region) setCaret(region.end);
				e.stopPropagation();
			}
			return;
		}
		if (!collapsed || !plain) return;
		const before = chipEndingAt(chips, s);
		const after = chipStartingAt(chips, s);
		if (e.key === 'ArrowLeft' && before) {
			e.preventDefault();
			setCaret(before.start);
		} else if (e.key === 'ArrowRight' && after) {
			e.preventDefault();
			setCaret(after.end);
		} else if (e.key === 'Backspace' && before) {
			e.preventDefault();
			replace(before.start, before.end, '');
		} else if (e.key === 'Delete' && after) {
			e.preventDefault();
			replace(after.start, after.end, '');
		}
	}

	// ── chips: click = builder, double-click / long-press = edit in place ─────────────────────────────────────────
	/** the text offset under a point of the hit layer (its transparent text = the field's) */
	function offsetAt(x: number, y: number): number | null {
		const doc = document as Document & { caretPositionFromPoint?: (x: number, y: number) => { offsetNode: Node; offset: number } | null };
		let node: Node | null = null;
		let offset = 0;
		if (doc.caretPositionFromPoint) {
			const p = doc.caretPositionFromPoint(x, y);
			if (p) [node, offset] = [p.offsetNode, p.offset];
		} else if (document.caretRangeFromPoint) {
			const r = document.caretRangeFromPoint(x, y);
			if (r) [node, offset] = [r.startContainer, r.startOffset];
		}
		const span = (node?.nodeType === Node.TEXT_NODE ? node.parentElement : (node as HTMLElement | null))?.closest<HTMLElement>('[data-start]');
		return span ? Number(span.dataset.start) + offset : null;
	}

	let clickTimer: ReturnType<typeof setTimeout> | undefined;
	let pressTimer: ReturnType<typeof setTimeout> | undefined;
	let pressed = false;

	function editAt(x: number, y: number, chip: Chip) {
		clearTimeout(clickTimer);
		editing = true;
		area?.focus();
		const at = offsetAt(x, y) ?? chip.end;
		setCaret(Math.max(chip.start, Math.min(chip.end, at)));
	}

	function chipDown(e: PointerEvent, i: number) {
		e.preventDefault();
		pressed = false;
		area?.focus();
		setCaret(chips[i].end);
		if (e.pointerType === 'touch' || e.pointerType === 'pen') {
			const chip = chips[i];
			const [x, y] = [e.clientX, e.clientY];
			pressTimer = setTimeout(() => {
				pressed = true;
				editAt(x, y, chip);
			}, 500);
		}
	}
	const chipUp = () => clearTimeout(pressTimer);

	function chipClick(e: MouseEvent, i: number) {
		e.preventDefault();
		if (pressed) return;
		const chip = chips[i];
		if (e.detail >= 2) return editAt(e.clientX, e.clientY, chip);
		clearTimeout(clickTimer);
		clickTimer = setTimeout(() => openBuilder({ start: chip.start, end: chip.end, code: value.slice(chip.start, chip.end) }), 250);
	}

	/** a problem range outside chips: the click places the caret there (the hit area sits over the textarea) */
	function rangeDown(e: PointerEvent) {
		e.preventDefault();
		area?.focus();
		const at = offsetAt(e.clientX, e.clientY);
		if (at !== null) setCaret(at);
	}

	// ── the builder ─────────────────────────────────────────────────────────────────────────────────────────────────
	let builder = $state<{ start: number; end: number; code: string; anchor: DOMRect } | null>(null);

	async function openBuilder(at: { start: number; end: number; code: string }) {
		clearTimeout(clickTimer);
		await tick();
		const anchor = rectAt(at.start) ?? area?.getBoundingClientRect();
		if (!anchor) return;
		builder = { ...at, anchor };
	}

	function apply(code: string) {
		const b = builder;
		builder = null;
		if (!b) return;
		replace(b.start, b.end, code);
		editing = false;
	}

	function closeBuilder() {
		const b = builder;
		builder = null;
		area?.focus();
		if (b) setCaret(Math.min(b.end, value.length));
	}

	// ── a jump from the Problems panel: focus + select the problem's range ─────────────────────────────────────────
	$effect(() => {
		const f = exprContext.focus;
		if (!f || !focusKey || f.key !== focusKey || !area) return;
		untrack(() => {
			exprContext.focus = null;
			const text = value;
			const start = Math.min(fieldOffset(text, f.line, f.start, paragraphs), text.length);
			const end = Math.min(fieldOffset(text, f.line, f.end, paragraphs), text.length);
			tick().then(() => {
				area?.scrollIntoView({ block: 'center' });
				area?.focus();
				editing = !!chipInside(chips, start) || !!chipInside(chips, end);
				area?.setSelectionRange(start, Math.max(start, end));
				caret = start;
			});
		});
	});
</script>

<div class="expr-field" class:single={!multiline} class:expression class:missing={tone === 'missing'} class:long={tone === 'long'} bind:this={field} style:--rows={multiline ? rows : 1} data-testid={testid ? `${testid}-field` : undefined}>
	<div class="mirror backdrop" aria-hidden="true" bind:this={backdrop}>
		{#each pieces as p (p.start)}<span
				data-start={p.start}
				data-end={p.end}
				class:chip={p.chip >= 0}
				class:first={p.first}
				class:last={p.last}
				class:error={p.level === 'error'}
				class:warning={p.level === 'warning'}>{p.text}</span
			>{/each}{'​'}
	</div>
	<textarea
		bind:this={area}
		{id}
		rows={multiline ? rows : 1}
		{lang}
		{value}
		{placeholder}
		spellcheck={!expression && !chips.length}
		aria-invalid={errors > 0 ? 'true' : undefined}
		aria-describedby={described ? `${uid}-problems` : undefined}
		oninput={onInput}
		onbeforeinput={onBeforeInput}
		onkeydown={onKeydown}
		onpointerdown={() => (pointerDown = true)}
		onpointerup={() => {
			pointerDown = false;
			onSelection();
		}}
		onblur={() => {
			editing = false;
			hint = null;
			pointerDown = false;
		}}
		data-testid={testid}
	></textarea>
	<div class="mirror hits" aria-hidden="true">
		{#each pieces as p (p.start)}{#if p.chip >= 0}<span
					class="hit chip-hit"
					data-start={p.start}
					data-end={p.end}
					data-chip={p.chip}
					title={chipTitle(p.chip)}
					onpointerdown={(e) => chipDown(e, p.chip)}
					onpointerup={chipUp}
					onpointercancel={chipUp}
					onclick={(e) => chipClick(e, p.chip)}
					role="presentation">{p.text}</span
				>{:else if p.level}<span class="hit range-hit" data-start={p.start} data-end={p.end} title={rangeTitle(p)} onpointerdown={rangeDown} role="presentation">{p.text}</span
				>{:else}<span data-start={p.start} data-end={p.end}>{p.text}</span>{/if}{/each}{'​'}
	</div>
	{#if hint}<span class="hint" style:left={hint.left} style:top={hint.top} data-testid="expr-hint">{m['creator.expr_hint_builder']()}</span>{/if}
	{#if described}<span id="{uid}-problems" class="sr-only">{described}</span>{/if}
</div>
{#if builder}
	<ExprBuilder code={builder.code} {expression} {lang} anchor={builder.anchor} onapply={apply} onclose={closeBuilder} />
{/if}

<style>
	/* one grid cell: the mirror sizes it, the textarea and the hit layer lie over it exactly */
	.expr-field {
		position: relative;
		display: grid;
		min-width: 0;
	}
	.expr-field > * {
		grid-area: 1 / 1;
	}
	.mirror,
	textarea {
		box-sizing: border-box;
		margin: 0;
		min-width: 0;
		font: inherit;
		font-size: var(--type-body-size);
		line-height: var(--type-body-line);
		letter-spacing: normal;
		white-space: pre-wrap;
		overflow-wrap: anywhere;
		word-break: normal;
		tab-size: 4;
		padding: var(--space-05); /* control-ok: multi-line, exempt */
		border: var(--border-width-hairline) solid transparent;
	}
	.mirror {
		/* the textarea's rows as the least height */
		min-height: calc(var(--rows) * var(--type-body-line) * var(--type-body-size) + 2 * var(--space-05) + 2 * var(--border-width-hairline));
		color: var(--ink);
		pointer-events: none;
	}
	textarea {
		height: 100%;
		min-height: 0;
		resize: none;
		overflow: hidden;
		background: transparent;
		color: transparent;
		caret-color: var(--ink);
		border-color: var(--border-muted);
	}
	textarea:focus {
		border-color: var(--ink);
	}
	textarea::selection {
		color: transparent;
		background: color-mix(in srgb, var(--primary-cyan) 35%, transparent);
	}
	.backdrop {
		z-index: 0;
		background: var(--paper);
	}
	textarea {
		z-index: 1;
	}
	.hits {
		z-index: 2;
	}
	.backdrop,
	textarea,
	.hits {
		position: relative;
	}
	/* state = the frame's colour + a thick left rule (locale-text), the mirrors keep the same inset */
	.missing > *,
	.long > * {
		border-left-width: 0.5em;
	}
	.missing textarea {
		border-color: var(--state-error);
	}
	.long textarea {
		border-color: var(--state-warning);
	}
	/* single line: one row at --control-height, like an input */
	.single .mirror,
	.single textarea {
		padding-block: calc((var(--control-height) - 2 * var(--border-width-hairline) - var(--type-body-line) * var(--type-body-size)) / 2); /* control-ok: a one-row textarea at --control-height */
	}
	.single .mirror {
		min-height: var(--control-height);
	}
	/* a chip: the code on a tint, rounded – drawn around the letters without moving them */
	.chip {
		background: var(--surface-tint-cyan);
		box-shadow:
			0 0 0 var(--border-width-hairline) var(--primary-cyan),
			0 0 0 0.125rem var(--surface-tint-cyan);
		box-decoration-break: clone;
		-webkit-box-decoration-break: clone;
	}
	.chip.first {
		border-top-left-radius: var(--radius-sm);
		border-bottom-left-radius: var(--radius-sm);
	}
	.chip.last {
		border-top-right-radius: var(--radius-sm);
		border-bottom-right-radius: var(--radius-sm);
	}
	.error,
	.warning {
		text-decoration-line: underline;
		text-decoration-style: wavy;
		text-decoration-thickness: var(--border-width-hairline);
		text-underline-offset: 0.2em;
		text-decoration-skip-ink: none;
	}
	.error {
		text-decoration-color: var(--state-error);
	}
	.warning {
		text-decoration-color: var(--state-warning);
	}
	/* the hit layer: transparent letters, only chips and problem ranges take the pointer */
	.hits {
		color: transparent;
		border-color: transparent;
	}
	.hit {
		pointer-events: auto;
	}
	.chip-hit {
		cursor: pointer;
	}
	.range-hit {
		cursor: text;
	}
	.hint {
		position: absolute;
		z-index: 2;
		padding: 0 var(--space-025);
		background: var(--ink);
		color: var(--paper);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		white-space: nowrap;
		pointer-events: none;
	}
	.sr-only {
		position: absolute;
		width: 1px; /* px-ok: the visually-hidden pattern */
		height: 1px; /* px-ok: the visually-hidden pattern */
		overflow: hidden;
		clip-path: inset(50%);
		white-space: nowrap;
	}
</style>
