// The lazy door to the code editor: nothing of CodeMirror is imported statically anywhere else, so the deck page
// (which only imports this package's components) loads the editor's chunk when the Rust tab first opens.
import type { EditorHandle, EditorOptions } from './editor-cm.ts';

export type { EditorHandle, EditorOptions };

let loading: Promise<typeof import('./editor-cm.ts')> | null = null;

/** the editor module (once; a failed load can be tried again) */
export function loadEditor(): Promise<(parent: HTMLElement, o: EditorOptions) => EditorHandle> {
	loading ??= import('./editor-cm.ts');
	return loading.then(
		(mod) => mod.createEditor,
		(e) => {
			loading = null;
			throw e;
		}
	);
}
