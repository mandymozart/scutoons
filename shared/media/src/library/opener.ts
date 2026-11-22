// The Library's open rule in the page: space-web's `library_open` / `library_open_kinds` (deck-catalog `library::open`)
// behind typed calls – where an address opens in an editor, or why not; no rule here.
import type { LibraryModule } from './core.ts';
import type { LibraryEditor, LibraryOpenKind, LibraryOpening } from './types.ts';

/** the core's open rule for one editor: where an address opens, and every kind's answer */
export interface LibraryOpener {
	open(address: string): LibraryOpening;
	kinds(): LibraryOpenKind[];
}

export function libraryOpener(mod: LibraryModule, editor: LibraryEditor): LibraryOpener {
	return {
		open: (address) => JSON.parse(mod.library_open(address, editor)),
		kinds: () => JSON.parse(mod.library_open_kinds(editor))
	};
}
