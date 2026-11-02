// The module's document (`module:<id>`, server ModuleSource): the rules of its id and the content the designer writes and
// reads back – pure, so the app's store is a thin wrapper around its API.
import { readWasm } from './api.ts';
import type { Descriptor, ModuleFiles, StoredModule, WasmInfo } from './types.ts';

/** a module document's id (server ModuleSource::ID_PATTERN) */
export const MODULE_ID = /^[a-z0-9][a-z0-9_-]{0,31}$/;

export const isModuleId = (id: string): boolean => MODULE_ID.test(id);

/** `module:walk` → `walk`; anything else → null */
export function moduleIdOf(key: string): string | null {
	if (!key.startsWith('module:')) return null;
	const id = key.slice('module:'.length);
	return isModuleId(id) ? id : null;
}

export function sameFiles(a: ModuleFiles, b: ModuleFiles): boolean {
	const ka = Object.keys(a);
	return ka.length === Object.keys(b).length && ka.every((k) => k in b && a[k] === b[k]);
}

/** what the last compile read its result from: the sources and what came out */
export interface CompileOutcome {
	files: ModuleFiles;
	descriptor: Descriptor;
	wasm: WasmInfo;
}

/**
 * The document to store: the sources always; descriptor and wasm only together and only when the sources are the ones the
 * compile read them from (other sources drop the old result, as on the server)
 */
export function moduleContent(id: string, name: string, files: ModuleFiles, result: CompileOutcome | null): Record<string, unknown> {
	const keep = result !== null && sameFiles(result.files, files);
	return {
		id,
		name,
		files,
		...(keep
			? { descriptor: result.descriptor, wasm: { hash: result.wasm.hash, bytes: result.wasm.bytes, ...(result.wasm.toolchain ? { toolchain: result.wasm.toolchain } : {}) } }
			: {})
	};
}

/** a stored document as the designer's module; null = it is no module (no sources) */
export function readModuleContent(content: unknown, fallbackName: string): StoredModule | null {
	if (typeof content !== 'object' || content === null) return null;
	const c = content as Record<string, unknown>;
	if (typeof c.files !== 'object' || c.files === null) return null;
	const files: ModuleFiles = {};
	for (const [k, v] of Object.entries(c.files)) if (typeof v === 'string') files[k] = v;
	if (Object.keys(files).length === 0) return null;
	const wasm = readWasm(c.wasm);
	const descriptor = typeof c.descriptor === 'object' && c.descriptor !== null ? (c.descriptor as Descriptor) : undefined;
	return { name: typeof c.name === 'string' && c.name !== '' ? c.name : fallbackName, files, ...(descriptor && wasm ? { descriptor, wasm } : {}) };
}
