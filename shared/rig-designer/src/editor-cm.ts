// The CodeMirror 6 editor of the Rust tab. This file is the ONLY one that imports CodeMirror, and it is only ever
// loaded through `loadEditor()` (a dynamic import, editor.ts) when the Rust tab first opens – the deck page's bundle
// does not grow. One view; every file keeps its own EditorState (text, cursor, undo history) while another shows.
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { bracketMatching, defaultHighlightStyle, indentOnInput, syntaxHighlighting } from '@codemirror/language';
import { rust } from '@codemirror/lang-rust';
import { lintGutter, setDiagnostics, type Diagnostic as CmDiagnostic } from '@codemirror/lint';
import { EditorState } from '@codemirror/state';
import { drawSelection, EditorView, highlightActiveLine, highlightActiveLineGutter, keymap, lineNumbers } from '@codemirror/view';
import { rangeFor, type Marker } from './diagnostics.ts';

export interface EditorHandle {
	/** shows a file (its own state is kept; `text` seeds it the first time, or when it was changed from outside) */
	show(file: string, text: string): void;
	/** the markers of the file shown */
	markers(list: Marker[]): void;
	/** moves the cursor to a 1-based line / column and focuses */
	goTo(line: number, column: number): void;
	/** forgets a file's state (removed / renamed) */
	forget(file: string): void;
	destroy(): void;
}

export interface EditorOptions {
	onChange: (file: string, text: string) => void;
	label: string;
}

// ink on paper, mono, rem (DESIGN.md "Brutalist tools": no px type, 2px frames are the page's)
const theme = EditorView.theme({
	'&': { height: '100%', color: 'var(--ink)', backgroundColor: 'var(--paper)', fontSize: 'var(--designer-code-size, 0.8125rem)' },
	'&.cm-focused': { outline: 'var(--border-width-focus) solid var(--ink)', outlineOffset: '-3px' },
	'.cm-scroller': { fontFamily: 'var(--font-mono)', lineHeight: '1.5', overflow: 'auto' },
	'.cm-content': { caretColor: 'var(--ink)', padding: '0.5rem 0' },
	'.cm-gutters': { backgroundColor: 'var(--paper)', color: 'color-mix(in srgb, var(--ink) 50%, transparent)', border: 'none', borderRight: '1px solid color-mix(in srgb, var(--ink) 20%, transparent)' },
	'.cm-activeLine': { backgroundColor: 'color-mix(in srgb, var(--ink) 6%, transparent)' },
	'.cm-activeLineGutter': { backgroundColor: 'color-mix(in srgb, var(--ink) 6%, transparent)', color: 'var(--ink)' },
	'.cm-selectionBackground, &.cm-focused .cm-selectionBackground': { backgroundColor: 'color-mix(in srgb, var(--primary-cyan, #00d1ff) 35%, transparent)' },
	'.cm-lintRange-error': { backgroundImage: 'none', textDecoration: 'underline wavy var(--state-error)', textUnderlineOffset: '0.2em' },
	'.cm-lintRange-warning': { backgroundImage: 'none', textDecoration: 'underline wavy var(--state-warning)', textUnderlineOffset: '0.2em' },
	'.cm-tooltip': { fontFamily: 'var(--font-mono)', fontSize: '0.75rem', border: 'var(--border-width-regular) solid var(--ink)', backgroundColor: 'var(--paper)', color: 'var(--ink)', borderRadius: '0' },
	'.cm-diagnostic': { padding: '0.25rem 0.5rem' }
});

export function createEditor(parent: HTMLElement, o: EditorOptions): EditorHandle {
	const states = new Map<string, EditorState>();
	let current = '';
	let view: EditorView | null = null;

	const extensions = (file: string) => [
		lineNumbers(),
		EditorView.lineWrapping,
		highlightActiveLineGutter(),
		highlightActiveLine(),
		drawSelection(),
		history(),
		indentOnInput(),
		bracketMatching(),
		syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
		rust(),
		lintGutter(),
		keymap.of([...defaultKeymap, ...historyKeymap, indentWithTab]),
		EditorView.contentAttributes.of({ 'aria-label': o.label, spellcheck: 'false', autocapitalize: 'off', autocorrect: 'off' }),
		EditorView.updateListener.of((u) => {
			if (u.docChanged && current === file) o.onChange(file, u.state.doc.toString());
		}),
		theme
	];

	const stateFor = (file: string, text: string): EditorState => EditorState.create({ doc: text, extensions: extensions(file) });

	return {
		show(file, text) {
			// the same file with the text the editor already holds (every keystroke asks): nothing to do
			if (view && current === file && view.state.doc.toString() === text) return;
			if (view && current) states.set(current, view.state);
			current = file;
			let state = states.get(file);
			// the text was replaced from outside (a load): start over with it
			if (!state || state.doc.toString() !== text) state = stateFor(file, text);
			states.set(file, state);
			if (!view) view = new EditorView({ state, parent });
			else view.setState(state);
		},
		markers(list) {
			if (!view) return;
			const text = view.state.doc.toString();
			const cm: CmDiagnostic[] = list.map((d) => {
				const { from, to } = rangeFor(text, d.line, d.column);
				return { from, to: Math.max(to, Math.min(from + 1, text.length)), severity: d.level === 'info' ? 'info' : d.level, message: d.message };
			});
			view.dispatch(setDiagnostics(view.state, cm));
		},
		goTo(line, column) {
			if (!view) return;
			const text = view.state.doc.toString();
			const { from } = rangeFor(text, line, column);
			view.dispatch({ selection: { anchor: from }, effects: EditorView.scrollIntoView(from, { y: 'center' }) });
			view.focus();
		},
		forget(file) {
			states.delete(file);
		},
		destroy() {
			view?.destroy();
			view = null;
			states.clear();
		}
	};
}
