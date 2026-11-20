// The Library's core in the page: space-web's `Library` (deck-catalog `library::shelf`) behind typed calls – JSON in,
// JSON out, no rule here.
import type { AssetFamily, LibraryContext, LibraryFitAnswer, LibraryKind, LibraryQuery, LibraryUse, LibraryView, SlotFit } from './types.ts';

/** space-web's `Library` instance */
interface ShelfHandle {
	families(single: boolean): string;
	context(context: string): void;
	view(query: string): string;
	takes(path: string, fit: string): string;
	uses(key: string): string;
	upload_path(request: string): string;
	free?(): void;
}

/** the space-web exports the Library uses */
export interface LibraryModule {
	Library: new (assets: string) => ShelfHandle;
	library_select(request: string): string;
	library_start_kind(fit: string): string;
}

export interface LibraryCore {
	families(single: boolean): AssetFamily[];
	setContext(context: LibraryContext): void;
	view(query: LibraryQuery): LibraryView;
	takes(path: string, fit: SlotFit): LibraryFitAnswer;
	uses(key: string): LibraryUse[];
	uploadPath(name: string, slot?: string | null): string | null;
	select(chosen: string[], anchor: string | null, id: string, order: string[], keys: { shift?: boolean; toggle?: boolean }): { chosen: string[]; anchor: string | null };
	startKind(fit: SlotFit): LibraryKind | null;
	free(): void;
}

/** the core over `assets` (`GET /api/assets`' answer or its items) */
export function libraryCore(mod: LibraryModule, assets: unknown): LibraryCore {
	const shelf = new mod.Library(JSON.stringify(assets));
	return {
		families: (single) => JSON.parse(shelf.families(single)),
		setContext: (context) => shelf.context(JSON.stringify(context)),
		view: (query) => JSON.parse(shelf.view(JSON.stringify(query))),
		takes: (path, fit) => JSON.parse(shelf.takes(path, JSON.stringify(fit))),
		uses: (key) => JSON.parse(shelf.uses(key)),
		uploadPath: (name, slot) => JSON.parse(shelf.upload_path(JSON.stringify({ name, slot: slot ?? null }))),
		select: (chosen, anchor, id, order, keys) => JSON.parse(mod.library_select(JSON.stringify({ chosen, anchor, id, order, shift: !!keys.shift, toggle: !!keys.toggle }))),
		startKind: (fit) => JSON.parse(mod.library_start_kind(JSON.stringify(fit))),
		free: () => shelf.free?.()
	};
}

/** the space-web module at `base` (the deck's: loaded once per page) */
export async function loadLibraryModule(base: string): Promise<LibraryModule> {
	const { spaceModule } = await import('@scutoons/live-deck');
	return (await spaceModule(base)) as unknown as LibraryModule;
}
