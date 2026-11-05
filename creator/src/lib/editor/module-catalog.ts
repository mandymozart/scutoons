// The Sound workspace's module list, New module and Derive (#350): the catalog kind `modules` as the workspace shows it,
// and a new module's first draft – a copy of a factory module that names it as its `origin`, or the starter. The factory
// flag and the read-only rule are the API's (403 factory_module, server docs/API.md "Modules"): this only reads the flag
// and builds the draft the workspace saves.
import { isModuleId, moduleContent, readModuleContent, type CompileOutcome } from '@scutoons/rig-designer/store';
import type { ModuleFiles, StoredModule } from '@scutoons/rig-designer';

export interface ModuleCatalog {
	/** every module id, sorted (a removed one is gone) */
	ids: string[];
	/** the factory modules (read-only) and what they hold – what New module copies */
	factory: Record<string, StoredModule>;
}

export const NO_MODULES: ModuleCatalog = { ids: [], factory: {} };

/** what a new module copies: a module's id and content */
export interface Template {
	id: string;
	module: StoredModule;
}

/** GET /catalog/docs/modules' entries as the workspace's list */
export function readCatalog(entries: { id: string; removed: boolean; content: unknown }[]): ModuleCatalog {
	const live = entries.filter((e) => !e.removed && isModuleId(e.id));
	const factory: Record<string, StoredModule> = {};
	for (const e of live) {
		const module = readModuleContent(e.content, e.id);
		if (module?.factory) factory[e.id] = module;
	}
	return { ids: live.map((e) => e.id).sort(), factory };
}

/** New module's "Start from" lines: the factory modules by id */
export function factoryChoices(catalog: ModuleCatalog): { id: string; label: string }[] {
	return Object.keys(catalog.factory)
		.sort()
		.map((id) => ({ id, label: `module:${id}` }));
}

/** the open module as a template (Derive): its files, and its compile when it is the files' */
export function templateOf(id: string, name: string, files: ModuleFiles, outcome: CompileOutcome | null): Template {
	return { id, module: { name, files, ...(outcome ? { descriptor: outcome.descriptor, wasm: outcome.wasm } : {}) } };
}

/** a new module's first draft: a copy of `from` (its compile kept – the same sources) naming it as its origin, or the starter */
export function firstDraft(id: string, from: Template | null, starter: ModuleFiles): Record<string, unknown> {
	const src = from?.module;
	const outcome = src?.descriptor && src.wasm ? { files: src.files, descriptor: src.descriptor, wasm: src.wasm } : null;
	return moduleContent(id, id, src?.files ?? starter, outcome, from?.id ?? null);
}

/** why `id` cannot be a new module's: 'invalid', 'taken', or '' (it can) */
export function idProblem(id: string, ids: string[]): 'invalid' | 'taken' | '' {
	if (!isModuleId(id)) return 'invalid';
	return ids.includes(id) ? 'taken' : '';
}
