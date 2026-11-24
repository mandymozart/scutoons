// The CodeMirror 6 editor of the Rust tab. This file is the ONLY one that imports CodeMirror, and it is only ever
// loaded through `loadEditor()` (a dynamic import, editor.ts) when the Rust tab first opens – the deck page's bundle
// does not grow. One view; every file keeps its own EditorState (text, cursor, undo history) while another shows.
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { bracketMatching, defaultHighlightStyle, HighlightStyle, indentOnInput, syntaxHighlighting } from '@codemirror/language';
import { rust } from '@codemirror/lang-rust';
import { lintGutter, setDiagnostics, type Diagnostic as CmDiagnostic } from '@codemirror/lint';
import { Compartment, EditorState } from '@codemirror/state';
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
	/** a read-only module (a factory module, #350): the text can be read and selected, not changed */
	setReadOnly(readOnly: boolean): void;
	destroy(): void;
}

export interface EditorOptions {
	onChange: (file: string, text: string) => void;
	label: string;
}

// the kit's colours (--tool-*): the stage shows through, the code in the mono face, rem
const theme = EditorView.theme({
	'&': { height: '100%', color: 'var(--tool-text)', backgroundColor: 'transparent', fontSize: 'var(--designer-code-size, 0.8125rem)' },
	'&.cm-focused': { outline: 'none' },
	'.cm-scroller': { fontFamily: 'var(--font-mono)', lineHeight: '1.5', overflow: 'auto' },
	'.cm-content': { caretColor: 'var(--tool-text)', padding: '0.5rem 0' },
	'.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--tool-text)' },
	'.cm-gutters': { backgroundColor: 'transparent', color: 'var(--tool-muted)', border: 'none', borderRight: '1px solid var(--tool-line)' },
	'.cm-activeLine': { backgroundColor: 'var(--tool-control)' },
	'.cm-activeLineGutter': { backgroundColor: 'var(--tool-control)', color: 'var(--tool-text)' },
	'.cm-selectionBackground, &.cm-focused .cm-selectionBackground, .cm-content ::selection': { backgroundColor: 'var(--tool-selection)' },
	'.cm-matchingBracket': { backgroundColor: 'var(--tool-selected)', outline: 'none' },
	'.cm-lintRange-error': { backgroundImage: 'none', textDecoration: 'underline wavy var(--tool-stop)', textUnderlineOffset: '0.2em' },
	'.cm-lintRange-warning': { backgroundImage: 'none', textDecoration: 'underline wavy var(--tool-warn)', textUnderlineOffset: '0.2em' },
	'.cm-tooltip': { fontSize: 'var(--type-caption-size)', border: '1px solid var(--tool-line)', backgroundColor: 'var(--tool-popover)', color: 'var(--tool-text)', textShadow: 'var(--tool-halo)', borderRadius: 'var(--editor-radius)' },
	// a state shows as its colour, never as a thick side bar (RULES H10)
	'.cm-diagnostic': { padding: '0.25rem 0.5rem' },
	'.cm-diagnostic-error, .cm-diagnostic-warning, .cm-diagnostic-info, .cm-diagnostic-hint': { borderLeft: 'none' },
	'.cm-diagnostic-error': { color: 'var(--tool-stop)' },
	'.cm-diagnostic-warning': { color: 'var(--tool-warn)' }
});

// the code keeps CodeMirror's own colours; mixed toward the text colour by --code-hue (the Rust tab sets it lower by
// night) so they read on the black stage as on the white one
const highlight = HighlightStyle.define(defaultHighlightStyle.specs.map((spec) => (typeof spec.color === 'string' ? { ...spec, color: `color-mix(in oklab, ${spec.color} var(--code-hue, 100%), var(--tool-text))` } : spec)));

export function createEditor(parent: HTMLElement, o: EditorOptions): EditorHandle {
	const states = new Map<string, EditorState>();
	let current = '';
	let view: EditorView | null = null;
	// read-only is the editor's, not a file's: every state carries the compartment, show() brings it up to date
	const lock = new Compartment();
	let readOnly = false;
	const sync = () => {
		if (view && view.state.facet(EditorState.readOnly) !== readOnly) view.dispatch({ effects: lock.reconfigure(EditorState.readOnly.of(readOnly)) });
	};

	const extensions = (file: string) => [
		lock.of(EditorState.readOnly.of(readOnly)),
		lineNumbers(),
		EditorView.lineWrapping,
		highlightActiveLineGutter(),
		highlightActiveLine(),
		drawSelection(),
		history(),
		indentOnInput(),
		bracketMatching(),
		syntaxHighlighting(highlight, { fallback: true }),
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
			sync();
		},
		setReadOnly(value) {
			readOnly = value;
			sync();
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
