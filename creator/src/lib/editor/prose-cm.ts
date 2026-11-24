// The writing page's text (lap 2 C14): CodeMirror 6 as a plain writing surface – no gutter, no box, the text wraps in
// the reading column; Markdown shown as you type (lib/editor/writing.ts reads each line), its marks quiet. Loaded only
// through `loadProse()` (a dynamic import) when a writing page opens, so no other page's bundle grows.
import { defaultKeymap, history, historyKeymap } from '@codemirror/commands';
import { Compartment, EditorSelection, EditorState, RangeSetBuilder, Transaction } from '@codemirror/state';
import { Decoration, EditorView, ViewPlugin, drawSelection, keymap, placeholder as placeholderText, type DecorationSet, type ViewUpdate } from '@codemirror/view';
import { inlineMarks, lineInfo, type WritingEdit } from './writing.ts';

export interface ProseSelection {
	from: number;
	to: number;
	/** the selection's box on the screen (client px); null while nothing is selected or the text has no focus */
	box: { left: number; right: number; top: number; bottom: number } | null;
}

export interface ProseOptions {
	text: string;
	readOnly: boolean;
	label: string;
	lang: string;
	placeholder?: string;
	onChange: (text: string) => void;
	onSelection: (selection: ProseSelection) => void;
	/** a key pressed while the text is read-only (the page asks whether to edit) */
	onLockedKey?: (event: KeyboardEvent) => boolean;
	onBlur?: () => void;
}

export interface ProseHandle {
	text(): string;
	/** the text from outside (load, restore, undo of the page, a language switch): the selection kept where it can be */
	setText(text: string, toStart?: boolean): void;
	setReadOnly(readOnly: boolean): void;
	/** puts the caret at the start of a 1-based line and scrolls it into the middle */
	jump(line: number): void;
	/** one edit of the pill (writing.ts), as one undo step */
	apply(make: (text: string, from: number, to: number) => WritingEdit): void;
	/** the 1-based line the caret is on */
	caretLine(): number;
	focus(): void;
	destroy(): void;
}

const line = (kind: string) => Decoration.line({ class: `wr-line wr-${kind}` });
const LINES = { heading: line('heading'), speaker: line('speaker'), direction: line('direction'), quote: line('quote'), item: line('item'), text: line('text'), blank: line('blank') };
const MARK = Decoration.mark({ class: 'wr-mark' });
const STRONG = Decoration.mark({ class: 'wr-strong' });
const EM = Decoration.mark({ class: 'wr-em' });
const LINK = Decoration.mark({ class: 'wr-link' });

function decorate(view: EditorView): DecorationSet {
	const builder = new RangeSetBuilder<Decoration>();
	for (const { from, to } of view.visibleRanges) {
		for (let pos = from; pos <= to; ) {
			const l = view.state.doc.lineAt(pos);
			const info = lineInfo(l.text);
			builder.add(l.from, l.from, LINES[info.kind]);
			if (info.mark) builder.add(l.from, l.from + info.mark, MARK);
			const body = l.text.slice(info.mark);
			const at = l.from + info.mark;
			for (const mark of inlineMarks(body)) {
				const [a, b] = mark.inner;
				if (a > mark.from) builder.add(at + mark.from, at + a, MARK);
				builder.add(at + a, at + b, mark.kind === 'strong' ? STRONG : mark.kind === 'em' ? EM : LINK);
				if (mark.to > b) builder.add(at + b, at + mark.to, MARK);
			}
			pos = l.to + 1;
		}
	}
	return builder.finish();
}

const markdown = ViewPlugin.fromClass(
	class {
		decorations: DecorationSet;
		constructor(view: EditorView) {
			this.decorations = decorate(view);
		}
		update(u: ViewUpdate) {
			if (u.docChanged || u.viewportChanged) this.decorations = decorate(u.view);
		}
	},
	{ decorations: (v) => v.decorations }
);

// the kit's colours (--tool-*); sizes from the page (--wr-*: the column sets them)
const theme = EditorView.theme({
	'&': { color: 'var(--tool-text)', backgroundColor: 'transparent', fontSize: 'inherit' },
	'&.cm-focused': { outline: 'none' },
	'.cm-scroller': { fontFamily: 'inherit', lineHeight: 'inherit', overflow: 'visible' },
	'.cm-content': { caretColor: 'var(--tool-text)', padding: '0', whiteSpace: 'pre-wrap' },
	'.cm-line': { padding: '0' },
	'.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--tool-text)', borderLeftWidth: '1px' },
	'.cm-selectionBackground, &.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-content ::selection': { backgroundColor: 'var(--tool-selected)' },
	'.cm-placeholder': { color: 'var(--tool-muted)' },
	'.wr-mark': { color: 'var(--tool-muted)', fontWeight: 'normal', fontStyle: 'normal', fontSize: '0.8em' },
	'.wr-heading': { fontSize: 'var(--wr-heading, 1.15em)', lineHeight: '1.3', paddingTop: '0.6em !important', paddingBottom: '0.15em !important' },
	'.wr-speaker': { fontSize: 'var(--wr-label, 0.62em)', letterSpacing: '0.12em', textTransform: 'uppercase', color: 'var(--tool-muted)', paddingTop: '1.4em !important' },
	'.wr-direction': { color: 'var(--tool-muted)', fontStyle: 'italic' },
	'.wr-quote': { fontStyle: 'italic' },
	'.wr-strong': { fontWeight: '600' },
	'.wr-em': { fontStyle: 'italic' },
	'.wr-link': { textDecoration: 'underline', textUnderlineOffset: '0.2em' }
});

export function createProse(parent: HTMLElement, o: ProseOptions): ProseHandle {
	const lock = new Compartment();
	let quiet = false;
	const report = (view: EditorView) => {
		const { from, to } = view.state.selection.main;
		let box: ProseSelection['box'] = null;
		if (from !== to && view.hasFocus) {
			const a = view.coordsAtPos(from, 1);
			const b = view.coordsAtPos(to, -1);
			if (a && b) box = { left: Math.min(a.left, b.left), right: Math.max(a.right, b.right), top: Math.min(a.top, b.top), bottom: Math.max(a.bottom, b.bottom) };
		}
		o.onSelection({ from, to, box });
	};
	const view = new EditorView({
		parent,
		state: EditorState.create({
			doc: o.text,
			extensions: [
				lock.of([EditorState.readOnly.of(o.readOnly)]),
				EditorView.lineWrapping,
				drawSelection(),
				history(),
				keymap.of([...defaultKeymap, ...historyKeymap]),
				markdown,
				theme,
				placeholderText(o.placeholder ?? ''),
				EditorView.contentAttributes.of({ 'aria-label': o.label, lang: o.lang, spellcheck: 'true', 'data-testid': 'writing-text' }),
				EditorView.domEventHandlers({
					keydown: (event, v) => (v.state.readOnly && o.onLockedKey ? o.onLockedKey(event) : false),
					blur: () => {
						o.onBlur?.();
						return false;
					}
				}),
				EditorView.updateListener.of((u) => {
					if (u.docChanged && !quiet) o.onChange(u.state.doc.toString());
					if (u.selectionSet || u.focusChanged || u.geometryChanged || u.docChanged) report(u.view);
				})
			]
		})
	});
	return {
		text: () => view.state.doc.toString(),
		setText(text, toStart = false) {
			if (text === view.state.doc.toString()) return;
			const { anchor, head } = view.state.selection.main;
			const clamp = (n: number) => (toStart ? 0 : Math.min(n, text.length));
			quiet = true;
			view.dispatch({ changes: { from: 0, to: view.state.doc.length, insert: text }, selection: EditorSelection.single(clamp(anchor), clamp(head)), annotations: Transaction.addToHistory.of(false) });
			quiet = false;
		},
		setReadOnly(readOnly) {
			if (view.state.readOnly !== readOnly) view.dispatch({ effects: lock.reconfigure([EditorState.readOnly.of(readOnly)]) });
		},
		jump(n) {
			const target = view.state.doc.line(Math.max(1, Math.min(n, view.state.doc.lines)));
			view.dispatch({ selection: EditorSelection.cursor(target.from), effects: EditorView.scrollIntoView(target.from, { y: 'start', yMargin: 96 }) });
			view.focus();
		},
		apply(make) {
			const { from, to } = view.state.selection.main;
			const edit = make(view.state.doc.toString(), from, to);
			view.dispatch({ changes: edit.changes, selection: EditorSelection.single(edit.selection.anchor, edit.selection.head), scrollIntoView: true, userEvent: 'input.format' });
			view.focus();
		},
		caretLine: () => view.state.doc.lineAt(view.state.selection.main.head).number,
		focus: () => view.focus(),
		destroy: () => view.destroy()
	};
}
